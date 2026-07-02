//! Label formatting functions for scale breaks.
//! Analogous to R's `scales` package (comma, percent, dollar, scientific,
//! number, SI, ordinal, bytes).

use std::sync::Arc;

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

/// A label formatter — any `Fn(f64) -> String`. The plain `label_*` functions
/// coerce into this, and the configurable `label_number`/`label_si`/… builders
/// return one directly.
pub type LabelFormatter = Arc<dyn Fn(f64) -> String + Send + Sync>;

/// Round `v` to a multiple of `accuracy` and format with the implied decimals.
fn format_accuracy(v: f64, accuracy: Option<f64>) -> String {
    match accuracy {
        Some(acc) if acc > 0.0 => {
            let rounded = (v / acc).round() * acc;
            let decimals = (-acc.log10().floor()).max(0.0) as usize;
            add_commas(&format!("{rounded:.decimals$}"))
        }
        _ => label_comma(v),
    }
}

/// General configurable number formatter (R's `scales::label_number`).
/// Multiplies by `scale`, rounds to `accuracy` (None = trim), and wraps in
/// `prefix`/`suffix`.
pub fn label_number(
    accuracy: Option<f64>,
    prefix: &str,
    suffix: &str,
    scale: f64,
) -> impl Fn(f64) -> String + Send + Sync {
    let prefix = prefix.to_string();
    let suffix = suffix.to_string();
    move |v: f64| format!("{prefix}{}{suffix}", format_accuracy(v * scale, accuracy))
}

/// SI-prefixed number formatter: 1_500 → "1.5k", 2.3e6 → "2.3M", 5e-4 → "500µ".
pub fn label_si() -> impl Fn(f64) -> String + Send + Sync {
    |v: f64| {
        if v == 0.0 {
            return "0".to_string();
        }
        let a = v.abs();
        let (div, suffix) = if a >= 1e12 {
            (1e12, "T")
        } else if a >= 1e9 {
            (1e9, "G")
        } else if a >= 1e6 {
            (1e6, "M")
        } else if a >= 1e3 {
            (1e3, "k")
        } else if a >= 1.0 {
            (1.0, "")
        } else if a >= 1e-3 {
            (1e-3, "m")
        } else if a >= 1e-6 {
            (1e-6, "µ")
        } else {
            (1e-9, "n")
        };
        let scaled = v / div;
        let s = format!("{scaled:.1}");
        let s = s.trim_end_matches('0').trim_end_matches('.');
        format!("{s}{suffix}")
    }
}

/// Ordinal formatter: 1 → "1st", 2 → "2nd", 3 → "3rd", 11 → "11th".
pub fn label_ordinal() -> impl Fn(f64) -> String + Send + Sync {
    |v: f64| {
        let n = v.round() as i64;
        let suffix = match (n.rem_euclid(10), n.rem_euclid(100)) {
            (1, r) if r != 11 => "st",
            (2, r) if r != 12 => "nd",
            (3, r) if r != 13 => "rd",
            _ => "th",
        };
        format!("{n}{suffix}")
    }
}

/// Byte-size formatter. `binary = true` uses 1024-based KiB/MiB; otherwise
/// 1000-based kB/MB.
pub fn label_bytes(binary: bool) -> impl Fn(f64) -> String + Send + Sync {
    let (base, units): (f64, &[&str]) = if binary {
        (1024.0, &["B", "KiB", "MiB", "GiB", "TiB"])
    } else {
        (1000.0, &["B", "kB", "MB", "GB", "TB"])
    };
    move |v: f64| {
        let a = v.abs();
        if a < base {
            return format!("{} {}", v.round() as i64, units[0]);
        }
        let mut val = a;
        let mut i = 0;
        while val >= base && i < units.len() - 1 {
            val /= base;
            i += 1;
        }
        let s = format!("{val:.1}");
        let s = s.trim_end_matches('0').trim_end_matches('.');
        let sign = if v < 0.0 { "-" } else { "" };
        format!("{sign}{s} {}", units[i])
    }
}

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

    #[test]
    fn test_label_si() {
        let f = label_si();
        assert_eq!(f(1500.0), "1.5k");
        assert_eq!(f(2_300_000.0), "2.3M");
        assert_eq!(f(5e9), "5G");
        assert_eq!(f(0.0), "0");
        assert_eq!(f(0.0005), "500µ");
        assert_eq!(f(-4000.0), "-4k");
    }

    #[test]
    fn test_label_number() {
        let f = label_number(Some(0.1), "", " kg", 1.0);
        assert_eq!(f(4.16), "4.2 kg");
        let pct = label_number(Some(1.0), "", "%", 100.0);
        assert_eq!(pct(0.25), "25%");
        let money = label_number(None, "€", "", 1.0);
        assert_eq!(money(1500.0), "€1,500");
    }

    #[test]
    fn test_label_ordinal() {
        let f = label_ordinal();
        assert_eq!(f(1.0), "1st");
        assert_eq!(f(2.0), "2nd");
        assert_eq!(f(3.0), "3rd");
        assert_eq!(f(4.0), "4th");
        assert_eq!(f(11.0), "11th");
        assert_eq!(f(22.0), "22nd");
    }

    #[test]
    fn test_label_bytes() {
        let f = label_bytes(false);
        assert_eq!(f(500.0), "500 B");
        assert_eq!(f(1500.0), "1.5 kB");
        assert_eq!(f(2_000_000.0), "2 MB");
        let b = label_bytes(true);
        assert_eq!(b(1024.0), "1 KiB");
    }
}
