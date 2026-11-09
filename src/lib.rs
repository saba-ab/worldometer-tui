//! worldo-tui: a terminal UI for browsing statistics published on worldometers.info.
//!
//! Layout (see `docs/plan.md` §4):
//! - [`parse`]: pure HTML/JS → typed data functions, tested on fixtures.
//! - [`net`]: fetch, cache and the background worker thread.
//! - [`app`]: the single owner of UI state; reacts to keys and worker messages.
//! - [`ui`]: pure drawing of [`app::App`] for a given clock.

pub mod app;
pub mod config;
pub mod counters;
pub mod export;
pub mod model;
pub mod msg;
pub mod net;
pub mod parse;
pub mod tables;
pub mod ui;
