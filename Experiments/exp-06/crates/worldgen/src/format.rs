//! Text the page prints: facts rows, and numbers formatted the way JavaScript formats them, so a row
//! is the same string on both pages and the harness can compare them literally.

/// One line of a step's facts: a key, a value, and whether the page colours it as a warning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FactRow {
    pub key: String,
    pub value: String,
    pub bad: bool,
}

impl FactRow {
    pub fn new(key: &str, value: impl Into<String>, bad: bool) -> FactRow {
        FactRow {
            key: key.to_string(),
            value: value.into(),
            bad,
        }
    }
}

/// JS `Number.prototype.toFixed`: the nearest decimal, a true half rounding away from zero. Rust's
/// `{:.n}` rounds a true half to even, so the two disagree on exactly representable halves such as
/// 0.125 → "0.13" against "0.12"; everything else is the same correctly rounded value.
pub fn js_fixed(x: f64, digits: usize) -> String {
    if x.is_nan() {
        return "NaN".to_string();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity" } else { "-Infinity" }.to_string();
    }
    // JS writes a sign only when x < 0, so a negative zero prints unsigned.
    let x = if x == 0.0 { 0.0 } else { x };
    if is_true_half(x.abs(), digits) {
        let scale = 10f64.powi(digits as i32);
        let up = ((x.abs() * scale).floor() + 1.0) / scale;
        let s = format!("{:.*}", digits, up);
        return if x < 0.0 { format!("-{s}") } else { s };
    }
    format!("{:.*}", digits, x)
}

/// Whether `x · 10^digits` is exactly a whole number and a half, read off the exact decimal
/// expansion rather than a product that rounds. A true half is a double `m / 2^(digits + 1)` with
/// `m` odd, whose expansion ends exactly at place `digits + 1`; any other double is at least an ulp
/// away and shows it well within the extra places.
fn is_true_half(x: f64, digits: usize) -> bool {
    const EXTRA: usize = 20;
    let wide = format!("{:.*}", digits + EXTRA, x);
    let tail = &wide[wide.len() - EXTRA..];
    tail.starts_with('5') && tail[1..].bytes().all(|b| b == b'0')
}

/// JS `Math.round`: the nearest integer, a half toward +∞ (so −2.5 → −2), as a float like JS's. Written
/// without `x + 0.5`, which rounds before the floor at 0.49999999999999994 and above 2^52.
pub fn js_round(x: f64) -> f64 {
    let f = x.floor();
    let r = if x - f >= 0.5 { f + 1.0 } else { f };
    if r == 0.0 && x < 0.0 {
        -0.0
    } else {
        r
    }
}

/// JS `Number.prototype.toLocaleString()` for an integer in the page's locale: thousands grouped by commas.
pub fn js_locale_int(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    if n < 0 {
        format!("-{grouped}")
    } else {
        grouped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_fixed_rounds_halves_away_from_zero_like_to_fixed() {
        assert_eq!(js_fixed(2.5, 0), "3");
        assert_eq!(js_fixed(-2.5, 0), "-3");
        assert_eq!(js_fixed(0.125, 2), "0.13"); // Rust's {:.2} would say 0.12
        assert_eq!(js_fixed(1.005, 2), "1.00"); // not a true half: 1.005 is below it in binary, as in JS
        assert_eq!(js_fixed(12.0, 1), "12.0");
        assert_eq!(js_fixed(269.4, 0), "269");
        assert_eq!(js_fixed(-0.04, 1), "-0.0"); // JS says "-0.0" too
        assert_eq!(js_fixed(10.235, 2), "10.23"); // below a half in binary; the old product test said 10.24
        assert_eq!(js_fixed(2.675, 2), "2.67"); // the same, the classic case
        assert_eq!(js_fixed(0.5, 0), "1");
        assert_eq!(js_fixed(1.5, 0), "2");
        assert_eq!(js_fixed(-0.125, 2), "-0.13");
        assert_eq!(js_fixed(-0.0, 1), "0.0");
        assert_eq!(js_fixed(f64::NAN, 2), "NaN");
        assert_eq!(js_fixed(f64::INFINITY, 0), "Infinity");
        assert_eq!(js_fixed(f64::NEG_INFINITY, 0), "-Infinity");
    }

    #[test]
    fn a_row_carries_its_flag() {
        let r = FactRow::new("sea edges", "1  (SSE)", false);
        assert_eq!(
            (r.key.as_str(), r.value.as_str(), r.bad),
            ("sea edges", "1  (SSE)", false)
        );
    }

    #[test]
    fn js_round_rounds_a_half_toward_plus_infinity() {
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(js_round(-2.5), -2.0); // Math.round(-2.5) is -2
        assert_eq!(js_round(16200.4), 16200.0);
        assert!(
            js_round(-0.4).is_sign_negative() && js_round(-0.4) == 0.0,
            "Math.round(-0.4) is -0"
        );
        assert_eq!(js_round(0.49999999999999994), 0.0);
        assert_eq!(js_round(4503599627370497.0), 4503599627370497.0);
    }

    #[test]
    fn js_locale_int_groups_thousands_with_commas() {
        assert_eq!(js_locale_int(16200), "16,200");
        assert_eq!(js_locale_int(15139), "15,139");
        assert_eq!(js_locale_int(1296), "1,296");
        assert_eq!(js_locale_int(999), "999");
        assert_eq!(js_locale_int(0), "0");
        assert_eq!(js_locale_int(-1234567), "-1,234,567");
    }
}
