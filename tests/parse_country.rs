use worldo_tui::parse::country::parse_country;

const FIXTURE: &str = include_str!("fixtures/country_georgia.html");

#[test]
fn parses_georgia() {
    let detail = parse_country(FIXTURE).expect("fixture parses");
    assert_eq!(detail.name, "Georgia");
    assert_eq!(detail.population, 3_804_642);
    assert_eq!(detail.rank, Some(131));
    assert_eq!(detail.world_share_pct, Some(0.05));

    assert_eq!(detail.history.len(), 20);
    let first = detail.history.first().unwrap();
    let last = detail.history.last().unwrap();
    assert_eq!(first.year, 1955);
    assert_eq!(first.population, 3_734_030);
    assert_eq!(last.year, 2026);
    assert_eq!(last.yearly_change_pct, Some(-0.05));
    assert_eq!(last.yearly_change, Some(-2_029));
    assert_eq!(last.density, Some(55.0));
    assert_eq!(last.urban_pop_pct, Some(61.3));
    assert_eq!(last.world_population, Some(8_300_678_396));
    assert!(detail.history.windows(2).all(|w| w[0].year < w[1].year));

    assert_eq!(detail.forecast.len(), 5);
    assert_eq!(detail.forecast.first().unwrap().year, 2030);
    assert_eq!(detail.forecast.last().unwrap().year, 2050);
}
