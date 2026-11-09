//! `config.toml` in the platform config directory (Phase 6).
//!
//! ```toml
//! cache_ttl_hours = 12      # minimum 6 (constraint C3)
//! default_tab = "live"      # live | countries | gdp | co2 | area
//! theme = "default"         # default | mono
//! ```

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::TimeDelta;
use serde::Deserialize;

use crate::app::Tab;
use crate::net::cache::project_dirs;

/// Never cache for less than this, whatever the config says (constraint C3).
pub const MIN_TTL_HOURS: f64 = 6.0;
pub const DEFAULT_TTL_HOURS: f64 = 12.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeName {
    #[default]
    Default,
    /// No colours, for monochrome terminals.
    Mono,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub cache_ttl_hours: f64,
    pub default_tab: Tab,
    pub theme: ThemeName,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cache_ttl_hours: DEFAULT_TTL_HOURS,
            default_tab: Tab::Live,
            theme: ThemeName::Default,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("cannot read {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("invalid {path}: {source}")]
    Toml {
        path: PathBuf,
        source: toml::de::Error,
    },
}

impl Config {
    /// The cache TTL, clamped to at least [`MIN_TTL_HOURS`].
    pub fn ttl(&self) -> TimeDelta {
        let hours = if self.cache_ttl_hours.is_finite() {
            self.cache_ttl_hours.max(MIN_TTL_HOURS)
        } else {
            DEFAULT_TTL_HOURS
        };
        TimeDelta::seconds((hours * 3600.0) as i64)
    }

    pub fn parse(src: &str, path: &Path) -> Result<Self, ConfigError> {
        toml::from_str(src).map_err(|source| ConfigError::Toml {
            path: path.to_path_buf(),
            source,
        })
    }

    /// Loads `path`; a missing file means defaults.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        match fs::read_to_string(path) {
            Ok(src) => Self::parse(&src, path),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(ConfigError::Io {
                path: path.to_path_buf(),
                source,
            }),
        }
    }
}

pub fn config_dir() -> Option<PathBuf> {
    project_dirs().map(|d| d.config_dir().to_path_buf())
}

pub fn default_config_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("config.toml"))
}

/// Optional user override for the Option B counter definitions.
pub fn counters_override_path() -> Option<PathBuf> {
    config_dir().map(|d| d.join("counters.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_config_uses_defaults() {
        let cfg = Config::parse("", Path::new("x")).unwrap();
        assert_eq!(cfg, Config::default());
        assert_eq!(cfg.ttl(), TimeDelta::hours(12));
    }

    #[test]
    fn values_are_read_and_ttl_is_clamped() {
        let cfg = Config::parse(
            "cache_ttl_hours = 1\ndefault_tab = \"countries\"\ntheme = \"mono\"",
            Path::new("x"),
        )
        .unwrap();
        assert_eq!(cfg.default_tab, Tab::Countries);
        assert_eq!(cfg.theme, ThemeName::Mono);
        assert_eq!(cfg.ttl(), TimeDelta::hours(6));
    }

    #[test]
    fn unknown_keys_are_rejected() {
        assert!(Config::parse("colour = \"red\"", Path::new("x")).is_err());
    }

    #[test]
    fn missing_file_means_defaults() {
        let cfg = Config::load(Path::new("/definitely/not/here/config.toml")).unwrap();
        assert_eq!(cfg, Config::default());
    }
}
