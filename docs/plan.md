# worldo-tui — Specification

A terminal UI (TUI) for browsing statistics published on https://www.worldometers.info, written in Rust with `ratatui`.

Status: phases 0–6 implemented (see `docs/DISCOVERY.md` for facts that replaced assumptions: counters are split between Option A for energy and Option B for population; the country URL pattern is confirmed). This document defines the full app and the order to build it in.

---

## 1. Goals and non-goals

### Goals
1. Browse worldometers' country tables (population, plus selected other tables) in a fast, keyboard-driven terminal UI.
2. Show a **live counters dashboard** (world population, births today, deaths today, etc.) that ticks every second, like the homepage.
3. Show a **country detail view** (historical population, density, etc.).
4. Work offline from a local cache; never hammer the site.
5. Be a clean, idiomatic Rust codebase that is easy to learn from: small modules, pure parsing functions, tests on fixtures.

### Non-goals
- No async runtime (no tokio). Threads + channels are enough for a handful of HTTP requests.
- No plugin system, no generic "scrape any page" engine. Each supported page gets an explicit parser.
- No republishing/serving of the data (HTTP API, web UI). Personal use only.
- No charts library beyond what `ratatui` provides (`Sparkline`, `BarChart`, `Chart` are enough).

---

## 2. Hard constraints (read before writing code)

| # | Constraint | Consequence |
|---|---|---|
| C1 | The homepage counters are **not in the HTML**. The page ships "retrieving data..." placeholders; JavaScript computes values client-side (base value + rate × elapsed time). | Scraping the homepage HTML yields nothing. Counters must be computed locally (see §6). |
| C2 | There is no public API. Every page is HTML that can change without notice. | Every parser must: find tables by **header text**, not CSS ids; map columns by header name; fail with a clear error, never panic. |
| C3 | Worldometers' terms restrict automated reuse. | Identify honestly in User-Agent, fetch only on startup/explicit refresh/stale cache, cache aggressively (TTL ≥ 6h), max 1 request per second, never auto-poll. |
| C4 | Development sandbox may not reach the site. | All parser tests run against **saved HTML fixtures** in `tests/fixtures/`. Network is only touched by `cargo run`. |

---

## 3. Phases and acceptance criteria

Implement strictly in order. Each phase ends with `cargo fmt && cargo clippy -- -D warnings && cargo test` green.

### Phase 0 — Discovery spike (no app code)
Purpose: replace assumptions in this spec with facts.

Tasks:
1. Download and save as fixtures (with `curl -A "worldo-tui/0.1 (personal use)"`):
   - `https://www.worldometers.info/world-population/population-by-country/` → `tests/fixtures/population_by_country.html`
   - One country page, e.g. Georgia. **Verify the real URL pattern** (expected to look like `/world-population/<slug>-population/`; confirm by following a link from the country table) → `tests/fixtures/country_georgia.html`
   - The homepage → `tests/fixtures/home.html`
   - Each extra table in Phase 4 (verify URLs exist first).
2. In the homepage, locate the counter script(s). Determine **how base values and rates are obtained**: inline JS constants, a separate `.js` file, or a JSON/XHR endpoint. Save any relevant JS to `tests/fixtures/`.
3. Write `docs/DISCOVERY.md` recording: real URLs, table header texts per page, the counter mechanism, and which counters are extractable.

Acceptance: `docs/DISCOVERY.md` exists; fixtures committed; this spec updated where facts differ.

### Phase 1 — Harden v0.1 (country table)
1. Replace the generic `Vec<Vec<String>>` model with a typed `CountryRow` (§5), built by mapping **header names → column indices**. Unknown extra columns are ignored; missing required columns → `ParseError::MissingColumn(name)`.
2. Parser tests run against `tests/fixtures/population_by_country.html` (row count > 200, India/China present, populations parse).
3. Number formatting in UI: thousands separators, right-aligned numeric columns, negative % in red, positive in green.
4. Sort by column keeps working; sort by name uses case-insensitive comparison.
5. Selecting a row and pressing `Enter` opens the country detail (stub screen until Phase 3).

Acceptance: snapshot test of the table screen (§9) passes; parser tests use the real fixture.

### Phase 2 — Cache + offline
1. Cache dir: `directories::ProjectDirs::from("info", "madbinary", "worldo-tui")` → `cache_dir()` (on macOS: `~/Library/Caches/info.madbinary.worldo-tui/`).
2. Store raw HTML per page: `<page_key>.html` + `<page_key>.meta.json` (`{ "url", "fetched_at" (RFC3339) }`).
3. Load order: fresh cache (age < TTL) → network → stale cache (with "STALE" badge) → error screen.
4. `r` forces a network fetch for the current screen, ignoring TTL.
5. Status bar shows `Updated 2h ago` / `STALE (3d)` / `OFFLINE`.
6. Rate limiter: global `Mutex<Instant>` ensuring ≥ 1s between requests.

