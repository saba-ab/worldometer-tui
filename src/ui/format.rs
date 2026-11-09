//! Number formatting: `1,234,567`, `+0.91 %`, `$1.23 T`, `+4.2/s`, `2h`.

/// U+2212 MINUS SIGN, as used by the site.
pub const MINUS: char = '\u{2212}';
/// Placeholder for missing values.
pub const MISSING: &str = "—";

/// Integer with thousands separators: `-3225184` → `"−3,225,184"`.
pub fn thousands(value: i128) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    if value < 0 {
        out.push(MINUS);
    }
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Rounded to an integer with thousands separators.
pub fn int(value: f64) -> String {
    thousands(value.round() as i128)
}

/// Fixed decimals; the integer part gets thousands separators.
pub fn float(value: f64, decimals: usize) -> String {
    if decimals == 0 {
        return int(value);
    }
    let text = format!("{:.*}", decimals, value.abs());
    let (whole, frac) = text.split_once('.').unwrap_or((&text, ""));
    let whole: i128 = whole.parse().unwrap_or(0);
    let sign = if value < 0.0 && text.chars().any(|c| c.is_ascii_digit() && c != '0') {
        MINUS.to_string()
    } else {
        String::new()
    };
    format!("{sign}{}.{frac}", thousands(whole))
}

/// Signed percentage: `+0.89 %`, `−0.23 %`.
pub fn signed_pct(value: f64) -> String {
    let body = float(value.abs(), 2);
    let sign = if value > 0.0 && body != "0.00" {
        "+".to_string()
    } else if value < 0.0 && body != "0.00" {
        MINUS.to_string()
    } else {
        String::new()
    };
    format!("{sign}{body} %")
}

/// Unsigned percentage: `17.80 %`.
pub fn pct(value: f64) -> String {
    format!("{} %", float(value, 2))
}

/// Whole dollars: `$95,212`.
pub fn usd(value: f64) -> String {
    let text = int(value.abs());
    if value < 0.0 {
        format!("{MINUS}${text}")
    } else {
        format!("${text}")
    }
}

/// Compact dollars: `$32.64 T`, `$1.23 B`, `$456.70 M`.
pub fn usd_compact(value: f64) -> String {
    const UNITS: [(f64, &str); 3] = [(1e12, "T"), (1e9, "B"), (1e6, "M")];
    for (scale, suffix) in UNITS {
        if value.abs() >= scale {
            return format!("${:.2} {suffix}", value / scale);
        }
    }
    usd(value)
}

/// Compact count for axis labels: `1.46 B`, `3.80 M`, `69.7 K`, `512`.
pub fn compact(value: f64) -> String {
    const UNITS: [(f64, &str); 3] = [(1e9, "B"), (1e6, "M"), (1e3, "K")];
    for (scale, suffix) in UNITS {
        if value.abs() >= scale {
            let scaled = value / scale;
            let decimals = if scaled.abs() >= 100.0 {
                0
            } else if scaled.abs() >= 10.0 {
                1
            } else {
                2
            };
            return format!("{} {suffix}", float(scaled, decimals));
        }
    }
    int(value)
}

/// A counter's rate in the most readable unit: `+4.2/s`, `+12/min`, `−1/day`.
pub fn rate(per_sec: f64) -> String {
    const UNITS: [(f64, &str); 4] = [(1.0, "s"), (60.0, "min"), (3600.0, "h"), (86400.0, "day")];
    let (scale, unit) = UNITS
        .iter()
        .copied()
        .find(|(scale, _)| (per_sec * scale).abs() >= 0.95)
        .unwrap_or(UNITS[UNITS.len() - 1]);
    let value = per_sec * scale;
    let decimals = if value.abs() >= 100.0 { 0 } else { 1 };
    let body = float(value.abs(), decimals);
    let sign = if value < 0.0 { MINUS } else { '+' };
    format!("{sign}{body}/{unit}")
}

/// Compact age: `just now`, `5m`, `2h`, `3d`.
pub fn age(seconds: i64) -> String {
    match seconds.max(0) {
        s if s < 60 => "just now".to_string(),
        s if s < 3600 => format!("{}m", s / 60),
        s if s < 86_400 => format!("{}h", s / 3600),
        s => format!("{}d", s / 86_400),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thousands_separators() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1_000), "1,000");
        assert_eq!(thousands(1_463_865_525), "1,463,865,525");
        assert_eq!(thousands(-3_225_184), "−3,225,184");
    }

    #[test]
    fn floats_and_percentages() {
        assert_eq!(float(1234.567, 1), "1,234.6");
        assert_eq!(float(-0.04, 1), "−0.0".replace('−', ""));
        assert_eq!(float(29.2, 1), "29.2");
        assert_eq!(signed_pct(0.89), "+0.89 %");
        assert_eq!(signed_pct(-0.23), "−0.23 %");
        assert_eq!(signed_pct(0.0), "0.00 %");
        assert_eq!(pct(17.8), "17.80 %");
    }

    #[test]
    fn dollars() {
        assert_eq!(usd(95_212.0), "$95,212");
        assert_eq!(usd_compact(32_642_373_000_000.0), "$32.64 T");
        assert_eq!(usd_compact(1_230_000_000.0), "$1.23 B");
        assert_eq!(usd_compact(45_000.0), "$45,000");
    }

    #[test]
    fn rates_pick_a_readable_unit() {
        assert_eq!(rate(4.18), "+4.2/s");
        assert_eq!(rate(0.5), "+30.0/min");
        assert_eq!(rate(-1.0 / 86_400.0), "−1.0/day");
        assert_eq!(rate(3600.0), "+3,600/s");
    }

    #[test]
    fn ages() {
        assert_eq!(age(5), "just now");
        assert_eq!(age(300), "5m");
        assert_eq!(age(7200), "2h");
        assert_eq!(age(3 * 86_400 + 5), "3d");
    }
}
