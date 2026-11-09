//! Pure parsers: `fn parse_x(html: &str) -> Result<X, ParseError>`. No I/O here.
//!
//! Tables are found by **header text** (never by CSS id), and columns are mapped
//! by header name using one rule table per page, so a site change is a one-line fix.

pub mod counters;
pub mod country;
pub mod population;
pub mod tables;

use std::sync::LazyLock;

use scraper::{ElementRef, Html, Node, Selector};

pub const BASE_URL: &str = "https://www.worldometers.info";

#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum ParseError {
    #[error("table with header containing {0:?} not found — page layout may have changed")]
    TableNotFound(&'static str),
    #[error("required column {0:?} missing")]
    MissingColumn(&'static str),
    #[error("table has no rows")]
    Empty,
    #[error("counter script not referenced from the homepage — page layout may have changed")]
    ScriptNotFound,
    #[error("counter constant {0:?} not found — script may have changed")]
    ConstantNotFound(&'static str),
}

fn selector(css: &str) -> Selector {
    Selector::parse(css).expect("constant selector is valid")
}

static TABLE: LazyLock<Selector> = LazyLock::new(|| selector("table"));
static TH: LazyLock<Selector> = LazyLock::new(|| selector("thead th"));
static BODY_ROW: LazyLock<Selector> = LazyLock::new(|| selector("tbody tr"));
static TD: LazyLock<Selector> = LazyLock::new(|| selector("td"));
static LINK: LazyLock<Selector> = LazyLock::new(|| selector("a[href]"));

/// Parses a number as printed on the site.
///
/// Handles thousands separators, `%`, `$`, a leading `+`, and the Unicode minus
/// sign (U+2212) used for negatives. Returns `None` for `"N.A."`, `""` and
/// anything else that is not a number.
pub fn parse_number(s: &str) -> Option<f64> {
    let cleaned: String = s
        .trim()
        .chars()
        .filter_map(|c| match c {
            '\u{2212}' | '\u{2013}' => Some('-'),
            ',' | '%' | '$' | '+' | ' ' | '\u{a0}' => None,
            c => Some(c),
        })
        .collect();
    let value: f64 = cleaned.parse().ok()?;
    value.is_finite().then_some(value)
}

/// Visible text of an element with whitespace collapsed. Footnote markers
/// (`<sup>[1]</sup>`) are skipped.
pub fn cell_text(el: ElementRef) -> String {
    let mut out = String::new();
    collect_text(el, &mut out);
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn collect_text(el: ElementRef, out: &mut String) {
    for child in el.children() {
        match child.value() {
            Node::Text(text) => {
                out.push_str(text);
                out.push(' ');
            }
            Node::Element(e) if e.name() == "sup" => {}
            Node::Element(_) => {
                if let Some(child_el) = ElementRef::wrap(child) {
                    collect_text(child_el, out);
                }
            }
            _ => {}
        }
    }
}

/// Turns a site-relative `href` into an absolute URL. Fragment-only links → `None`.
pub fn absolute_url(href: &str) -> Option<String> {
    if href.starts_with("http://") || href.starts_with("https://") {
        Some(href.to_string())
    } else if href.starts_with('/') {
        Some(format!("{BASE_URL}{href}"))
    } else {
        None
    }
}

/// First link inside a cell, as an absolute URL.
pub fn cell_link(el: ElementRef) -> Option<String> {
    el.select(&LINK)
        .filter_map(|a| a.value().attr("href"))
        .find_map(absolute_url)
}

/// A `<table>` split into lower-cased header texts and body rows of cells.
pub struct RawTable<'a> {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<ElementRef<'a>>>,
}

/// All tables whose headers contain every one of `needles` (case-insensitive),
/// in document order.
pub fn tables_with_headers<'a>(doc: &'a Html, needles: &[&str]) -> Vec<RawTable<'a>> {
    doc.select(&TABLE)
        .filter_map(|table| {
            let headers: Vec<String> = table
                .select(&TH)
                .map(|th| cell_text(th).to_lowercase())
                .collect();
            let matches = needles
                .iter()
                .all(|needle| headers.iter().any(|h| h.contains(&needle.to_lowercase())));
            if !matches {
                return None;
            }
            let rows = table
                .select(&BODY_ROW)
                .map(|tr| tr.select(&TD).collect())
                .collect();
            Some(RawTable { headers, rows })
        })
        .collect()
}

/// The first table that has a header containing `needle`.
pub fn find_table<'a>(doc: &'a Html, needle: &'static str) -> Result<RawTable<'a>, ParseError> {
    tables_with_headers(doc, &[needle])
        .into_iter()
        .next()
        .ok_or(ParseError::TableNotFound(needle))
}

/// How to recognise one column from its header text.
///
/// A header matches when it contains every `include` substring and none of the
/// `exclude` substrings (all lower-case).
pub struct ColumnRule<F> {
    pub field: F,
    pub name: &'static str,
    pub include: &'static [&'static str],
    pub exclude: &'static [&'static str],
    pub required: bool,
}

