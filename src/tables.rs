//! Table tabs: typed rows → display cells, plus pure sort/search/selection logic.
//!
//! Each typed row (`CountryRow`, `GdpRow`, …) describes its columns once via
//! [`TableRow`]; the UI, sorting and CSV export work on the resulting [`Table`].

use std::cmp::Ordering;

use crate::model::{AreaRow, Co2Row, CountryRow, GdpRow};
use crate::msg::PageKey;

/// How a column is formatted and aligned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColKind {
    Text,
    Int,
    Float(u8),
    /// `+0.89 %` / `−0.23 %`, coloured by sign.
    SignedPct,
    Pct,
    Usd,
    UsdCompact,
}

impl ColKind {
    pub fn is_numeric(self) -> bool {
        self != ColKind::Text
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnDef {
    pub title: &'static str,
    /// Column name in CSV export.
    pub csv: &'static str,
    pub width: u16,
    pub kind: ColKind,
}

const fn column(title: &'static str, csv: &'static str, width: u16, kind: ColKind) -> ColumnDef {
    ColumnDef {
        title,
        csv,
        width,
        kind,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Text(String),
    Num(Option<f64>),
}

impl Cell {
    pub fn text(&self) -> Option<&str> {
        match self {
            Cell::Text(t) => Some(t),
            Cell::Num(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub cells: Vec<Cell>,
    pub detail_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub columns: &'static [ColumnDef],
    pub rows: Vec<Row>,
}

/// Every table has the rank in column 0 and the country name in column 1.
pub const RANK_COL: usize = 0;
pub const NAME_COL: usize = 1;

pub trait TableRow {
    const COLUMNS: &'static [ColumnDef];
    fn cells(&self) -> Vec<Cell>;
    fn detail_url(&self) -> Option<String> {
        None
    }
}

impl Table {
    pub fn from_rows<R: TableRow>(rows: &[R]) -> Self {
        Table {
            columns: R::COLUMNS,
            rows: rows
                .iter()
                .map(|r| Row {
                    cells: r.cells(),
                    detail_url: r.detail_url(),
                })
                .collect(),
        }
    }
}

fn int<T: Into<f64>>(v: T) -> Cell {
    Cell::Num(Some(v.into()))
}

fn opt<T: Into<f64>>(v: Option<T>) -> Cell {
    Cell::Num(v.map(Into::into))
}

fn big(v: u64) -> Cell {
    Cell::Num(Some(v as f64))
}

fn big_opt(v: Option<u64>) -> Cell {
    Cell::Num(v.map(|v| v as f64))
}

fn signed_opt(v: Option<i64>) -> Cell {
    Cell::Num(v.map(|v| v as f64))
}

impl TableRow for CountryRow {
    const COLUMNS: &'static [ColumnDef] = &[
        column("#", "rank", 4, ColKind::Int),
        column("Country", "country", 22, ColKind::Text),
        column("Population", "population", 14, ColKind::Int),
        column("Yearly %", "yearly_change_pct", 10, ColKind::SignedPct),
        column("Net change", "net_change", 12, ColKind::Int),
        column("Density", "density_per_km2", 9, ColKind::Int),
        column("Med. age", "median_age", 10, ColKind::Float(1)),
        column("Fert.", "fertility_rate", 7, ColKind::Float(2)),
        column("World %", "world_share_pct", 9, ColKind::Pct),
        column("Urban %", "urban_pop_pct", 9, ColKind::Pct),
        column("Migrants", "migrants_net", 10, ColKind::Int),
        column("Land km²", "land_area_km2", 12, ColKind::Int),
    ];

    fn cells(&self) -> Vec<Cell> {
        vec![
            int(self.rank),
            Cell::Text(self.name.clone()),
            big(self.population),
            opt(self.yearly_change_pct),
            signed_opt(self.net_change),
            opt(self.density_per_km2),
            opt(self.median_age),
            opt(self.fertility_rate),
            opt(self.world_share_pct),
            opt(self.urban_pop_pct),
            signed_opt(self.migrants_net),
            big_opt(self.land_area_km2),
        ]
    }

    fn detail_url(&self) -> Option<String> {
        self.detail_url.clone()
    }
}

impl TableRow for GdpRow {
    const COLUMNS: &'static [ColumnDef] = &[
        column("#", "rank", 4, ColKind::Int),
        column("Country", "country", 26, ColKind::Text),
        column("GDP (US$)", "gdp_usd", 12, ColKind::UsdCompact),
        column("Growth", "growth_pct", 9, ColKind::SignedPct),
        column("Per capita", "per_capita_usd", 11, ColKind::Usd),
        column("World %", "world_share_pct", 8, ColKind::Pct),
    ];

    fn cells(&self) -> Vec<Cell> {
        vec![
            int(self.rank),
            Cell::Text(self.name.clone()),
            Cell::Num(Some(self.gdp_usd)),
            opt(self.growth_pct),
            opt(self.per_capita_usd),
            opt(self.world_share_pct),
        ]
    }
}

impl TableRow for Co2Row {
    const COLUMNS: &'static [ColumnDef] = &[
        column("#", "rank", 4, ColKind::Int),
        column("Country", "country", 26, ColKind::Text),
        column("CO₂ (tons)", "emissions_tons", 15, ColKind::Int),
        column("1y change", "yearly_change_pct", 10, ColKind::SignedPct),
        column("Per capita", "per_capita_tons", 10, ColKind::Float(2)),
        column("World %", "world_share_pct", 8, ColKind::Pct),
    ];

    fn cells(&self) -> Vec<Cell> {
        vec![
            int(self.rank),
            Cell::Text(self.name.clone()),
            big(self.emissions_tons),
            opt(self.yearly_change_pct),
            opt(self.per_capita_tons),
            opt(self.world_share_pct),
        ]
    }
}

impl TableRow for AreaRow {
    const COLUMNS: &'static [ColumnDef] = &[
        column("#", "rank", 4, ColKind::Int),
        column("Country", "country", 26, ColKind::Text),
        column("Total km²", "total_area_km2", 13, ColKind::Int),
        column("Land km²", "land_area_km2", 13, ColKind::Int),
        column("World %", "world_share_pct", 8, ColKind::Pct),
    ];

    fn cells(&self) -> Vec<Cell> {
        vec![
            int(self.rank),
            Cell::Text(self.name.clone()),
            big(self.total_area_km2),
            big_opt(self.land_area_km2),
            opt(self.world_share_pct),
        ]
    }
}

/// The four table tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableKind {
    Population,
    Gdp,
    Co2,
    Area,
}

impl TableKind {
    pub const ALL: [TableKind; 4] = [
        TableKind::Population,
        TableKind::Gdp,
        TableKind::Co2,
        TableKind::Area,
    ];

