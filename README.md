# worldo-tui

A fast, keyboard-driven terminal UI for browsing statistics published on
[worldometers.info](https://www.worldometers.info), written in Rust with
[`ratatui`](https://ratatui.rs).

```
 worldo-tui   [1] Live   [2] Countries   [3] GDP   [4] CO₂   [5] Area
   # ▲ Country                    Population   Yearly %   Net change   Density   Med. age   Fert.   World %   Urban %
›    1 India                   1,476,625,576    +0.87 %   12,760,051       497       29.2    1.93   17.80 %   36.00 %
     2 China                   1,412,914,089    −0.22 %   −3,182,005       151       40.6    1.03   17.00 %   66.80 %
     3 United States             349,035,494    +0.51 %    1,759,686        38       38.7    1.62    4.20 %   80.30 %
 / search  h/l sort col  s asc/desc  Enter detail  r refresh  ? help  q quit                              Updated 2h ago
```

More screens (country detail with chart, live counters, help) are in
[`tests/snapshots/`](tests/snapshots/). They are the exact text the UI renders
at 120×30.

## Features

- **Countries**: population by country, with sort by any column, search, and
  a detail view for each country (history since 1955, forecast to 2050, chart).
- **Live**: counters that tick every second with no network traffic.
  - *Population* counters are **estimates** computed locally from UN World
    Population Prospects 2024 annual figures, and are labelled as such.
    Worldometers' own population counters come from an obfuscated script whose
    licence forbids reuse, so we don't use it.
  - *Energy* counters use the constants from worldometers' readable
    live-counter module and the same formulas the site uses.
- **GDP**, **CO₂ emissions** and **Area** tables.
- **Offline**: every page is cached on disk. Without a network the app shows
  the cached copy and marks it `OFFLINE` / `STALE (3d)`.
- `--export csv <table>` writes raw values to stdout.

## Install

```bash
cargo install --path .
```

This installs the `worldo-tui` binary.

## Usage

```bash
worldo-tui                          # start the UI
worldo-tui --offline                # never touch the network; cache only
worldo-tui --export csv countries   # countries | gdp | co2 | area
worldo-tui --config path/to/config.toml
```

### Keybindings

| Key | Scope | Action |
|---|---|---|
| `1`..`5`, `Tab`/`Shift-Tab` | global | switch tab |
| `j`/`k`, `↓`/`↑` | lists | move selection |
| `g` / `G` | lists | first / last |
| `PgDn` / `PgUp` | lists | page |
| `h`/`l`, `←`/`→` | tables | change sort column |
| `s` | tables | toggle asc/desc |
| `/` | tables | search (`Enter` keep, `Esc` clear) |
| `Enter` | country table | open detail |
| `Esc` / `Backspace` | detail, popup | back |
| `r` | global | force refresh current screen |
| `?` | global | help popup |
| `q`, `Ctrl-C` | global | quit |

### Configuration

Optional `config.toml` in the platform config directory
(`~/Library/Application Support/info.madbinary.worldo-tui/` on macOS,
`~/.config/worldo-tui/` on Linux):

```toml
cache_ttl_hours = 12       # minimum 6
default_tab = "live"       # live | countries | gdp | co2 | area
theme = "default"          # default | mono
```

To change the estimated counters, copy [`assets/counters.toml`](assets/counters.toml)
into the same directory and edit it.

The cache lives in `~/Library/Caches/info.madbinary.worldo-tui/` on macOS
(`~/.cache/worldo-tui/` on Linux). Each page is stored as `<key>.html` plus
`<key>.meta.json` (`{"url", "fetched_at"}`).

## Data source and terms

All data comes from [worldometers.info](https://www.worldometers.info), whose
terms restrict automated reuse. This tool is for **personal use**, and it
tries to be a good citizen:

- It identifies itself honestly (`User-Agent: worldo-tui/<version> (personal use)`).
- It fetches only on startup or first visit of a tab, on an explicit `r`, or
  when the cache is older than the TTL (at least 6 hours). It never polls.
- It sends at most one request per second.
- It doesn't republish or serve the data.

## Development

```bash
cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
```

- Parsers are pure functions tested against saved pages in `tests/fixtures/`.
  No test touches the network.
- UI snapshot tests use `ratatui`'s `TestBackend` with
  [`insta`](https://insta.rs). After an intentional UI change, run
  `INSTA_UPDATE=always cargo test` and review the diff in `tests/snapshots/`.
- Design: [`docs/plan.md`](docs/plan.md). Facts about the site (URLs, table
  headers, how counters work): [`docs/DISCOVERY.md`](docs/DISCOVERY.md).
- When the site changes a column name, edit the `COLUMNS` rule table at the
  top of the matching file in `src/parse/`.