/// Shorthand constructor for rule tables.
pub const fn col<F>(
    field: F,
    name: &'static str,
    include: &'static [&'static str],
    exclude: &'static [&'static str],
    required: bool,
) -> ColumnRule<F> {
    ColumnRule {
        field,
        name,
        include,
        exclude,
        required,
    }
}

/// Field → column index, built from a table's headers and a rule table.
pub struct ColumnMap<F> {
    columns: Vec<(F, usize)>,
}

impl<F: Copy + PartialEq> ColumnMap<F> {
    /// Maps each rule to the first unclaimed header that matches it. Unknown
    /// extra columns are ignored; a missing required column is an error.
    pub fn new(headers: &[String], rules: &[ColumnRule<F>]) -> Result<Self, ParseError> {
        let mut columns: Vec<(F, usize)> = Vec::new();
        for rule in rules {
            let found = headers.iter().enumerate().find(|(i, header)| {
                !columns.iter().any(|(_, taken)| taken == i)
                    && rule.include.iter().all(|s| header.contains(s))
                    && !rule.exclude.iter().any(|s| header.contains(s))
            });
            match found {
                Some((i, _)) => columns.push((rule.field, i)),
                None if rule.required => return Err(ParseError::MissingColumn(rule.name)),
                None => {}
            }
        }
        Ok(Self { columns })
    }

    pub fn index(&self, field: F) -> Option<usize> {
        self.columns
            .iter()
            .find(|(f, _)| *f == field)
            .map(|(_, i)| *i)
    }

    pub fn cell<'a>(&self, row: &[ElementRef<'a>], field: F) -> Option<ElementRef<'a>> {
        row.get(self.index(field)?).copied()
    }

    pub fn text(&self, row: &[ElementRef], field: F) -> Option<String> {
        self.cell(row, field)
            .map(cell_text)
            .filter(|text| !text.is_empty())
    }

    pub fn num(&self, row: &[ElementRef], field: F) -> Option<f64> {
        self.cell(row, field)
            .and_then(|cell| parse_number(&cell_text(cell)))
    }

    pub fn int(&self, row: &[ElementRef], field: F) -> Option<i64> {
        self.num(row, field).map(|v| v.round() as i64)
    }

    pub fn uint(&self, row: &[ElementRef], field: F) -> Option<u64> {
        self.num(row, field)
            .filter(|v| *v >= 0.0)
            .map(|v| v.round() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_number_handles_site_formats() {
        assert_eq!(parse_number("1,463,865,525"), Some(1_463_865_525.0));
        assert_eq!(parse_number("-0.5 %"), Some(-0.5));
        assert_eq!(parse_number("\u{2212}0.05%"), Some(-0.05));
        assert_eq!(parse_number("\u{2212}440,456"), Some(-440_456.0));
        assert_eq!(parse_number("+0.87%"), Some(0.87));
        assert_eq!(
            parse_number("$32,642,373,000,000"),
            Some(32_642_373_000_000.0)
        );
        assert_eq!(parse_number(" 29.2 "), Some(29.2));
    }

    #[test]
    fn parse_number_rejects_non_numbers() {
        assert_eq!(parse_number("N.A."), None);
        assert_eq!(parse_number(""), None);
        assert_eq!(parse_number("-"), None);
        assert_eq!(parse_number("India"), None);
    }

    #[test]
    fn cell_text_skips_footnotes() {
        let doc = Html::parse_fragment(
            "<table><tr><td>36.0%<sup><a>[<!-- -->1<!-- -->]</a></sup></td></tr></table>",
        );
        let td = doc.select(&TD).next().unwrap();
        assert_eq!(cell_text(td), "36.0%");
    }

    #[test]
    fn absolute_url_variants() {
        assert_eq!(
            absolute_url("/world-population/georgia-population/").as_deref(),
            Some("https://www.worldometers.info/world-population/georgia-population/")
        );
        assert_eq!(absolute_url("#note-1"), None);
        assert_eq!(
            absolute_url("https://x.y/").as_deref(),
            Some("https://x.y/")
        );
    }

    #[test]
    fn column_map_respects_exclusions_and_requirements() {
        #[derive(Clone, Copy, PartialEq, Debug)]
        enum F {
            Pop,
            Urban,
            Missing,
        }
        let headers = vec![
            "urban population".to_string(),
            "population 2026".to_string(),
        ];
        let rules = [
            col(F::Pop, "population", &["population"], &["urban"], true),
            col(F::Urban, "urban", &["urban"], &[], false),
            col(F::Missing, "missing", &["nope"], &[], false),
        ];
        let map = ColumnMap::new(&headers, &rules).unwrap();
        assert_eq!(map.index(F::Pop), Some(1));
        assert_eq!(map.index(F::Urban), Some(0));
        assert_eq!(map.index(F::Missing), None);

        let strict = [col(F::Missing, "missing", &["nope"], &[], true)];
        assert_eq!(
            ColumnMap::new(&headers, &strict).err(),
            Some(ParseError::MissingColumn("missing"))
        );
    }
}
