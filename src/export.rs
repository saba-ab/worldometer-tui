//! `--export csv <table>`: write a table to stdout as CSV (Phase 6).

use std::fmt::Write as _;

use crate::msg::{Msg, PageData, Request};
use crate::net::worker::Loader;
use crate::tables::{Cell, Table, TableKind};

/// RFC 4180 quoting: quote fields containing `,`, `"` or newlines.
fn csv_field(text: &str) -> String {
    if text.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

/// Raw values (no thousands separators, no units) so spreadsheets can use them.
pub fn to_csv(table: &Table) -> String {
    let mut out = String::new();
    let header: Vec<&str> = table.columns.iter().map(|c| c.csv).collect();
    out.push_str(&header.join(","));
    out.push('\n');
    for row in &table.rows {
        let fields: Vec<String> = row
            .cells
            .iter()
            .map(|cell| match cell {
                Cell::Text(text) => csv_field(text),
                Cell::Num(Some(v)) => v.to_string(),
                Cell::Num(None) => String::new(),
            })
            .collect();
        let _ = writeln!(out, "{}", fields.join(","));
    }
    out
}

/// Loads `kind` (cache → network → stale cache) and renders it as CSV.
pub fn export_csv(loader: &Loader, kind: TableKind) -> Result<String, String> {
    let request = Request {
        key: kind.page_key(),
        force: false,
    };
    match loader.load(&request) {
        Msg::Loaded {
            data: PageData::Table(table),
            ..
        } => Ok(to_csv(&table)),
        Msg::Loaded { .. } => Err("unexpected page type".to_string()),
        Msg::Failed { error, .. } => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AreaRow;

    #[test]
    fn csv_has_raw_values_and_quotes() {
        let rows = vec![
            AreaRow {
                rank: 1,
                name: "Russia".into(),
                total_area_km2: 17_098_242,
                land_area_km2: Some(16_377_742),
                world_share_pct: Some(12.4),
            },
            AreaRow {
                rank: 2,
                name: "Korea, \"South\"".into(),
                total_area_km2: 100_210,
                land_area_km2: None,
                world_share_pct: None,
            },
        ];
        let csv = to_csv(&Table::from_rows(&rows));
        assert_eq!(
            csv,
            "rank,country,total_area_km2,land_area_km2,world_share_pct\n\
             1,Russia,17098242,16377742,12.4\n\
             2,\"Korea, \"\"South\"\"\",100210,,\n"
        );
    }
}
