use worldo_tui::parse::tables::{parse_area, parse_co2, parse_gdp};

#[test]
fn parses_gdp() {
    let rows = parse_gdp(include_str!("fixtures/gdp_by_country.html")).expect("fixture parses");
    assert!(rows.len() > 150, "only {} rows", rows.len());
    let us = &rows[0];
    assert_eq!(us.name, "United States");
    assert_eq!(us.gdp_usd, 32_642_373_000_000.0);
    assert_eq!(us.growth_pct, Some(2.33));
    assert_eq!(us.per_capita_usd, Some(95_212.0));
}

#[test]
fn parses_co2() {
    let rows = parse_co2(include_str!("fixtures/co2_by_country.html")).expect("fixture parses");
    assert!(rows.len() > 150, "only {} rows", rows.len());
    let china = &rows[0];
    assert_eq!(china.name, "China");
    assert_eq!(china.emissions_tons, 13_124_727_993);
    assert_eq!(china.yearly_change_pct, Some(0.79));
    assert_eq!(china.per_capita_tons, Some(9.13));
    assert_eq!(china.world_share_pct, Some(33.1));
}

#[test]
fn parses_area() {
    let rows = parse_area(include_str!("fixtures/largest_countries.html")).expect("fixture parses");
    assert!(rows.len() > 200, "only {} rows", rows.len());
    let russia = &rows[0];
    assert_eq!(russia.name, "Russia");
    assert_eq!(russia.total_area_km2, 17_098_242);
    // km², not mi²: the mi² column comes first in the header list after "Tot. Area (km²)".
    assert_eq!(russia.land_area_km2, Some(16_377_742));
    assert_eq!(russia.world_share_pct, Some(12.4));
}