    pub fn title(self) -> &'static str {
        match self {
            TableKind::Population => "Countries",
            TableKind::Gdp => "GDP",
            TableKind::Co2 => "CO₂",
            TableKind::Area => "Area",
        }
    }

    /// Name used by `--export csv <table>`.
    pub fn cli_name(self) -> &'static str {
        match self {
            TableKind::Population => "countries",
            TableKind::Gdp => "gdp",
            TableKind::Co2 => "co2",
            TableKind::Area => "area",
        }
    }

    pub fn from_cli_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|k| k.cli_name().eq_ignore_ascii_case(name))
    }

    pub fn page_key(self) -> PageKey {
        match self {
            TableKind::Population => PageKey::Population,
            TableKind::Gdp => PageKey::Gdp,
            TableKind::Co2 => PageKey::Co2,
            TableKind::Area => PageKey::Area,
        }
    }
}

// ------------------------------------------------------------- sort/search

/// Compares two cells for sorting. Text is case-insensitive.
/// `None` numbers compare as greater than any value.
pub fn compare_cells(a: &Cell, b: &Cell) -> Ordering {
    match (a, b) {
        (Cell::Text(a), Cell::Text(b)) => a.to_lowercase().cmp(&b.to_lowercase()),
        (Cell::Num(Some(a)), Cell::Num(Some(b))) => a.total_cmp(b),
        (Cell::Num(None), Cell::Num(None)) => Ordering::Equal,
        (Cell::Num(None), _) => Ordering::Greater,
        (_, Cell::Num(None)) => Ordering::Less,
        (Cell::Text(_), Cell::Num(_)) => Ordering::Less,
        (Cell::Num(_), Cell::Text(_)) => Ordering::Greater,
    }
}

fn is_missing(cell: Option<&Cell>) -> bool {
    matches!(cell, Some(Cell::Num(None)) | None)
}

/// Row indices matching `query` (case-insensitive substring of the name),
/// sorted by `sort_col`. Missing values always sort last. Ties keep rank order.
pub fn sorted_filtered(
    table: &Table,
    sort_col: usize,
    descending: bool,
    query: &str,
) -> Vec<usize> {
    let query = query.trim().to_lowercase();
    let mut order: Vec<usize> = table
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| {
            query.is_empty()
                || row
                    .cells
                    .get(NAME_COL)
                    .and_then(Cell::text)
                    .is_some_and(|name| name.to_lowercase().contains(&query))
        })
        .map(|(i, _)| i)
        .collect();
    order.sort_by(|&a, &b| {
        let ca = table.rows[a].cells.get(sort_col);
        let cb = table.rows[b].cells.get(sort_col);
        let missing = is_missing(ca).cmp(&is_missing(cb));
        if missing != Ordering::Equal {
            return missing;
        }
        let ord = match (ca, cb) {
            (Some(x), Some(y)) => compare_cells(x, y),
            _ => Ordering::Equal,
        };
        if descending { ord.reverse() } else { ord }
    });
    order
}

/// Sort/search/selection state for one table tab.
#[derive(Debug, Clone, PartialEq)]
pub struct TableView {
    pub sort_col: usize,
    pub descending: bool,
    pub query: String,
    /// Index into `order`.
    pub selected: usize,
    /// First visible row (scroll position), maintained by the UI.
    pub offset: usize,
    /// Visible row indices in display order.
    pub order: Vec<usize>,
}

