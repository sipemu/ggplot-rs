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

/// Format as US dollar with thousands separators (e.g., 1234 → "$1,234",
/// -500 → "-$500"). Equivalent to `label_currency("$", "", ",", None)`.
pub fn label_dollar(v: f64) -> String {
    currency(v, "$", "", ",", None)
}

/// Sign-aware currency formatter (R's `scales::label_currency`): the minus sign
/// goes *before* the prefix ("-€5", not "€-5"), thousands are grouped with
/// `big_mark` (e.g. `","`, `"."`, `" "`, `"'"`; `""` disables grouping), and
/// `accuracy` rounds to a multiple of it (`None` = up to two decimals, trailing
/// zeros trimmed). The decimal mark is always `.`.
///
/// ```
/// use ggplot_rs::scale::format::label_currency;
/// let eur = label_currency("€", "", ".", Some(1.0));
/// assert_eq!(eur(-1234567.0), "-€1.234.567");
/// let chf = label_currency("", " CHF", "'", Some(0.01));
/// assert_eq!(chf(9876.5), "9'876.50 CHF");
/// ```
pub fn label_currency(
    prefix: &str,
    suffix: &str,
    big_mark: &str,
    accuracy: Option<f64>,
) -> impl Fn(f64) -> String + Send + Sync {
    let prefix = prefix.to_string();
    let suffix = suffix.to_string();
    let big_mark = big_mark.to_string();
    move |v: f64| currency(v, &prefix, &suffix, &big_mark, accuracy)
}

/// Shared body of [`label_currency`] / [`label_dollar`].
fn currency(v: f64, prefix: &str, suffix: &str, big_mark: &str, accuracy: Option<f64>) -> String {
    // Format the magnitude, then put the sign in front of the prefix. NaN and
    // ±inf pass through as "NaN" / "inf".
    let (_, body) = split_sign(format_accuracy(v.abs(), accuracy));
    let sign = if v < 0.0 && !is_zero_number(&body) {
        "-"
    } else {
        ""
    };
    let body = if big_mark == "," {
        body
    } else {
        body.replace(',', big_mark)
    };
    format!("{sign}{prefix}{body}{suffix}")
}

/// Split a leading `-` off a formatted number.
fn split_sign(s: String) -> (&'static str, String) {
    match s.strip_prefix('-') {
        Some(rest) => ("-", rest.to_string()),
        None => ("", s),
    }
}

/// True when a formatted number is (rounded to) zero, e.g. "0", "0.00".
fn is_zero_number(s: &str) -> bool {
    s.chars().all(|c| matches!(c, '0' | '.' | ','))
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
            let s = format!("{rounded:.decimals$}");
            // Group only the integer part ("1234.50" → "1,234.50").
            match s.split_once('.') {
                Some((int_part, dec_part)) => format!("{}.{dec_part}", add_commas(int_part)),
                None => add_commas(&s),
            }
        }
        _ => label_comma(v),
    }
}

/// General configurable number formatter (R's `scales::label_number`).
/// Multiplies by `scale`, rounds to `accuracy` (None = trim), groups thousands
/// with `,`, and wraps in `prefix`/`suffix` — with the minus sign placed before
/// the prefix (`-€5`). For a custom thousands mark use [`label_currency`].
pub fn label_number(
    accuracy: Option<f64>,
    prefix: &str,
    suffix: &str,
    scale: f64,
) -> impl Fn(f64) -> String + Send + Sync {
    let prefix = prefix.to_string();
    let suffix = suffix.to_string();
    move |v: f64| {
        // Sign-aware: "-€5", never "€-5"; a value that rounds to zero loses
        // its sign ("€0", not "-€0").
        let (sign, body) = split_sign(format_accuracy(v * scale, accuracy));
        let sign = if is_zero_number(&body) { "" } else { sign };
        format!("{sign}{prefix}{body}{suffix}")
    }
}

// ─── Locale-aware variants ─────────────────────────────────────────────
//
// Each `*_marks` formatter takes an explicit decimal mark and thousands mark;
// each `*_locale` formatter takes them (plus the percent / currency
// conventions) from a [`Locale`]. With `"."` / `","` they reproduce the plain
// formatters exactly.

use crate::locale::{localize_number, Locale};

/// Shared body of the `label_number*` formatters: scale, round to `accuracy`,
/// group, swap marks, then wrap in `prefix`/`suffix` with the sign in front.
#[allow(clippy::too_many_arguments)]
fn number_body(
    v: f64,
    accuracy: Option<f64>,
    prefix: &str,
    suffix: &str,
    scale: f64,
    decimal_mark: &str,
    big_mark: &str,
    min_grouping_digits: usize,
) -> String {
    let (sign, body) = split_sign(format_accuracy(v * scale, accuracy));
    let sign = if is_zero_number(&body) { "" } else { sign };
    let body = localize_number(&body, decimal_mark, big_mark, min_grouping_digits);
    format!("{sign}{prefix}{body}{suffix}")
}

