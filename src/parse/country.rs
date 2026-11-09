//! Country page (e.g. `/world-population/georgia-population/`) → [`CountryDetail`].

use std::sync::LazyLock;

use scraper::{ElementRef, Html, Selector};

use super::{ColumnMap, ColumnRule, ParseError, cell_text, col, tables_with_headers};
use crate::model::{CountryDetail, HistoryPoint};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Field {
    Year,
    Population,
    YearlyPct,
    YearlyChange,
    Migrants,
    MedianAge,
    Fertility,
    Density,
    UrbanPct,
    Share,
    WorldPopulation,
    Rank,
}

/// Header-matching rules shared by the history and forecast tables.
const COLUMNS: &[ColumnRule<Field>] = &[
    col(Field::Year, "year", &["year"], &["yearly"], true),
    col(
        Field::Population,
        "population",
        &["population"],
        &["urban", "world"],
        true,
    ),
    col(
        Field::YearlyPct,
        "yearly % change",
        &["yearly %"],
        &[],
        false,
    ),
    col(
        Field::YearlyChange,
        "yearly change",
        &["yearly change"],
        &[],
        false,
    ),
    col(Field::Migrants, "migrants", &["migrants"], &[], false),
    col(Field::MedianAge, "median age", &["median age"], &[], false),
    col(Field::Fertility, "fertility", &["fertility"], &[], false),
    col(Field::Density, "density", &["density"], &[], false),
    col(Field::UrbanPct, "urban pop %", &["urban pop %"], &[], false),
    col(Field::Share, "share of world", &["share"], &[], false),
    col(
        Field::WorldPopulation,
        "world population",
        &["world population"],
        &[],
        false,
    ),
    col(Field::Rank, "global rank", &["rank"], &[], false),
];

const TABLE_MARKER: &str = "yearly % change";

static H1: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("h1").expect("constant selector is valid"));

pub fn parse_country(html: &str) -> Result<CountryDetail, ParseError> {
    let doc = Html::parse_document(html);
    let mut tables = tables_with_headers(&doc, &[TABLE_MARKER, "year"]).into_iter();

    let history_table = tables
        .next()
        .ok_or(ParseError::TableNotFound(TABLE_MARKER))?;
    let map = ColumnMap::new(&history_table.headers, COLUMNS)?;
    let mut history = parse_points(&map, &history_table.rows);
    if history.is_empty() {
        return Err(ParseError::Empty);
    }
    history.sort_by_key(|p| p.year);

    let mut forecast = match tables.next() {
        Some(table) => match ColumnMap::new(&table.headers, COLUMNS) {
            Ok(map) => parse_points(&map, &table.rows),
            Err(_) => Vec::new(),
        },
        None => Vec::new(),
    };
    forecast.sort_by_key(|p| p.year);

    let name = page_name(&doc)
        .or_else(|| name_from_rank_header(&history_table.headers))
        .unwrap_or_else(|| "Unknown".to_string());
    let latest = history.last().ok_or(ParseError::Empty)?;

    Ok(CountryDetail {
        name,
        population: latest.population,
        rank: latest.rank,
        world_share_pct: latest.world_share_pct,
        history,
        forecast,
    })
}

fn parse_points(map: &ColumnMap<Field>, rows: &[Vec<ElementRef>]) -> Vec<HistoryPoint> {
    rows.iter()
        .filter_map(|row| {
            Some(HistoryPoint {
                year: i32::try_from(map.int(row, Field::Year)?).ok()?,
                population: map.uint(row, Field::Population)?,
                yearly_change_pct: map.num(row, Field::YearlyPct),
                yearly_change: map.int(row, Field::YearlyChange),
                migrants_net: map.int(row, Field::Migrants),
                median_age: map.num(row, Field::MedianAge),
                fertility_rate: map.num(row, Field::Fertility),
                density: map.num(row, Field::Density),
                urban_pop_pct: map.num(row, Field::UrbanPct),
                world_share_pct: map.num(row, Field::Share),
                world_population: map.uint(row, Field::WorldPopulation),
                rank: map
                    .uint(row, Field::Rank)
                    .and_then(|r| u32::try_from(r).ok()),
            })
        })
        .collect()
}

/// `<h1>Georgia Population <small>(LIVE)</small></h1>` → `"Georgia"`.
fn page_name(doc: &Html) -> Option<String> {
    doc.select(&H1).find_map(|h1| {
        let text = cell_text(h1).replace("(LIVE)", "");
        let name = text.trim().strip_suffix("Population")?.trim();
        (!name.is_empty()).then(|| name.to_string())
    })
}

/// Fallback: the `"<Country> Global Rank"` header (lower-cased) → `"<country>"`.
fn name_from_rank_header(headers: &[String]) -> Option<String> {
    headers.iter().find_map(|h| {
        let name = h.strip_suffix("global rank")?.trim();
        (!name.is_empty()).then(|| name.to_string())
    })
}