impl Default for TableView {
    fn default() -> Self {
        Self {
            sort_col: RANK_COL,
            descending: false,
            query: String::new(),
            selected: 0,
            offset: 0,
            order: Vec::new(),
        }
    }
}

impl TableView {
    /// Recomputes `order`, keeping the same underlying row selected if visible.
    pub fn rebuild(&mut self, table: &Table) {
        let previous = self.order.get(self.selected).copied();
        self.order = sorted_filtered(table, self.sort_col, self.descending, &self.query);
        self.selected = previous
            .and_then(|row| self.order.iter().position(|&r| r == row))
            .unwrap_or(0);
    }

    pub fn selected_row<'t>(&self, table: &'t Table) -> Option<&'t Row> {
        table.rows.get(*self.order.get(self.selected)?)
    }

    pub fn move_by(&mut self, delta: isize) {
        if self.order.is_empty() {
            self.selected = 0;
            return;
        }
        let last = self.order.len() as isize - 1;
        self.selected = (self.selected as isize + delta).clamp(0, last) as usize;
    }

    pub fn first(&mut self) {
        self.selected = 0;
    }

    pub fn last(&mut self) {
        self.selected = self.order.len().saturating_sub(1);
    }

    /// Moves the sort column by `delta`, wrapping. Text and rank columns sort
    /// ascending first; numeric columns descending (largest first).
    pub fn shift_sort(&mut self, delta: isize, table: &Table) {
        let n = table.columns.len() as isize;
        if n == 0 {
            return;
        }
        let col = (self.sort_col as isize + delta).rem_euclid(n) as usize;
        self.sort_col = col;
        self.descending = col != RANK_COL && table.columns[col].kind.is_numeric();
        self.rebuild(table);
    }

    pub fn toggle_direction(&mut self, table: &Table) {
        self.descending = !self.descending;
        self.rebuild(table);
    }

    pub fn set_query(&mut self, query: String, table: &Table) {
        self.query = query;
        self.rebuild(table);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> Table {
        const COLS: &[ColumnDef] = &[
            column("#", "rank", 4, ColKind::Int),
            column("Name", "name", 10, ColKind::Text),
            column("Val", "val", 6, ColKind::Float(1)),
        ];
        let row = |rank: f64, name: &str, val: Option<f64>| Row {
            cells: vec![
                Cell::Num(Some(rank)),
                Cell::Text(name.into()),
                Cell::Num(val),
            ],
            detail_url: None,
        };
        Table {
            columns: COLS,
            rows: vec![
                row(1.0, "india", Some(5.0)),
                row(2.0, "China", None),
                row(3.0, "brazil", Some(9.0)),
                row(4.0, "Chad", Some(1.0)),
            ],
        }
    }

    #[test]
    fn sort_by_name_is_case_insensitive() {
        let t = table();
        assert_eq!(sorted_filtered(&t, NAME_COL, false, ""), vec![2, 3, 1, 0]);
        assert_eq!(sorted_filtered(&t, NAME_COL, true, ""), vec![0, 1, 3, 2]);
    }

    #[test]
    fn missing_values_sort_last_both_ways() {
        let t = table();
        assert_eq!(sorted_filtered(&t, 2, false, ""), vec![3, 0, 2, 1]);
        assert_eq!(sorted_filtered(&t, 2, true, ""), vec![2, 0, 3, 1]);
    }

    #[test]
    fn search_filters_by_name_substring() {
        let t = table();
        assert_eq!(sorted_filtered(&t, RANK_COL, false, "CH"), vec![1, 3]);
        assert!(sorted_filtered(&t, RANK_COL, false, "zz").is_empty());
    }

    #[test]
    fn rebuild_keeps_selected_row() {
        let t = table();
        let mut view = TableView::default();
        view.rebuild(&t);
        view.move_by(2); // brazil (row 2)
        view.shift_sort(1, &t); // sort by name: brazil first
        assert_eq!(
            view.selected_row(&t).unwrap().cells[1],
            Cell::Text("brazil".into())
        );
        assert_eq!(view.selected, 0);
    }

    #[test]
    fn selection_is_clamped() {
        let t = table();
        let mut view = TableView::default();
        view.rebuild(&t);
        view.move_by(-5);
        assert_eq!(view.selected, 0);
        view.move_by(50);
        assert_eq!(view.selected, 3);
        view.set_query("india".into(), &t);
        assert_eq!(view.order, vec![0]);
        assert_eq!(view.selected, 0);
    }

    #[test]
    fn numeric_columns_default_to_descending() {
        let t = table();
        let mut view = TableView::default();
        view.rebuild(&t);
        view.shift_sort(2, &t);
        assert_eq!(view.sort_col, 2);
        assert!(view.descending);
        view.shift_sort(1, &t); // wraps to rank
        assert_eq!(view.sort_col, RANK_COL);
        assert!(!view.descending);
    }
}