/// [`label_number`] with an explicit decimal mark and thousands mark
/// (`""` disables grouping).
///
/// ```
/// use ggplot_rs::scale::format::label_number_marks;
/// let f = label_number_marks(Some(0.01), "", " kg", 1.0, ",", ".");
/// assert_eq!(f(-1234.5), "-1.234,50 kg");
/// ```
pub fn label_number_marks(
    accuracy: Option<f64>,
    prefix: &str,
    suffix: &str,
    scale: f64,
    decimal_mark: &str,
    big_mark: &str,
) -> impl Fn(f64) -> String + Send + Sync {
    let prefix = prefix.to_string();
    let suffix = suffix.to_string();
    let dm = decimal_mark.to_string();
    let bm = big_mark.to_string();
    move |v: f64| number_body(v, accuracy, &prefix, &suffix, scale, &dm, &bm, 1)
}

/// [`label_number`] (scale 1) with the marks of `locale`.
///
/// ```
/// use ggplot_rs::locale::Locale;
/// use ggplot_rs::scale::format::label_number_locale;
/// let f = label_number_locale(&Locale::de_de(), Some(0.1), "", " kg");
/// assert_eq!(f(1234.56), "1.234,6 kg");
/// ```
pub fn label_number_locale(
    locale: &Locale,
    accuracy: Option<f64>,
    prefix: &str,
    suffix: &str,
) -> impl Fn(f64) -> String + Send + Sync {
    let prefix = prefix.to_string();
    let suffix = suffix.to_string();
    let loc = locale.clone();
    move |v: f64| {
        number_body(
            v,
            accuracy,
            &prefix,
            &suffix,
            1.0,
            &loc.decimal_mark,
            &loc.big_mark,
            loc.min_grouping_digits,
        )
    }
}

/// [`label_comma`] with explicit marks: `label_comma_marks(",", ".")(1234567.5)`
/// is `"1.234.567,5"`.
pub fn label_comma_marks(
    decimal_mark: &str,
    big_mark: &str,
) -> impl Fn(f64) -> String + Send + Sync {
    let dm = decimal_mark.to_string();
    let bm = big_mark.to_string();
    move |v: f64| localize_number(&label_comma(v), &dm, &bm, 1)
}

/// [`label_comma`] with the marks of `locale` (de-DE: `1.234.567`).
pub fn label_comma_locale(locale: &Locale) -> impl Fn(f64) -> String + Send + Sync {
    let loc = locale.clone();
    move |v: f64| loc.localize_number(&label_comma(v))
}

/// Shared body of the percent formatters. `accuracy` (in percentage points,
/// e.g. `0.1`) rounds; `None` behaves like [`label_percent`] (whole numbers
/// verbatim, otherwise one decimal).
fn percent_body(
    v: f64,
    accuracy: Option<f64>,
    decimal_mark: &str,
    big_mark: &str,
    min_grouping_digits: usize,
    suffix: &str,
) -> String {
    match accuracy {
        Some(_) => number_body(
            v,
            accuracy,
            "",
            suffix,
            100.0,
            decimal_mark,
            big_mark,
            min_grouping_digits,
        ),
        None => {
            let pct = v * 100.0;
            let body = if (pct - pct.round()).abs() < 1e-10 {
                format!("{}", pct.round() as i64)
            } else {
                format!("{pct:.1}")
            };
            let body = localize_number(&body, decimal_mark, big_mark, min_grouping_digits);
            format!("{body}{suffix}")
        }
    }
}

/// [`label_percent`] with explicit marks and percent suffix (`"%"`, `" %"`).
/// `accuracy` is in percentage points (`Some(0.1)` → one decimal); `None`
/// matches [`label_percent`].
///
/// ```
/// use ggplot_rs::scale::format::label_percent_marks;
/// let f = label_percent_marks(None, ",", ".", " %");
/// assert_eq!(f(0.123), "12,3 %");
/// ```
pub fn label_percent_marks(
    accuracy: Option<f64>,
    decimal_mark: &str,
    big_mark: &str,
    suffix: &str,
) -> impl Fn(f64) -> String + Send + Sync {
    let dm = decimal_mark.to_string();
    let bm = big_mark.to_string();
    let suffix = suffix.to_string();
    move |v: f64| percent_body(v, accuracy, &dm, &bm, 1, &suffix)
}

