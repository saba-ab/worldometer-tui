# Discovery (Phase 0)

Recorded 2026-11-09 by fetching pages with `curl -A "worldo-tui/0.1 (personal use)"`.
All fixtures live in `tests/fixtures/`.

## Pages and URLs

| Page key | URL | Fixture |
|---|---|---|
| `population` | `https://www.worldometers.info/world-population/population-by-country/` | `population_by_country.html` |
| `country_<slug>` | `https://www.worldometers.info/world-population/<slug>-population/` (from the `href` of the country cell, e.g. `/world-population/georgia-population/`) | `country_georgia.html` |
| `home` | `https://www.worldometers.info/` | `home.html` |
| `counters_js` | `https://cdn.worldometers.info/assets/rts-live-counter-v2.astro_astro_type_script_index_0_lang.<hash>.js` (URL is content-hashed; discover it from the `<script src>` in `home.html`) | `rts_live_counter_v2.js` |
| `gdp` | `https://www.worldometers.info/gdp/gdp-by-country/` | `gdp_by_country.html` |
| `co2` | `https://www.worldometers.info/co2-emissions/co2-emissions-by-country/` | `co2_by_country.html` |
| `area` | `https://www.worldometers.info/geography/largest-countries-in-the-world/` | `largest_countries.html` |

The site is built with Astro. Tables have no stable ids (`<table id class="datatable …">`), so we find them by header text, as the spec requires.

## Table headers

**Population by country** (1 table, 239 rows, one `<tr id="<slug>">` each):
`# | Country or other entity | Population 2026 | Yearly Change | Net Change | Density (P/km²) | Land Area (km²) | Migrants (net) | Fert. Rate | Median Age | Urban Pop % | World Share`

- The country cell has `<a href="/world-population/<slug>-population/">Name</a>`.
- Negative numbers use U+2212 MINUS SIGN (`−440,456`, `−0.05%`), not ASCII `-`.
- Some cells have footnotes: `36.0%<sup>…[1]</sup>`. Text inside `<sup>` must be ignored.
- Every `<td>` also has a `data-order` attribute (scaled integer). We parse the visible text instead.
- Two rows (Pitcairn Islands, Åland Islands) show `–` (en dash) for population and every numeric column. Population is required, so the parser skips them and the app lists 237 of 239 entities.

**Country page** (e.g. Georgia), 3 tables:
1. History, 20 rows (2026, 2025, 2024, 2023, 2022, 2020, 2015, … 1955), newest first:
   `Year | Population | Yearly % Change | Yearly Change | Migrants (net) | Median Age | Fertility Rate | Density (P/km²) | Urban Pop % | Urban Population | Country's Share of World Pop | World Population | Georgia Global Rank`
2. Forecast (same headers, 5 rows: 2030 … 2050).
3. Largest cities: `Rank | City or Urban Settlement | Population` (not used).

The name is in `<h1>…Georgia Population <small>(LIVE)</small></h1>`. The live counter on this page is also `retrieving data...` (see below), so the "current population" we show is the newest history row.

**GDP by country**, 2 tables (first one = latest IMF estimates):
`# | Country | GDP (US$) | GDP (Full Value, US$) | GDP Growth | GDP per Capita (US$)` (the second table adds `Share of World GDP`).
`GDP (US$)` is abbreviated (`$32.64 trillion`), so we read `GDP (Full Value, US$)` (`$32,642,373,000,000`). The country name is a plain `<span>`, with no link.

**CO2 emissions by country**, 1 table, 207 rows:
`# | Country | CO2 emissions (tons, 2024) | 1 Year Change | Per Capita | Share of World`

**Largest countries by area**, 1 table, 239 rows:
`# | Country or other entity | Tot. Area (km²) | Tot. Area (mi²) | Land Area (km²) | Land Area (mi²) | World Share`

## Counter mechanism (constraint C1)

The homepage renders every counter as `<span class="rts-counter" rel="<id>">retrieving data...</span>`. There are two mechanisms:

1. **Legacy counters** (`current_population`, `births_today`, `dth1s_this_year`, `co2_emissions/this_year`, …, about 60 ids) are filled by `https://www.realtimestatistics.net/rts/RTSp.js`. That script is **obfuscated** and starts with *"Copyright Worldometers.info. All Rights Reserved. Copy, modification, or use is strictly prohibited"*. We don't download, deobfuscate or reuse it, and we don't keep it as a fixture.
   → These counters use **Option B**: `assets/counters.toml` with annual figures from public sources (UN World Population Prospects 2024), shown as **Estimated** with the source.
2. **v2 energy counters** (`<rts-live-counter-v2 data-counter-id="v2__…">`) are computed by a small readable ES module, `rts-live-counter-v2.…js`. It holds plain constants (`generatedYear`, oil/gas/coal reserves + reserve year + annual production, total annual energy in EJ, low-carbon share as a fraction). The formulas are:
   - `…/today` counters: `rate_per_sec × seconds since midnight` (the site uses the **UTC** day; we use local midnight as the spec says), with `rate = annual / 31,536,000`.
   - `…_reserves/now`: `reserves − (annual production / 31,536,000) × (now − Dec 31 of reserve year, UTC)`.
   - Energy: `EJ × 10^18 / 3.6×10^9` = MWh per year.
   → These counters use **Option A**: `parse/counters.rs` reads the constants (cached like any other page) and labels them as worldometers data.

## Differences from the spec

- v0.1 did not exist yet (`src/main.rs` was "Hello, world!"). Phase 1 was built from scratch.
- Country URL pattern confirmed as `/world-population/<slug>-population/` (e.g. `us-population`, not `united-states-population`). Always take the `href` from the table; don't build the URL from the name.
- Counters are split: Option A for energy, Option B for population counters.
- Detail page has no separate "rank" or "share" headline. Both come from the newest history row (`<Country> Global Rank`, `Country's Share of World Pop`).
