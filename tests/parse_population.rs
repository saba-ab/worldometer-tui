use worldo_tui::parse::ParseError;
use worldo_tui::parse::population::parse_population;

const FIXTURE: &str = include_str!("fixtures/population_by_country.html");

#[test]
fn parses_the_real_table() {
    let rows = parse_population(FIXTURE).expect("fixture parses");
    assert!(rows.len() > 200, "only {} rows", rows.len());

    let india = rows
        .iter()
        .find(|r| r.name == "India")
        .expect("India present");
    assert_eq!(india.rank, 1);
    assert_eq!(india.population, 1_476_625_576);
    assert_eq!(india.yearly_change_pct, Some(0.87));
    assert_eq!(india.net_change, Some(12_760_051));
    assert_eq!(india.migrants_net, Some(-440_456));
    assert_eq!(india.median_age, Some(29.2));
    // The urban % cell carries a footnote marker that must be ignored.
    assert_eq!(india.urban_pop_pct, Some(36.0));
    assert_eq!(india.world_share_pct, Some(17.8));
    assert_eq!(
        india.detail_url.as_deref(),
        Some("https://www.worldometers.info/world-population/india-population/")
    );

    assert!(rows.iter().any(|r| r.name == "China"));
    let georgia = rows
        .iter()
        .find(|r| r.name == "Georgia")
        .expect("Georgia present");
    assert_eq!(georgia.yearly_change_pct, Some(-0.05));
    assert_eq!(georgia.land_area_km2, Some(69_700));
}

#[test]
fn ranks_are_sequential() {
    let rows = parse_population(FIXTURE).unwrap();
    for (i, row) in rows.iter().enumerate() {
        assert_eq!(row.rank as usize, i + 1, "row {}", row.name);
    }
}

#[test]
fn missing_population_column_is_reported() {
    let doctored = FIXTURE.replace("<span>Population 2026</span>", "<span>People 2026</span>");
    assert_eq!(
        parse_population(&doctored),
        Err(ParseError::MissingColumn("population"))
    );
}

#[test]
fn missing_table_is_reported() {
    assert_eq!(
        parse_population("<html><body><p>maintenance</p></body></html>"),
        Err(ParseError::TableNotFound("yearly change"))
    );
}