/// [`label_percent`] in the conventions of `locale`: de-DE / fr-FR `"12,3 %"`,
/// en-US / en-GB `"12.3%"`. `accuracy` as in [`label_percent_marks`].
pub fn label_percent_locale(
    locale: &Locale,
    accuracy: Option<f64>,
) -> impl Fn(f64) -> String + Send + Sync {
    let loc = locale.clone();
    move |v: f64| {
        percent_body(
            v,
            accuracy,
            &loc.decimal_mark,
            &loc.big_mark,
            loc.min_grouping_digits,
            &loc.percent_suffix,
        )
    }
}

/// [`label_currency`] with an explicit decimal mark as well.
///
/// ```
/// use ggplot_rs::scale::format::label_currency_marks;
/// let eur = label_currency_marks("", " €", ".", ",", Some(0.01));
/// assert_eq!(eur(-1234.5), "-1.234,50 €");
/// ```
pub fn label_currency_marks(
    prefix: &str,
    suffix: &str,
    big_mark: &str,
    decimal_mark: &str,
    accuracy: Option<f64>,
) -> impl Fn(f64) -> String + Send + Sync {
    let prefix = prefix.to_string();
    let suffix = suffix.to_string();
    let bm = big_mark.to_string();
    let dm = decimal_mark.to_string();
    move |v: f64| currency_marks(v, &prefix, &suffix, &bm, &dm, 1, accuracy)
}

/// Currency in the conventions of `locale` — its symbol, placement and marks:
/// de-DE `"1.234,50 €"`, en-US `"$1,234.50"`, en-GB `"£1,234.50"`, fr-FR
/// `"1 234,50 €"`. Use [`Locale::with_currency`] for another symbol.
pub fn label_currency_locale(
    locale: &Locale,
    accuracy: Option<f64>,
) -> impl Fn(f64) -> String + Send + Sync {
    let (prefix, suffix) = locale.currency_affixes(&locale.currency_symbol);
    let loc = locale.clone();
    move |v: f64| {
        currency_marks(
            v,
            &prefix,
            &suffix,
            &loc.big_mark,
            &loc.decimal_mark,
            loc.min_grouping_digits,
            accuracy,
        )
    }
}

