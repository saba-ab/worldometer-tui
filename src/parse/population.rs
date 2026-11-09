//! "Population by country" table → [`CountryRow`]s.

use scraper::Html;

use super::{ColumnMap, ColumnRule, ParseError, cell_link, col, find_table};
use crate::model::CountryRow;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Field {
    Rank,
    Name,
    Population,
    YearlyChange,
    NetChange,
    Density,
    LandArea,
    Migrants,
    Fertility,
    MedianAge,
    Urban,
    Share,
}

/// Header-matching rules. Edit here when the site renames a column.
const COLUMNS: &[ColumnRule<Field>] = &[
    col(Field::Rank, "#", &["#"], &[], false),
    col(Field::Name, "country", &["country"], &[], true),
    col(
        Field::Population,
        "population",
        &["population"],
        &["urban"],
        true,
    ),
    col(
        Field::YearlyChange,
        "yearly change",
        &["yearly change"],
        &[],
        false,
    ),
    col(Field::NetChange, "net change", &["net change"], &[], false),
    col(Field::Density, "density", &["density"], &[], false),
    col(Field::LandArea, "land area", &["land area"], &[], false),
    col(Field::Migrants, "migrants", &["migrants"], &[], false),
    col(Field::Fertility, "fertility", &["fert"], &[], false),
    col(Field::MedianAge, "median age", &["median age"], &[], false),
    col(Field::Urban, "urban pop", &["urban"], &[], false),
    col(Field::Share, "world share", &["world share"], &[], false),
];

/// The header text used to locate the table.
const TABLE_MARKER: &str = "yearly change";

pub fn parse_population(html: &str) -> Result<Vec<CountryRow>, ParseError> {
    let doc = Html::parse_document(html);
    let table = find_table(&doc, TABLE_MARKER)?;
    let map = ColumnMap::new(&table.headers, COLUMNS)?;

    let rows: Vec<CountryRow> = table
        .rows
        .iter()
        .enumerate()
        .filter_map(|(i, row)| {
            let name = map.text(row, Field::Name)?;
            let population = map.uint(row, Field::Population)?;
            Some(CountryRow {
                rank: map
                    .uint(row, Field::Rank)
                    .and_then(|r| u32::try_from(r).ok())
                    .unwrap_or(i as u32 + 1),
                detail_url: map.cell(row, Field::Name).and_then(cell_link),
                name,
                population,
                yearly_change_pct: map.num(row, Field::YearlyChange),
                net_change: map.int(row, Field::NetChange),
                density_per_km2: map.num(row, Field::Density),
                land_area_km2: map.uint(row, Field::LandArea),
                migrants_net: map.int(row, Field::Migrants),
                fertility_rate: map.num(row, Field::Fertility),
                median_age: map.num(row, Field::MedianAge),
                urban_pop_pct: map.num(row, Field::Urban),
                world_share_pct: map.num(row, Field::Share),
            })
        })
        .collect();

    if rows.is_empty() {
        return Err(ParseError::Empty);
    }
    Ok(rows)
}
