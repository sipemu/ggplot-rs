//! Label formatting functions for scale breaks.
//! Analogous to R's `scales` package (comma, percent, dollar, scientific).

/// Format with comma separators for thousands (e.g., 1,234,567).
pub fn label_comma(v: f64) -> String {
    if v == v.round() && v.abs() < 1e15 {
        let s = format!("{}", v as i64);
        add_commas(&s)
    } else {
        let s = format!("{:.2}", v);
        let s = s.trim_end_matches('0').trim_end_matches('.');
        if let Some((int_part, dec_part)) = s.split_once('.') {
            format!("{}.{}", add_commas(int_part), dec_part)
        } else {
            add_commas(s)
        }
    }
}

/// Format as percentage (e.g., 0.5 → "50%").
pub fn label_percent(v: f64) -> String {
    let pct = v * 100.0;
    if (pct - pct.round()).abs() < 1e-10 {
        format!("{}%", pct.round() as i64)
    } else {
        format!("{:.1}%", pct)
    }
}

/// Format as US dollar (e.g., 1234.5 → "$1,235").
pub fn label_dollar(v: f64) -> String {
    if v < 0.0 {
        format!("-${}", label_comma(-v))
    } else {
        format!("${}", label_comma(v))
    }
}

/// Format in scientific notation (e.g., 12345 → "1.23e4").
pub fn label_scientific(v: f64) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    let exp = v.abs().log10().floor() as i32;
    let mantissa = v / 10f64.powi(exp);
    if (mantissa - mantissa.round()).abs() < 1e-10 {
        format!("{}e{}", mantissa.round() as i64, exp)
    } else {
        let s = format!("{:.2}e{}", mantissa, exp);
        // Trim trailing zeros in mantissa
        if let Some((m, e)) = s.split_once('e') {
            let m = m.trim_end_matches('0').trim_end_matches('.');
            format!("{m}e{e}")
        } else {
            s
        }
    }
}

fn add_commas(s: &str) -> String {
    let negative = s.starts_with('-');
    let digits = if negative { &s[1..] } else { s };
    let mut result = String::new();
    for (i, ch) in digits.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    let formatted: String = result.chars().rev().collect();
    if negative {
        format!("-{formatted}")
    } else {
        formatted
    }
}

/// A label formatter function type.
pub type LabelFormatter = fn(f64) -> String;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_label_comma() {
        assert_eq!(label_comma(1000.0), "1,000");
        assert_eq!(label_comma(1234567.0), "1,234,567");
        assert_eq!(label_comma(42.0), "42");
        assert_eq!(label_comma(-5000.0), "-5,000");
    }

    #[test]
    fn test_label_percent() {
        assert_eq!(label_percent(0.5), "50%");
        assert_eq!(label_percent(0.0), "0%");
        assert_eq!(label_percent(1.0), "100%");
        assert_eq!(label_percent(0.123), "12.3%");
    }

    #[test]
    fn test_label_dollar() {
        assert_eq!(label_dollar(1000.0), "$1,000");
        assert_eq!(label_dollar(0.0), "$0");
        assert_eq!(label_dollar(-500.0), "-$500");
    }

    #[test]
    fn test_label_scientific() {
        assert_eq!(label_scientific(12345.0), "1.23e4");
        assert_eq!(label_scientific(0.0), "0");
        assert_eq!(label_scientific(100.0), "1e2");
    }
}
