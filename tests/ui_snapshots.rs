//! Snapshot tests of each screen at 120×30 with fixture data and a frozen clock.

use std::time::SystemTime;

use chrono::{DateTime, FixedOffset, TimeDelta, TimeZone, Utc};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use worldo_tui::app::{App, Tab};
use worldo_tui::counters::{DEFAULT_COUNTERS_TOML, load_estimated};
use worldo_tui::msg::{Freshness, Msg, PageData, PageKey};
use worldo_tui::parse::{counters, country, population};
use worldo_tui::tables::Table;
use worldo_tui::ui::{self, Theme};

const WIDTH: u16 = 120;
const HEIGHT: u16 = 30;

fn fetched_at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 9, 10, 0, 0).unwrap()
}

/// Two hours after the data was fetched.
fn now() -> SystemTime {
    SystemTime::from(fetched_at() + TimeDelta::hours(2))
}

fn fresh() -> Freshness {
    Freshness::Fresh {
        fetched_at: fetched_at(),
    }
}

fn app(tab: Tab) -> App {
    let estimated = load_estimated(DEFAULT_COUNTERS_TOML).map_err(|e| e.to_string());
    App::new(
        tab,
        estimated,
        TimeDelta::hours(12),
        FixedOffset::east_opt(0).unwrap(),
        Theme::default(),
    )
}

fn press(app: &mut App, code: KeyCode) {
    app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
}

fn with_population(app: &mut App) {
    let rows =
        population::parse_population(include_str!("fixtures/population_by_country.html")).unwrap();
    app.on_msg(Msg::Loaded {
        key: PageKey::Population,
        data: PageData::Table(Table::from_rows(&rows)),
        freshness: fresh(),
    });
}

fn render(app: &mut App) -> String {
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
    terminal.draw(|frame| ui::draw(frame, app, now())).unwrap();
    terminal.backend().to_string()
}

#[test]
fn countries_table() {
    let mut app = app(Tab::Countries);
    with_population(&mut app);
    press(&mut app, KeyCode::Char('j'));
    insta::assert_snapshot!(render(&mut app));
}

#[test]
fn countries_table_sorted_and_searched() {
    let mut app = app(Tab::Countries);
    with_population(&mut app);
    press(&mut app, KeyCode::Char('l')); // sort by name, ascending
    press(&mut app, KeyCode::Char('/'));
    for c in "an".chars() {
        press(&mut app, KeyCode::Char(c));
    }
    press(&mut app, KeyCode::Enter);
    insta::assert_snapshot!(render(&mut app));
}

#[test]
fn country_detail() {
    let mut app = app(Tab::Countries);
    with_population(&mut app);
    press(&mut app, KeyCode::Char('/'));
    for c in "georgia".chars() {
        press(&mut app, KeyCode::Char(c));
    }
    press(&mut app, KeyCode::Enter);
    press(&mut app, KeyCode::Enter);
    let url = app.detail.as_ref().expect("detail opened").url.clone();
    let detail = country::parse_country(include_str!("fixtures/country_georgia.html")).unwrap();
    app.on_msg(Msg::Loaded {
        key: PageKey::Country(url),
        data: PageData::Country(detail),
        freshness: fresh(),
    });
    insta::assert_snapshot!(render(&mut app));
}

#[test]
fn live_dashboard() {
    let mut app = app(Tab::Live);
    let constants =
        counters::parse_energy_constants(include_str!("fixtures/rts_live_counter_v2.js")).unwrap();
    app.on_msg(Msg::Loaded {
        key: PageKey::Counters,
        data: PageData::Energy(constants),
        freshness: fresh(),
    });
    insta::assert_snapshot!(render(&mut app));
}

#[test]
fn offline_error_screen() {
    let mut app = app(Tab::Gdp);
    app.on_msg(Msg::Failed {
        key: PageKey::Gdp,
        error: "network: error sending request".into(),
    });
    insta::assert_snapshot!(render(&mut app));
}

#[test]
fn help_popup() {
    let mut app = app(Tab::Countries);
    with_population(&mut app);
    press(&mut app, KeyCode::Char('?'));
    insta::assert_snapshot!(render(&mut app));
}
