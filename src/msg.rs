//! Messages between the UI thread and the fetch worker.

use chrono::{DateTime, Utc};

use crate::model::{CountryDetail, EnergyConstants};
use crate::parse::BASE_URL;
use crate::tables::Table;

/// A page (or group of pages) the worker can load.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PageKey {
    Population,
    Gdp,
    Co2,
    Area,
    /// Homepage + energy-counter script.
    Counters,
    /// A country page, by absolute URL.
    Country(String),
}

impl PageKey {
    /// File-name-safe cache key.
    pub fn cache_key(&self) -> String {
        match self {
            PageKey::Population => "population".into(),
            PageKey::Gdp => "gdp".into(),
            PageKey::Co2 => "co2".into(),
            PageKey::Area => "area".into(),
            PageKey::Counters => "counters".into(),
            PageKey::Country(url) => format!("country_{}", slug(url)),
        }
    }

    /// URL of the main page. For `Counters` this is the homepage.
    pub fn url(&self) -> String {
        match self {
            PageKey::Population => format!("{BASE_URL}/world-population/population-by-country/"),
            PageKey::Gdp => format!("{BASE_URL}/gdp/gdp-by-country/"),
            PageKey::Co2 => format!("{BASE_URL}/co2-emissions/co2-emissions-by-country/"),
            PageKey::Area => format!("{BASE_URL}/geography/largest-countries-in-the-world/"),
            PageKey::Counters => format!("{BASE_URL}/"),
            PageKey::Country(url) => url.clone(),
        }
    }
}

/// Last path segment of a URL, reduced to `[a-z0-9-]`.
/// `https://…/world-population/georgia-population/` → `georgia-population`.
pub fn slug(url: &str) -> String {
    let segment = url
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default();
    let cleaned: String = segment
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    if cleaned.is_empty() {
        "unknown".into()
    } else {
        cleaned
    }
}

/// How current the delivered data is.
#[derive(Debug, Clone, PartialEq)]
pub enum Freshness {
    /// Fetched now, or read from a cache entry younger than the TTL.
    Fresh { fetched_at: DateTime<Utc> },
    /// The network failed; this is the cached copy.
    Offline {
        fetched_at: DateTime<Utc>,
        error: String,
    },
}

impl Freshness {
    pub fn fetched_at(&self) -> DateTime<Utc> {
        match self {
            Freshness::Fresh { fetched_at } | Freshness::Offline { fetched_at, .. } => *fetched_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PageData {
    Table(Table),
    Country(CountryDetail),
    Energy(EnergyConstants),
}

/// UI → worker.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub key: PageKey,
    /// Ignore the TTL and go to the network.
    pub force: bool,
}

/// Worker → UI.
#[derive(Debug, Clone, PartialEq)]
pub enum Msg {
    Loaded {
        key: PageKey,
        data: PageData,
        freshness: Freshness,
    },
    Failed {
        key: PageKey,
        error: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_keys_are_file_safe() {
        let key = PageKey::Country(
            "https://www.worldometers.info/world-population/georgia-population/".into(),
        );
        assert_eq!(key.cache_key(), "country_georgia-population");
        assert_eq!(slug("https://x/../a b?c/"), "a-b-c");
        assert_eq!(slug(""), "unknown");
    }
}
