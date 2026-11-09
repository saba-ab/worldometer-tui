//! The fetch worker: `Request` in, `Msg` out, on its own thread.
//!
//! Load order per page (docs/plan.md Phase 2): fresh cache → network → stale
//! cache → error. A page is written to the cache only after it parses, so a
//! site redesign never overwrites a good cached copy.

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use chrono::{TimeDelta, Utc};

use super::cache::{Cache, is_fresh};
use super::fetch::Fetcher;
use crate::msg::{Freshness, Msg, PageData, PageKey, Request};
use crate::parse::{self, ParseError};
use crate::tables::Table;

pub struct Loader {
    cache: Cache,
    ttl: TimeDelta,
    /// `Err` holds why the network is unavailable (offline mode, TLS init failure).
    fetcher: Result<Fetcher, String>,
}

impl Loader {
    pub fn new(cache: Cache, ttl: TimeDelta, offline: bool) -> Self {
        let fetcher = if offline {
            Err("offline mode (--offline)".to_string())
        } else {
            Fetcher::new().map_err(|e| e.to_string())
        };
        Self {
            cache,
            ttl,
            fetcher,
        }
    }

    /// Loads and parses one page, producing the message for the UI.
    pub fn load(&self, request: &Request) -> Msg {
        let key = request.key.clone();
        let force = request.force;
        let result = match &key {
            PageKey::Population => self.table(&key, force, parse::population::parse_population),
            PageKey::Gdp => self.table(&key, force, parse::tables::parse_gdp),
            PageKey::Co2 => self.table(&key, force, parse::tables::parse_co2),
            PageKey::Area => self.table(&key, force, parse::tables::parse_area),
            PageKey::Country(url) => self
                .obtain(&key.cache_key(), url, force, parse::country::parse_country)
                .map(|(detail, f)| (PageData::Country(detail), f)),
            PageKey::Counters => self.energy(force),
        };
        match result {
            Ok((data, freshness)) => Msg::Loaded {
                key,
                data,
                freshness,
            },
            Err(error) => Msg::Failed { key, error },
        }
    }

    fn table<R: crate::tables::TableRow>(
        &self,
        key: &PageKey,
        force: bool,
        parse: fn(&str) -> Result<Vec<R>, ParseError>,
    ) -> Result<(PageData, Freshness), String> {
        self.obtain(&key.cache_key(), &key.url(), force, parse)
            .map(|(rows, f)| (PageData::Table(Table::from_rows(&rows)), f))
    }

    /// Homepage → script URL → energy constants.
    fn energy(&self, force: bool) -> Result<(PageData, Freshness), String> {
        let home_url = PageKey::Counters.url();
        let (script_url, home_freshness) = self.obtain(
            "home",
            &home_url,
            force,
            parse::counters::find_counter_script_url,
        )?;
        let (constants, js_freshness) = self.obtain(
            "counters_js",
            &script_url,
            force,
            parse::counters::parse_energy_constants,
        )?;
        let freshness = match home_freshness {
            offline @ Freshness::Offline { .. } => offline,
            Freshness::Fresh { .. } => js_freshness,
        };
        Ok((PageData::Energy(constants), freshness))
    }

    /// Fresh cache → network → stale cache, parsing each candidate.
    pub fn obtain<T>(
        &self,
        cache_key: &str,
        url: &str,
        force: bool,
        parse: impl Fn(&str) -> Result<T, ParseError>,
    ) -> Result<(T, Freshness), String> {
        let now = Utc::now();
        let cached = self.cache.read(cache_key);

        if !force
            && let Some(entry) = cached
                .as_ref()
                .filter(|e| is_fresh(e.meta.fetched_at, now, self.ttl))
            && let Ok(value) = parse(&entry.body)
        {
            return Ok((
                value,
                Freshness::Fresh {
                    fetched_at: entry.meta.fetched_at,
                },
            ));
        }

        let network_error = match &self.fetcher {
            Err(reason) => reason.clone(),
            Ok(fetcher) => match fetcher.fetch(url) {
                Ok(body) => match parse(&body) {
                    Ok(value) => {
                        // A cache write failure only costs a refetch later.
                        let _ = self.cache.write(cache_key, url, &body, now);
                        return Ok((value, Freshness::Fresh { fetched_at: now }));
                    }
                    Err(e) => e.to_string(),
                },
                Err(e) => e.to_string(),
            },
        };

        match cached {
            Some(entry) => match parse(&entry.body) {
                Ok(value) => Ok((
                    value,
                    Freshness::Offline {
                        fetched_at: entry.meta.fetched_at,
                        error: network_error,
                    },
                )),
                Err(parse_error) => Err(format!(
                    "{network_error}; cached copy unusable: {parse_error}"
                )),
            },
            None => Err(network_error),
        }
    }
}

/// Spawns the worker thread. It exits when the request sender is dropped.
pub fn spawn(cache: Cache, ttl: TimeDelta, offline: bool) -> (Sender<Request>, Receiver<Msg>) {
    let (request_tx, request_rx) = mpsc::channel::<Request>();
    let (msg_tx, msg_rx) = mpsc::channel::<Msg>();
    thread::spawn(move || {
        let loader = Loader::new(cache, ttl, offline);
        for request in request_rx {
            if msg_tx.send(loader.load(&request)).is_err() {
                break;
            }
        }
    });
    (request_tx, msg_rx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeDelta;

    fn temp_cache(name: &str) -> Cache {
        let dir = std::env::temp_dir().join(format!("worldo-worker-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        Cache::new(dir)
    }

    fn parse_len(s: &str) -> Result<usize, ParseError> {
        if s.starts_with("ok") {
            Ok(s.len())
        } else {
            Err(ParseError::Empty)
        }
    }

    #[test]
    fn offline_without_cache_is_an_error() {
        let loader = Loader::new(temp_cache("none"), TimeDelta::hours(6), true);
        let result = loader.obtain("k", "https://example.test/", false, parse_len);
        assert!(result.unwrap_err().contains("offline"));
    }

    #[test]
    fn fresh_cache_is_used_without_network() {
        let cache = temp_cache("fresh");
        cache.write("k", "u", "ok-cached", Utc::now()).unwrap();
        let loader = Loader::new(cache.clone(), TimeDelta::hours(6), true);
        let (value, freshness) = loader.obtain("k", "u", false, parse_len).unwrap();
        assert_eq!(value, "ok-cached".len());
        assert!(matches!(freshness, Freshness::Fresh { .. }));
        std::fs::remove_dir_all(cache.dir()).unwrap();
    }

    #[test]
    fn stale_cache_is_served_when_network_fails() {
        let cache = temp_cache("stale");
        let old = Utc::now() - TimeDelta::days(3);
        cache.write("k", "u", "ok-old", old).unwrap();
        let loader = Loader::new(cache.clone(), TimeDelta::hours(6), true);
        let (_, freshness) = loader.obtain("k", "u", false, parse_len).unwrap();
        assert_eq!(freshness.fetched_at(), old);
        assert!(matches!(freshness, Freshness::Offline { .. }));
        // Forcing a refresh offline still falls back to the cache.
        assert!(loader.obtain("k", "u", true, parse_len).is_ok());
        std::fs::remove_dir_all(cache.dir()).unwrap();
    }
}