/// Sign-aware currency body with both marks (see [`currency`]).
fn currency_marks(
    v: f64,
    prefix: &str,
    suffix: &str,
    big_mark: &str,
    decimal_mark: &str,
    min_grouping_digits: usize,
    accuracy: Option<f64>,
) -> String {
    let (_, body) = split_sign(format_accuracy(v.abs(), accuracy));
    let sign = if v < 0.0 && !is_zero_number(&body) {
        "-"
    } else {
        ""
    };
    let body = localize_number(&body, decimal_mark, big_mark, min_grouping_digits);
    format!("{sign}{prefix}{body}{suffix}")
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
    fn label_number_is_sign_aware() {
        let eur = label_number(None, "€", "", 1.0);
        assert_eq!(eur(-5.0), "-€5");
        assert_eq!(eur(-1234.5), "-€1,234.5");
        let f = label_number(Some(1.0), "€", "", 1.0);
        assert_eq!(f(-0.2), "€0", "rounds to zero: no sign");
        assert_eq!(f(1_234_567.0), "€1,234,567");
        // Thousands grouping never touches the decimals.
        let g = label_number(Some(0.01), "", "", 1.0);
        assert_eq!(g(1234.5), "1,234.50");
        assert_eq!(g(-98765.4321), "-98,765.43");
    }

    #[test]
    fn label_currency_marks_and_signs() {
        let eur = label_currency("€", "", ".", Some(1.0));
        assert_eq!(eur(-1_234_567.0), "-€1.234.567");
        assert_eq!(eur(0.0), "€0");
        let chf = label_currency("", " CHF", "'", Some(0.01));
        assert_eq!(chf(9876.5), "9'876.50 CHF");
        assert_eq!(chf(-0.001), "0.00 CHF");
        let plain = label_currency("$", "", "", None);
        assert_eq!(plain(1234.5), "$1234.5");
        let usd = label_currency("$", "", ",", None);
        for v in [0.0, 1.0, -500.0, 1234.5, -1_000_000.0, 0.125] {
            assert_eq!(usd(v), label_dollar(v), "label_dollar parity at {v}");
        }
        assert_eq!(label_dollar(-1234.0), "-$1,234");
        assert_eq!(usd(f64::NAN), "$NaN");
        assert_eq!(usd(f64::NEG_INFINITY), "-$inf");
    }

    #[test]
    fn marks_variants_match_plain_formatters_with_default_marks() {
        let num = label_number_marks(Some(0.01), "€", "", 1.0, ".", ",");
        let plain = label_number(Some(0.01), "€", "", 1.0);
        let comma = label_comma_marks(".", ",");
        let pct = label_percent_marks(None, ".", "", "%"); // label_percent never groups
        let cur = label_currency_marks("$", "", ",", ".", None);
        let us = Locale::en_us();
        let pct_us = label_percent_locale(&us, None);
        let comma_us = label_comma_locale(&us);
        for v in [
            0.0, 1.0, -1.0, 0.125, -0.004, 999.0, 1000.0, -1234.5, 98765.4321, 1e9, -2.5e12,
        ] {
            assert_eq!(num(v), plain(v), "label_number at {v}");
            assert_eq!(comma(v), label_comma(v), "label_comma at {v}");
            assert_eq!(comma_us(v), label_comma(v), "label_comma en-US at {v}");
            assert_eq!(pct(v), label_percent(v), "label_percent at {v}");
            assert_eq!(cur(v), label_dollar(v), "label_dollar at {v}");
            if v.abs() < 10.0 {
                // label_percent never groups; en-US groups from 1,000 %.
                assert_eq!(pct_us(v), label_percent(v), "en-US percent at {v}");
            }
        }
    }

    #[test]
    fn currency_per_locale() {
        let acc = Some(0.01);
        assert_eq!(
            label_currency_locale(&Locale::de_de(), acc)(1234.5),
            "1.234,50 €"
        );
        assert_eq!(
            label_currency_locale(&Locale::en_us(), acc)(1234.5),
            "$1,234.50"
        );
        assert_eq!(
            label_currency_locale(&Locale::en_gb(), acc)(1234.5),
            "£1,234.50"
        );
        assert_eq!(
            label_currency_locale(&Locale::fr_fr(), acc)(1234.5),
            "1 234,50 €"
        );
        // Negative: sign before the whole amount, also before a prefix.
        assert_eq!(
            label_currency_locale(&Locale::de_de(), acc)(-1234.5),
            "-1.234,50 €"
        );
        assert_eq!(
            label_currency_locale(&Locale::en_us(), acc)(-1234.5),
            "-$1,234.50"
        );
        // Rounds to zero → no sign.
        assert_eq!(
            label_currency_locale(&Locale::de_de(), acc)(-0.001),
            "0,00 €"
        );
        // Rounding and large numbers.
        let de0 = label_currency_locale(&Locale::de_de(), Some(1.0));
        assert_eq!(de0(1_234_567.5), "1.234.568 €");
        assert_eq!(de0(999.4), "999 €");
        let de_trim = label_currency_locale(&Locale::de_de(), None);
        assert_eq!(de_trim(1234.5), "1.234,5 €");
        assert_eq!(de_trim(2_000_000.0), "2.000.000 €");
        let chf = label_currency_locale(&Locale::de_de().with_currency("CHF"), acc);
        assert_eq!(chf(1234.5), "1.234,50 CHF");
    }

    #[test]
    fn numbers_per_locale() {
        let cases: [(Locale, [&str; 5]); 4] = [
            (
                Locale::en_us(),
                ["1,234.57", "-1,234.57", "0.10", "1,234,567,890.00", "0.00"],
            ),
            (
                Locale::en_gb(),
                ["1,234.57", "-1,234.57", "0.10", "1,234,567,890.00", "0.00"],
            ),
            (
                Locale::de_de(),
                ["1.234,57", "-1.234,57", "0,10", "1.234.567.890,00", "0,00"],
            ),
            (
                Locale::fr_fr(),
                ["1 234,57", "-1 234,57", "0,10", "1 234 567 890,00", "0,00"],
            ),
        ];
        for (loc, want) in cases {
            let f = label_number_locale(&loc, Some(0.01), "", "");
            let got = [
                f(1234.567),
                f(-1234.567),
                f(0.1),
                f(1_234_567_890.0),
                f(-0.004),
            ];
            assert_eq!(got.map(|s| s), want.map(String::from), "{}", loc.tag);
        }
        let de = Locale::de_de();
        assert_eq!(label_comma_locale(&de)(-1_234_567.0), "-1.234.567");
        assert_eq!(label_comma_locale(&de)(1234.567), "1.234,57");
        assert_eq!(label_comma_locale(&Locale::fr_fr())(1234.5), "1 234,5");
        let pre = label_number_locale(&de, None, "€", "");
        assert_eq!(pre(-5.0), "-€5");
    }

    #[test]
    fn percent_per_locale() {
        let de = label_percent_locale(&Locale::de_de(), None);
        assert_eq!(de(0.5), "50 %");
        assert_eq!(de(0.123), "12,3 %");
        assert_eq!(de(-0.25), "-25 %");
        assert_eq!(de(12.345), "1.234,5 %");
        let fr = label_percent_locale(&Locale::fr_fr(), Some(0.01));
        assert_eq!(fr(0.12345), "12,35 %");
        let us = label_percent_locale(&Locale::en_us(), None);
        assert_eq!(us(0.123), "12.3%");
        assert_eq!(
            label_percent_locale(&Locale::en_gb(), Some(1.0))(0.506),
            "51%"
        );
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
