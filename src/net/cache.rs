//! Raw-HTML cache: `<key>.html` + `<key>.meta.json` (`{ "url", "fetched_at" }`).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::{DateTime, TimeDelta, Utc};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meta {
    pub url: String,
    /// RFC 3339.
    pub fetched_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub body: String,
    pub meta: Meta,
}

/// Whether data fetched at `fetched_at` is still fresh at `now`.
/// A timestamp in the future (clock skew) counts as fresh.
pub fn is_fresh(fetched_at: DateTime<Utc>, now: DateTime<Utc>, ttl: TimeDelta) -> bool {
    now.signed_duration_since(fetched_at) < ttl
}

pub fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("info", "madbinary", "worldo-tui")
}

#[derive(Debug, Clone)]
pub struct Cache {
    dir: PathBuf,
}

impl Cache {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// `~/Library/Caches/info.madbinary.worldo-tui/` on macOS.
    pub fn default_dir() -> Option<PathBuf> {
        project_dirs().map(|d| d.cache_dir().to_path_buf())
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn body_path(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{key}.html"))
    }

    fn meta_path(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{key}.meta.json"))
    }

    /// The cached entry, or `None` if missing or unreadable.
    pub fn read(&self, key: &str) -> Option<Entry> {
        let meta: Meta =
            serde_json::from_str(&fs::read_to_string(self.meta_path(key)).ok()?).ok()?;
        let body = fs::read_to_string(self.body_path(key)).ok()?;
        Some(Entry { body, meta })
    }

    /// Writes body then metadata, each via a temp file + rename so a crash never
    /// leaves a half-written entry that looks valid.
    pub fn write(
        &self,
        key: &str,
        url: &str,
        body: &str,
        fetched_at: DateTime<Utc>,
    ) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let meta = Meta {
            url: url.to_string(),
            fetched_at,
        };
        let meta_json = serde_json::to_string_pretty(&meta).map_err(io::Error::other)?;
        write_atomic(&self.body_path(key), body)?;
        write_atomic(&self.meta_path(key), &meta_json)
    }
}

fn write_atomic(path: &Path, contents: &str) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, contents)?;
    fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t(h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 9, h, 0, 0).unwrap()
    }

    #[test]
    fn freshness_respects_ttl() {
        let ttl = TimeDelta::hours(6);
        assert!(is_fresh(t(0), t(0), ttl));
        assert!(is_fresh(t(0), t(5), ttl));
        assert!(!is_fresh(t(0), t(6), ttl));
        assert!(!is_fresh(t(0), t(12), ttl));
        // Clock skew: fetched "in the future".
        assert!(is_fresh(t(12), t(0), ttl));
    }

    #[test]
    fn round_trips_entries() {
        let dir = std::env::temp_dir().join(format!("worldo-tui-test-{}", std::process::id()));
        let cache = Cache::new(&dir);
        assert!(cache.read("population").is_none());
        cache
            .write("population", "https://example.test/", "<html></html>", t(3))
            .unwrap();
        let entry = cache.read("population").unwrap();
        assert_eq!(entry.body, "<html></html>");
        assert_eq!(entry.meta.url, "https://example.test/");
        assert_eq!(entry.meta.fetched_at, t(3));
        let meta_json = fs::read_to_string(dir.join("population.meta.json")).unwrap();
        assert!(meta_json.contains("2026-11-09T03:00:00Z"));
        fs::remove_dir_all(dir).unwrap();
    }
}