Acceptance: app starts and works with network disabled if cache exists; unit tests for TTL logic using an injected clock (`fn is_fresh(fetched_at, now, ttl) -> bool`).

### Phase 3 — Country detail view
Contents (adjust to what Phase 0 found on the page):
- Header: country name, current population, world rank, share of world population.
- Historical population table (year, population, yearly change %, density…).
- `Sparkline` or `Chart` of population over time.
- `Esc`/`Backspace` returns to the country list with selection preserved.

Acceptance: parser test on `country_georgia.html`; snapshot test of the detail screen.

### Phase 4 — Live counters dashboard
See §6. Screen shows a grid of counter cards grouped by category; values update every second with no network traffic.

Acceptance: `Counter::value_at(t)` unit-tested; dashboard snapshot test with a fixed clock.

### Phase 5 — Additional tables (optional, pick per DISCOVERY.md)
Candidates (confirm URLs and headers in Phase 0 first): GDP by country, CO2 emissions by country, countries by area. Each gets: fixture, typed row struct, parser, tab in the UI.

### Phase 6 — Polish
- `--export csv <table>` CLI flag (use `clap`) writing the current table to stdout.
- Config file `config.toml` in `config_dir()`: cache TTL, default tab, color theme.
- `?` opens a help popup listing all keybindings.
- Release profile: `lto = true`, `strip = true`. Install via `cargo install --path .`.

---

## 4. Architecture

```
            ┌──────────────┐  Msg::Loaded(..)   ┌────────────────────┐
 key events │  UI thread   │◀───────────────────│  fetch worker(s)   │
 ─────────▶ │  App state   │                    │  (std::thread)     │
            │  draw loop   │ Request(PageKey) ─▶│  cache → http →    │
            └──────┬───────┘                    │  parse             │
                   │ every 100ms tick           └────────────────────┘
                   ▼
            counters recomputed from clock (no I/O)
```

Rules:
- **Single owner of state**: `App` lives on the UI thread. Workers never touch it; they send `Msg` values through `std::sync::mpsc`.
- **Parsing is pure**: `fn parse_x(html: &str) -> Result<X, ParseError>`. No I/O inside parsers.
- **Fetching is dumb**: `fn fetch(url) -> Result<String, FetchError>` returns HTML text only.
- **UI never blocks**: no network or disk I/O in `draw` or `on_key`.
- **Event loop**: `event::poll(100ms)` → handle key → drain channel with `try_recv` → advance tick → `draw`.

### Module layout

```
src/
  main.rs            // init terminal, run loop, restore terminal; arg parsing (Phase 6)
  app.rs             // App struct, Screen enum, on_key, on_msg
  msg.rs             // Msg enum (worker → UI)
  net/
    mod.rs
    fetch.rs         // reqwest blocking client, UA, timeout, rate limit
    cache.rs         // read/write cache files, TTL logic
    worker.rs        // spawns threads: cache→network→parse, sends Msg
  parse/
    mod.rs           // shared helpers: find_table_by_header, header index map, parse_number
    population.rs    // CountryRow parser
    country.rs       // CountryDetail parser
    counters.rs      // counter definitions extraction (per DISCOVERY.md)
  model.rs           // CountryRow, CountryDetail, HistoryPoint, Counter
  ui/
    mod.rs           // top-level draw: tabs + body + status bar
    table.rs         // country table screen
    detail.rs        // country detail screen
    dashboard.rs     // counters screen
    help.rs          // help popup
    format.rs        // number formatting (1,234,567 / 1.23 B / +0.91 %)
tests/
  fixtures/*.html
  parse_population.rs
  parse_country.rs
```

---

## 5. Data model

```rust
pub struct CountryRow {
    pub rank: u32,
    pub name: String,
    pub detail_url: Option<String>, // href from the country cell, absolute URL
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

pub struct HistoryPoint { pub year: i32, pub population: u64, pub yearly_change_pct: Option<f64>, pub density: Option<f64> }

pub struct CountryDetail { pub name: String, pub population: u64, pub rank: Option<u32>, pub history: Vec<HistoryPoint> }

pub struct Counter {
    pub key: &'static str,      // "world_population"
    pub label: &'static str,    // "Current World Population"
    pub category: Category,     // Population, Health, Energy, ...
    pub base: f64,              // value at `epoch`
    pub rate_per_sec: f64,      // may be negative
    pub epoch: SystemTime,
    pub reset: Reset,           // Never | Daily | Yearly  ("births today" resets at local midnight)
}
```

Column mapping: header text is matched case-insensitively by **substring** (e.g. `"population"` + not `"urban"`; `"yearly change"`; `"density"`). Put the matching rules in one table (`const COLUMNS: &[(Field, &[&str])]`) so a site change is a one-line fix.

Numbers: `parse_number("1,463,865,525") == Some(1463865525.0)`, `"-0.5 %" → -0.5`, `"N.A."`/`""` → `None`. Never `unwrap` a parse of site data.

---

## 6. Live counters design

Worldometers computes counters in the browser from a starting value and a per-second rate. We do the same:

