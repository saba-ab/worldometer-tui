use worldo_tui::parse::ParseError;
use worldo_tui::parse::counters::{find_counter_script_url, parse_energy_constants};

#[test]
fn finds_the_energy_counter_script_on_the_homepage() {
    let url = find_counter_script_url(include_str!("fixtures/home.html")).expect("script found");
    assert!(url.starts_with("https://"));
    assert!(url.contains("rts-live-counter-v2"));
    assert!(url.ends_with(".js"));
}

#[test]
fn homepage_without_script_is_an_error() {
    assert_eq!(
        find_counter_script_url("<html><script src=\"/other.js\"></script></html>"),
        Err(ParseError::ScriptNotFound)
    );
}

#[test]
fn reads_energy_constants() {
    let c = parse_energy_constants(include_str!("fixtures/rts_live_counter_v2.js"))
        .expect("constants parse");
    assert_eq!(c.oil.reserves, 1_765_151_568_000.0);
    assert_eq!(c.oil.reserves_year, 2025);
    assert!((c.oil.production_per_year - 30_790_285_016.966_97).abs() < 1.0);
    assert_eq!(c.gas.reserves_year, 2024);
    assert_eq!(c.coal.reserves_year, 2023);
    assert!(c.coal.reserves > 1e12);
    assert!((c.energy_total_ej_per_year - 639.402_897_78).abs() < 1e-6);
    assert!(
        (c.low_carbon_share - 8_881_262_666_833_477.0 / 75_754_626_661_749_875.0).abs() < 1e-12
    );
}
