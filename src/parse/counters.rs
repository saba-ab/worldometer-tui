//! Energy-counter constants (Option A in `docs/plan.md` §6).
//!
//! The homepage loads a small, readable ES module
//! (`rts-live-counter-v2.<hash>.js`) that holds plain constants such as
//! ``oil:{reserves:{value:{$exact:`1765151568000`},unit:`barrel`,year:2025}``.
//! We locate that script from the homepage and read the constants. The
//! obfuscated legacy counter script is deliberately not used (see DISCOVERY.md).

use std::sync::LazyLock;

use scraper::{Html, Selector};

use super::{ParseError, absolute_url};
use crate::model::{EnergyConstants, Reserve};

static SCRIPT: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("script[src]").expect("constant selector is valid"));

/// Substring identifying the energy-counter module in a `<script src>`.
const SCRIPT_MARKER: &str = "rts-live-counter-v2";

/// Finds the absolute URL of the energy-counter module in the homepage HTML.
pub fn find_counter_script_url(home_html: &str) -> Result<String, ParseError> {
    let doc = Html::parse_document(home_html);
    doc.select(&SCRIPT)
        .filter_map(|s| s.value().attr("src"))
        .filter(|src| src.contains(SCRIPT_MARKER))
        .find_map(absolute_url)
        .ok_or(ParseError::ScriptNotFound)
}

pub fn parse_energy_constants(js: &str) -> Result<EnergyConstants, ParseError> {
    Ok(EnergyConstants {
        oil: reserve(js, "oil:{", "crudeProductionAnnual:{", "oil")?,
        gas: reserve(js, "gas:{", "productionAnnual:{", "gas")?,
        coal: reserve(js, "coal:{", "productionAnnual:{", "coal")?,
        energy_total_ej_per_year: exact_number(js, &["energy:{", "totalAnnual:{"])
            .ok_or(ParseError::ConstantNotFound("energy.totalAnnual"))?,
        low_carbon_share: exact_number(js, &["energy:{", "lowCarbonShare:{"])
            .filter(|share| (0.0..=1.0).contains(share))
            .ok_or(ParseError::ConstantNotFound("energy.lowCarbonShare"))?,
    })
}

fn reserve(
    js: &str,
    section: &str,
    production_key: &str,
    name: &'static str,
) -> Result<Reserve, ParseError> {
    let reserves_pos =
        seek(js, 0, &[section, "reserves:{"]).ok_or(ParseError::ConstantNotFound(name))?;
    let reserves = exact_at(js, reserves_pos)
        .and_then(parse_exact)
        .ok_or(ParseError::ConstantNotFound(name))?;
    let reserves_year = year_at(js, reserves_pos).ok_or(ParseError::ConstantNotFound(name))?;
    let production_per_year = exact_number(js, &[section, production_key])
        .filter(|p| *p > 0.0)
        .ok_or(ParseError::ConstantNotFound(name))?;
    Ok(Reserve {
        reserves,
        reserves_year,
        production_per_year,
    })
}

/// Advances through `keys` in order, returning the position right after the last.
fn seek(js: &str, from: usize, keys: &[&str]) -> Option<usize> {
    keys.iter().try_fold(from, |pos, key| {
        js.get(pos..)?.find(key).map(|i| pos + i + key.len())
    })
}

/// The next ``$exact:`…` `` literal at or after `pos`.
fn exact_at(js: &str, pos: usize) -> Option<&str> {
    let start = seek(js, pos, &["$exact:`"])?;
    let len = js.get(start..)?.find('`')?;
    js.get(start..start + len)
}

fn exact_number(js: &str, keys: &[&str]) -> Option<f64> {
    let pos = seek(js, 0, keys)?;
    exact_at(js, pos).and_then(parse_exact)
}

/// Parses a decimal literal or a fraction `"a/b"`.
fn parse_exact(literal: &str) -> Option<f64> {
    let value = match literal.split_once('/') {
        Some((num, den)) => {
            let den: f64 = den.trim().parse().ok()?;
            if den == 0.0 {
                return None;
            }
            num.trim().parse::<f64>().ok()? / den
        }
        None => literal.trim().parse().ok()?,
    };
    value.is_finite().then_some(value)
}

/// `year:2025` following `pos`, sanity-checked.
fn year_at(js: &str, pos: usize) -> Option<i32> {
    let start = seek(js, pos, &["year:"])?;
    let digits: String = js
        .get(start..)?
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok().filter(|y| (1990..=2100).contains(y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_exact_handles_fractions() {
        assert_eq!(parse_exact("1/4"), Some(0.25));
        assert_eq!(parse_exact("12.5"), Some(12.5));
        assert_eq!(parse_exact("1/0"), None);
        assert_eq!(parse_exact("abc"), None);
    }

    #[test]
    fn missing_constant_is_an_error() {
        assert_eq!(
            parse_energy_constants("var n = t({});"),
            Err(ParseError::ConstantNotFound("oil"))
        );
    }
}
