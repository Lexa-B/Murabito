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
    let scale = 10f64.powi(digits as i32);
    let scaled = x.abs() * scale;
    if scaled.fract() == 0.5 {
        let up = (scaled.floor() + 1.0) / scale;
        let s = format!("{:.*}", digits, up);
        return if x < 0.0 { format!("-{s}") } else { s };
    }
    format!("{:.*}", digits, x)
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
    }

    #[test]
    fn a_row_carries_its_flag() {
        let r = FactRow::new("sea edges", "1  (SSE)", false);
        assert_eq!(
            (r.key.as_str(), r.value.as_str(), r.bad),
            ("sea edges", "1  (SSE)", false)
        );
    }
}
