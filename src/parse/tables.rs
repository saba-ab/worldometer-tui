//! Phase 5 tables: GDP, CO2 emissions and area by country.

use scraper::Html;

use super::{ColumnMap, ColumnRule, ParseError, col, find_table};
use crate::model::{AreaRow, Co2Row, GdpRow};

fn rank_or_index(rank: Option<u64>, index: usize) -> u32 {
    rank.and_then(|r| u32::try_from(r).ok())
        .unwrap_or(index as u32 + 1)
}

fn non_empty<T>(rows: Vec<T>) -> Result<Vec<T>, ParseError> {
    if rows.is_empty() {
        Err(ParseError::Empty)
    } else {
        Ok(rows)
    }
}

// ---------------------------------------------------------------- GDP

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Gdp {
    Rank,
    Name,
    Value,
    Growth,
    PerCapita,
    Share,
}

const GDP_COLUMNS: &[ColumnRule<Gdp>] = &[
    col(Gdp::Rank, "#", &["#"], &[], false),
    col(Gdp::Name, "country", &["country"], &[], true),
    // "GDP (US$)" is abbreviated ("$32.64 trillion"); the full value is exact.
    col(
        Gdp::Value,
        "gdp (full value)",
        &["gdp", "full value"],
        &[],
        true,
    ),
    col(Gdp::Growth, "gdp growth", &["growth"], &[], false),
    col(Gdp::PerCapita, "per capita", &["per capita"], &[], false),
    col(Gdp::Share, "share of world", &["share"], &[], false),
];

pub fn parse_gdp(html: &str) -> Result<Vec<GdpRow>, ParseError> {
    let doc = Html::parse_document(html);
    let table = find_table(&doc, "full value")?;
    let map = ColumnMap::new(&table.headers, GDP_COLUMNS)?;
    let rows = table
        .rows
        .iter()
        .enumerate()
        .filter_map(|(i, row)| {
            Some(GdpRow {
                rank: rank_or_index(map.uint(row, Gdp::Rank), i),
                name: map.text(row, Gdp::Name)?,
                gdp_usd: map.num(row, Gdp::Value)?,
                growth_pct: map.num(row, Gdp::Growth),
                per_capita_usd: map.num(row, Gdp::PerCapita),
                world_share_pct: map.num(row, Gdp::Share),
            })
        })
        .collect();
    non_empty(rows)
}

// ---------------------------------------------------------------- CO2

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Co2 {
    Rank,
    Name,
    Emissions,
    Change,
    PerCapita,
    Share,
}

const CO2_COLUMNS: &[ColumnRule<Co2>] = &[
    col(Co2::Rank, "#", &["#"], &[], false),
    col(Co2::Name, "country", &["country"], &[], true),
    col(
        Co2::Emissions,
        "co2 emissions",
        &["co2 emissions"],
        &[],
        true,
    ),
    col(Co2::Change, "1 year change", &["change"], &[], false),
    col(Co2::PerCapita, "per capita", &["per capita"], &[], false),
    col(Co2::Share, "share of world", &["share"], &[], false),
];

pub fn parse_co2(html: &str) -> Result<Vec<Co2Row>, ParseError> {
    let doc = Html::parse_document(html);
    let table = find_table(&doc, "co2 emissions")?;
    let map = ColumnMap::new(&table.headers, CO2_COLUMNS)?;
    let rows = table
        .rows
        .iter()
        .enumerate()
        .filter_map(|(i, row)| {
            Some(Co2Row {
                rank: rank_or_index(map.uint(row, Co2::Rank), i),
                name: map.text(row, Co2::Name)?,
                emissions_tons: map.uint(row, Co2::Emissions)?,
                yearly_change_pct: map.num(row, Co2::Change),
                per_capita_tons: map.num(row, Co2::PerCapita),
                world_share_pct: map.num(row, Co2::Share),
            })
        })
        .collect();
    non_empty(rows)
}

// ---------------------------------------------------------------- Area

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Area {
    Rank,
    Name,
    Total,
    Land,
    Share,
}

const AREA_COLUMNS: &[ColumnRule<Area>] = &[
    col(Area::Rank, "#", &["#"], &[], false),
    col(Area::Name, "country", &["country"], &[], true),
    col(
        Area::Total,
        "tot. area (km²)",
        &["tot. area", "km"],
        &[],
        true,
    ),
    col(
        Area::Land,
        "land area (km²)",
        &["land area", "km"],
        &[],
        false,
    ),
    col(Area::Share, "world share", &["share"], &[], false),
];

pub fn parse_area(html: &str) -> Result<Vec<AreaRow>, ParseError> {
    let doc = Html::parse_document(html);
    let table = find_table(&doc, "tot. area")?;
    let map = ColumnMap::new(&table.headers, AREA_COLUMNS)?;
    let rows = table
        .rows
        .iter()
        .enumerate()
        .filter_map(|(i, row)| {
            Some(AreaRow {
                rank: rank_or_index(map.uint(row, Area::Rank), i),
                name: map.text(row, Area::Name)?,
                total_area_km2: map.uint(row, Area::Total)?,
                land_area_km2: map.uint(row, Area::Land),
                world_share_pct: map.num(row, Area::Share),
            })
        })
        .collect();
    non_empty(rows)
}