```
value(t) = base + rate_per_sec × (t − epoch)
```

For `Reset::Daily` / `Reset::Yearly`, `epoch` is the start of the current local day/year and `base` is 0.

**Where base/rate come from** — decided by Phase 0:
- **Option A (preferred if possible):** base/rate are extractable from the site's JS or an endpoint → `parse/counters.rs` extracts them; cached like any page.
- **Option B (fallback):** ship `counters.toml` with base/rate/epoch per counter, sourced from public annual figures (e.g. UN World Population Prospects: births/year ÷ 31,557,600 s). Label the dashboard "Estimated" and show the source.

Never present option-B values as worldometers data.

UI: cards with label, big number (thousands separators), and `+N/s` rate. Redraw each tick; values are computed in `draw`, which is pure (clock passed in as `SystemTime` so tests can freeze time).

---

## 7. UI

### Layout
```
┌ worldo-tui ─ [1] Countries  [2] Live  [3] GDP … ───────────────────────────┐
│                                                                            │
│                      (active screen body)                                  │
│                                                                            │
├────────────────────────────────────────────────────────────────────────────┤
│ / search  s sort  Enter detail  r refresh  ? help  q quit   Updated 2h ago │
└────────────────────────────────────────────────────────────────────────────┘
```

### Countries screen
```
 #   Country           Population ▼   Yearly %   Net change   Density   Med. age
 1   India          1,463,865,525     +0.89 %   12,929,734       492       28
›2   China          1,416,096,094     −0.23 %   −3,225,184       151       40
```

### Detail screen
```
 Georgia  ·  Population 3,806,048  ·  Rank #132  ·  0.05 % of world
 ▁▂▃▅▆▇█▇▆▅▅▄▄▄▄   (population 1955 → now)
 Year   Population   Yearly %   Density
 2025   3,806,048    −0.36 %    55
```

### Keybindings
| Key | Scope | Action |
|---|---|---|
| `1`..`9`, `Tab`/`Shift-Tab` | global | switch tab |
| `j`/`k`, `↓`/`↑` | lists | move selection |
| `g` / `G` | lists | first / last |
| `PgDn` / `PgUp` | lists | page |
| `h`/`l`, `←`/`→` | tables | change sort column |
| `s` | tables | toggle asc/desc |
| `/` | tables | search mode (`Enter` keep, `Esc` clear) |
| `Enter` | country table | open detail |
| `Esc` / `Backspace` | detail, popup | back |
| `r` | global | force refresh current screen |
| `?` | global | help popup |
| `q`, `Ctrl-C` | global | quit |

Terminal hygiene: always restore the terminal on exit **and on panic** (`ratatui::init()` installs the panic hook; keep it).

---

## 8. Errors

```rust
#[derive(thiserror::Error, Debug)]
pub enum FetchError { #[error("network: {0}")] Http(#[from] reqwest::Error), #[error("HTTP {0}")] Status(u16) }

#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("table with header containing {0:?} not found — page layout may have changed")] TableNotFound(&'static str),
    #[error("required column {0:?} missing")] MissingColumn(&'static str),
    #[error("table has no rows")] Empty,
}
```
- Libraries/modules return typed errors (`thiserror`); `main.rs` uses `anyhow`.
- No `unwrap()`/`expect()` outside tests and constant `Selector::parse`.
- Errors show in the screen body with the message and "press r to retry"; stale cache is shown instead when available.

---

## 9. Testing

| Kind | Tool | What |
|---|---|---|
| Parser | `cargo test` on `tests/fixtures/*.html` | row counts, known countries, numeric parsing, missing-column error with a doctored fixture |
| Pure logic | unit tests | `parse_number`, `is_fresh`, `Counter::value_at`, sorting, search filter |
| UI snapshots | `ratatui::backend::TestBackend` + `insta` | each screen at 120×30 with fixed data and frozen clock |
| Manual | `cargo run` | real network, offline mode (disable Wi-Fi), resize terminal |

No test may hit the network.

---

## 10. Dependencies

| Crate | Purpose |
|---|---|
| `ratatui` 0.29, `crossterm` 0.28 | TUI |
| `reqwest` 0.12 (`blocking`, `rustls-tls`, no default features) | HTTP |
| `scraper` 0.20 | HTML parsing |
| `anyhow`, `thiserror` | errors |
| `directories` | cache/config paths |
| `serde`, `serde_json`, `toml` | cache meta, config |
| `chrono` (or `jiff`) | timestamps, local midnight for counters |
| `clap` (Phase 6) | CLI flags |
| `insta` (dev) | snapshot tests |

Add a crate only when the phase that needs it starts.

---

## 11. Definition of done (whole app)
- All phases 0–4 complete; 5–6 as chosen.
- `cargo clippy -- -D warnings` clean, `cargo test` green, no network in tests.
- Works offline from cache; shows staleness honestly.
- README with screenshots (or asciinema), install instructions (`cargo install --path .`), keybindings, and the data-source/terms note.