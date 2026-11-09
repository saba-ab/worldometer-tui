//! Live counters (docs/plan.md §6): `value(t) = base + rate_per_sec × (t − epoch)`.
//!
//! Values are computed from the clock only — no I/O — so the dashboard can tick
//! every frame and tests can freeze time.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, TimeZone, Utc};
use serde::Deserialize;

use crate::model::{EnergyConstants, Reserve};

/// Seconds per Julian year, used for Option B estimates.
pub const SECONDS_PER_YEAR: f64 = 31_557_600.0;
/// Seconds per year used by worldometers' own energy-counter script (365 days).
pub const SITE_SECONDS_PER_YEAR: f64 = 31_536_000.0;

/// The bundled Option B counter definitions.
pub const DEFAULT_COUNTERS_TOML: &str = include_str!("../assets/counters.toml");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Population,
    Health,
    Environment,
    Energy,
    Economy,
    Society,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Category::Population => "Population",
            Category::Health => "Health",
            Category::Environment => "Environment",
            Category::Energy => "Energy",
            Category::Economy => "Economy",
            Category::Society => "Society",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Reset {
    Never,
    /// Counts from local midnight.
    Daily,
    /// Counts from local January 1.
    Yearly,
}

/// Where a counter's base and rate come from.
#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    /// Option A: constants read from worldometers' own script.
    Worldometers,
    /// Option B: local estimate from public figures; never presented as site data.
    Estimated(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Counter {
    pub key: String,
    pub label: String,
    pub unit: Option<String>,
    pub category: Category,
    /// Value at `epoch` (for `Reset::Never`) or at the start of the period.
    pub base: f64,
    /// May be negative.
    pub rate_per_sec: f64,
    pub epoch: SystemTime,
    pub reset: Reset,
    pub source: Source,
}

impl Counter {
    /// The counter's value at `t`. Daily/yearly resets use the `tz` calendar.
    pub fn value_at(&self, t: SystemTime, tz: FixedOffset) -> f64 {
        let start = match self.reset {
            Reset::Never => self.epoch,
            Reset::Daily => start_of_day(t, tz),
            Reset::Yearly => start_of_year(t, tz),
        };
        self.base + self.rate_per_sec * seconds_between(start, t)
    }
}

/// `end − start` in seconds; negative when `end` is earlier.
pub fn seconds_between(start: SystemTime, end: SystemTime) -> f64 {
    match end.duration_since(start) {
        Ok(d) => d.as_secs_f64(),
        Err(e) => -e.duration().as_secs_f64(),
    }
}

fn local_date(t: SystemTime, tz: FixedOffset) -> NaiveDate {
    DateTime::<Utc>::from(t).with_timezone(&tz).date_naive()
}

fn start_of_date(date: Option<NaiveDate>, tz: FixedOffset, fallback: SystemTime) -> SystemTime {
    date.and_then(|d| d.and_hms_opt(0, 0, 0))
        .and_then(|midnight| tz.from_local_datetime(&midnight).single())
        .map(SystemTime::from)
        .unwrap_or(fallback)
}

pub fn start_of_day(t: SystemTime, tz: FixedOffset) -> SystemTime {
    start_of_date(Some(local_date(t, tz)), tz, t)
}

pub fn start_of_year(t: SystemTime, tz: FixedOffset) -> SystemTime {
    let year = local_date(t, tz).year();
    start_of_date(NaiveDate::from_ymd_opt(year, 1, 1), tz, t)
}

// ------------------------------------------------------------ Option B (TOML)

#[derive(thiserror::Error, Debug)]
pub enum CounterFileError {
    #[error("invalid counters file: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("counter {0:?} has reset = \"never\" but no epoch")]
    MissingEpoch(String),
}

#[derive(Deserialize)]
struct CounterFile {
    counter: Vec<CounterSpec>,
}

#[derive(Deserialize)]
struct CounterSpec {
    key: String,
    label: String,
    category: Category,
    #[serde(default)]
    base: f64,
    epoch: Option<DateTime<Utc>>,
    per_year: f64,
    reset: Reset,
    source: String,
    unit: Option<String>,
}

/// Parses Option B counter definitions (see `assets/counters.toml`).
pub fn load_estimated(toml_src: &str) -> Result<Vec<Counter>, CounterFileError> {
    let file: CounterFile = toml::from_str(toml_src)?;
    file.counter
        .into_iter()
        .map(|spec| {
            let epoch = match (spec.reset, spec.epoch) {
                (_, Some(epoch)) => SystemTime::from(epoch),
                (Reset::Never, None) => return Err(CounterFileError::MissingEpoch(spec.key)),
                (_, None) => UNIX_EPOCH,
            };
            Ok(Counter {
                key: spec.key,
                label: spec.label,
                unit: spec.unit,
                category: spec.category,
                base: spec.base,
                rate_per_sec: spec.per_year / SECONDS_PER_YEAR,
                epoch,
                reset: spec.reset,
                source: Source::Estimated(spec.source),
            })
        })
        .collect()
}

// ------------------------------------------------------- Option A (energy)

/// December 31 (00:00 UTC) of `year`, matching the site's `Date.UTC(year, 11, 31)`.
fn reserves_epoch(year: i32) -> SystemTime {
    Utc.with_ymd_and_hms(year, 12, 31, 0, 0, 0)
        .single()
        .map(SystemTime::from)
        .unwrap_or(UNIX_EPOCH)
}

fn daily(key: &str, label: &str, unit: &str, per_sec: f64) -> Counter {
    Counter {
        key: key.to_string(),
        label: label.to_string(),
        unit: Some(unit.to_string()),
        category: Category::Energy,
        base: 0.0,
        rate_per_sec: per_sec,
        epoch: UNIX_EPOCH,
        reset: Reset::Daily,
        source: Source::Worldometers,
    }
}

fn remaining(key: &str, label: &str, unit: &str, reserve: &Reserve) -> Counter {
    Counter {
        key: key.to_string(),
        label: label.to_string(),
        unit: Some(unit.to_string()),
        category: Category::Energy,
        base: reserve.reserves,
        rate_per_sec: -reserve.production_per_year / SITE_SECONDS_PER_YEAR,
        epoch: reserves_epoch(reserve.reserves_year),
        reset: Reset::Never,
        source: Source::Worldometers,
    }
}

/// Energy counters built from the site's constants, using the site's formulas.
pub fn energy_counters(c: &EnergyConstants) -> Vec<Counter> {
    // EJ/year → MWh/s: × 10^18 J/EJ ÷ 3.6×10^9 J/MWh ÷ seconds per year.
    let total_mwh_per_sec = c.energy_total_ej_per_year * 1e18 / 3.6e9 / SITE_SECONDS_PER_YEAR;
    let oil = &c.oil;
    let oil_days = Counter {
        key: "oil_days_left".to_string(),
        label: "Days of oil left".to_string(),
        unit: Some("days".to_string()),
        base: oil.reserves / (oil.production_per_year / 365.0),
        // One day of production per day.
        rate_per_sec: -1.0 / 86_400.0,
        ..remaining("oil_days_left", "", "", oil)
    };
    vec![
        daily(
            "energy_used_today",
            "Energy used today",
            "MWh",
            total_mwh_per_sec,
        ),
        daily(
            "energy_non_renewable_today",
            "– from non-renewable sources",
            "MWh",
            total_mwh_per_sec * (1.0 - c.low_carbon_share),
        ),
        daily(
            "energy_renewable_today",
            "– from low-carbon sources",
            "MWh",
            total_mwh_per_sec * c.low_carbon_share,
        ),
        daily(
            "oil_pumped_today",
            "Oil pumped today",
            "barrels",
            oil.production_per_year / SITE_SECONDS_PER_YEAR,
        ),
        remaining("oil_left", "Oil left", "barrels", oil),
        oil_days,
        remaining("gas_left", "Natural gas left", "m³", &c.gas),
        remaining("coal_left", "Coal left", "tonnes", &c.coal),
    ]
}

/// Convenience for tests and callers holding a Unix timestamp.
pub fn at_unix(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc() -> FixedOffset {
        FixedOffset::east_opt(0).unwrap()
    }

    fn counter(reset: Reset, base: f64, rate: f64, epoch: SystemTime) -> Counter {
        Counter {
            key: "k".into(),
            label: "l".into(),
            unit: None,
            category: Category::Population,
            base,
            rate_per_sec: rate,
            epoch,
            reset,
            source: Source::Estimated("test".into()),
        }
    }

    #[test]
    fn never_reset_grows_linearly_from_epoch() {
        let epoch = at_unix(1_000);
        let c = counter(Reset::Never, 100.0, 2.0, epoch);
        assert_eq!(c.value_at(at_unix(1_000), utc()), 100.0);
        assert_eq!(c.value_at(at_unix(1_010), utc()), 120.0);
        // Before the epoch the value extrapolates backwards.
        assert_eq!(c.value_at(at_unix(990), utc()), 80.0);
    }

    #[test]
    fn negative_rates_decrease() {
        let c = counter(Reset::Never, 100.0, -0.5, at_unix(0));
        assert_eq!(c.value_at(at_unix(100), utc()), 50.0);
    }

    #[test]
    fn daily_reset_counts_from_local_midnight() {
        // 2026-11-09T06:00:00Z
        let t = Utc.with_ymd_and_hms(2026, 11, 9, 6, 0, 0).unwrap();
        let c = counter(Reset::Daily, 0.0, 1.0, UNIX_EPOCH);
        assert_eq!(c.value_at(t.into(), utc()), 6.0 * 3600.0);
        // In UTC+4 it is 10:00 local.
        let plus4 = FixedOffset::east_opt(4 * 3600).unwrap();
        assert_eq!(c.value_at(t.into(), plus4), 10.0 * 3600.0);
    }

    #[test]
    fn yearly_reset_counts_from_local_new_year() {
        let t = Utc.with_ymd_and_hms(2026, 1, 2, 0, 0, 0).unwrap();
        let c = counter(Reset::Yearly, 0.0, 1.0, UNIX_EPOCH);
        assert_eq!(c.value_at(t.into(), utc()), 86_400.0);
    }

    #[test]
    fn bundled_counters_parse() {
        let counters = load_estimated(DEFAULT_COUNTERS_TOML).unwrap();
        assert!(counters.len() >= 5);
        assert!(
            counters
                .iter()
                .all(|c| matches!(c.source, Source::Estimated(_)))
        );
        let pop = counters
            .iter()
            .find(|c| c.key == "world_population")
            .unwrap();
        let at_epoch = pop.value_at(pop.epoch, utc());
        assert_eq!(at_epoch, 8_300_678_396.0);
    }

    #[test]
    fn never_without_epoch_is_rejected() {
        let src = r#"
            [[counter]]
            key = "x"
            label = "X"
            category = "population"
            per_year = 1
            reset = "never"
            source = "s"
        "#;
        assert!(matches!(
            load_estimated(src),
            Err(CounterFileError::MissingEpoch(_))
        ));
    }

    #[test]
    fn energy_counters_follow_site_formulas() {
        let constants = EnergyConstants {
            oil: Reserve {
                reserves: 1_000.0,
                reserves_year: 2025,
                production_per_year: SITE_SECONDS_PER_YEAR,
            },
            gas: Reserve {
                reserves: 10.0,
                reserves_year: 2024,
                production_per_year: 1.0,
            },
            coal: Reserve {
                reserves: 10.0,
                reserves_year: 2023,
                production_per_year: 1.0,
            },
            energy_total_ej_per_year: 3.6e9 * SITE_SECONDS_PER_YEAR / 1e18,
            low_carbon_share: 0.25,
        };
        let counters = energy_counters(&constants);
        let get = |k: &str| counters.iter().find(|c| c.key == k).unwrap();
        assert!((get("energy_used_today").rate_per_sec - 1.0).abs() < 1e-9);
        assert!((get("energy_renewable_today").rate_per_sec - 0.25).abs() < 1e-9);
        let oil = get("oil_left");
        // 1 barrel per second, starting 2025-12-31.
        let ten_secs_later = oil.epoch + Duration::from_secs(10);
        assert_eq!(oil.value_at(ten_secs_later, utc()), 990.0);
        let days = get("oil_days_left");
        let one_day_later = days.epoch + Duration::from_secs(86_400);
        assert!(
            (days.value_at(days.epoch, utc()) - days.value_at(one_day_later, utc()) - 1.0).abs()
                < 1e-9
        );
    }
}
