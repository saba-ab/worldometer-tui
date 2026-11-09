//! Typed data parsed from worldometers pages.

/// One row of the "population by country" table.
#[derive(Debug, Clone, PartialEq)]
pub struct CountryRow {
    pub rank: u32,
    pub name: String,
    /// Absolute URL of the country page, taken from the country cell's link.
    pub detail_url: Option<String>,
    pub population: u64,
    pub yearly_change_pct: Option<f64>,
    pub net_change: Option<i64>,
    pub density_per_km2: Option<f64>,
    pub land_area_km2: Option<u64>,
    pub migrants_net: Option<i64>,
    pub fertility_rate: Option<f64>,
    pub median_age: Option<f64>,
    pub urban_pop_pct: Option<f64>,
    pub world_share_pct: Option<f64>,
}

/// One year of a country's population history (or forecast).
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryPoint {
    pub year: i32,
    pub population: u64,
    pub yearly_change_pct: Option<f64>,
    pub yearly_change: Option<i64>,
    pub migrants_net: Option<i64>,
    pub median_age: Option<f64>,
    pub fertility_rate: Option<f64>,
    pub density: Option<f64>,
    pub urban_pop_pct: Option<f64>,
    pub world_share_pct: Option<f64>,
    pub world_population: Option<u64>,
    pub rank: Option<u32>,
}

/// A country page: headline numbers plus history (oldest first) and forecast.
#[derive(Debug, Clone, PartialEq)]
pub struct CountryDetail {
    pub name: String,
    pub population: u64,
    pub rank: Option<u32>,
    pub world_share_pct: Option<f64>,
    /// Sorted by year, ascending.
    pub history: Vec<HistoryPoint>,
    /// Sorted by year, ascending. May be empty.
    pub forecast: Vec<HistoryPoint>,
}

/// One row of the "GDP by country" table.
#[derive(Debug, Clone, PartialEq)]
pub struct GdpRow {
    pub rank: u32,
    pub name: String,
    pub gdp_usd: f64,
    pub growth_pct: Option<f64>,
    pub per_capita_usd: Option<f64>,
    pub world_share_pct: Option<f64>,
}

/// One row of the "CO2 emissions by country" table.
#[derive(Debug, Clone, PartialEq)]
pub struct Co2Row {
    pub rank: u32,
    pub name: String,
    pub emissions_tons: u64,
    pub yearly_change_pct: Option<f64>,
    pub per_capita_tons: Option<f64>,
    pub world_share_pct: Option<f64>,
}

/// One row of the "largest countries by area" table.
#[derive(Debug, Clone, PartialEq)]
pub struct AreaRow {
    pub rank: u32,
    pub name: String,
    pub total_area_km2: u64,
    pub land_area_km2: Option<u64>,
    pub world_share_pct: Option<f64>,
}

/// Constants read from worldometers' readable energy-counter script
/// (see `docs/DISCOVERY.md`, "v2 energy counters").
#[derive(Debug, Clone, PartialEq)]
pub struct EnergyConstants {
    pub oil: Reserve,
    pub gas: Reserve,
    pub coal: Reserve,
    /// Total primary energy used per year, in exajoules.
    pub energy_total_ej_per_year: f64,
    /// Share of energy from low-carbon sources, 0..=1.
    pub low_carbon_share: f64,
}

/// Known reserves of a fossil fuel and its annual production.
#[derive(Debug, Clone, PartialEq)]
pub struct Reserve {
    pub reserves: f64,
    /// Reserves are as of December 31 of this year.
    pub reserves_year: i32,
    pub production_per_year: f64,
}
