//! Public value and label formatting helpers.
//!
//! [`format_value`] renders a single data [`Value`] the way ggplot-rs shows it
//! in hover tooltips (rounded numbers, calendar dates for datetimes). The
//! `label_*` axis/legend formatters from [`crate::scale::format`] are
//! re-exported here so hosts can format numbers consistently with the plot.

use crate::data::Value;

pub use crate::locale::{CurrencyPosition, Locale};
pub use crate::scale::format::{
    label_bytes, label_comma, label_comma_locale, label_comma_marks, label_currency,
    label_currency_locale, label_currency_marks, label_dollar, label_number, label_number_locale,
    label_number_marks, label_ordinal, label_percent, label_percent_locale, label_percent_marks,
    label_scientific, label_si, LabelFormatter,
};

/// Format one data value for display (tooltips, data labels):
///
/// - strings as-is, booleans as `true`/`false`, `NA` as the empty string;
/// - numbers rounded to three decimals with trailing zeros dropped
///   (`3.14159` → `"3.142"`, `2.0` → `"2"`);
/// - datetimes (epoch seconds) as `YYYY-MM-DD[ HH:MM:SS]` UTC.
///
/// ```
/// use ggplot_rs::data::Value;
/// use ggplot_rs::format::format_value;
/// assert_eq!(format_value(&Value::Float(3.14159)), "3.142");
/// assert_eq!(format_value(&Value::DateTime(86_400)), "1970-01-02");
/// assert_eq!(format_value(&Value::Na), "");
/// ```
pub fn format_value(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Na => String::new(),
        // A datetime axis reads as a calendar date, not raw epoch seconds.
        Value::DateTime(secs) => crate::data::format_epoch_secs(*secs),
        _ => v
            .as_f64()
            .map(|f| format!("{}", (f * 1000.0).round() / 1000.0))
            .unwrap_or_default(),
    }
}

/// [`format_value`] in the conventions of `locale`: numbers get its decimal
/// and thousands marks, datetimes its date format and month names.
///
/// ```
/// use ggplot_rs::data::Value;
/// use ggplot_rs::format::{format_value_locale, Locale};
/// let de = Locale::de_de();
/// assert_eq!(format_value_locale(&Value::Float(1234.56789), &de), "1.234,568");
/// assert_eq!(format_value_locale(&Value::DateTime(1_792_022_400), &de), "15. Okt. 2026");
/// ```
pub fn format_value_locale(v: &Value, locale: &Locale) -> String {
    match v {
        Value::DateTime(secs) => locale.format_datetime(*secs),
        Value::Str(_) | Value::Bool(_) | Value::Na => format_value(v),
        _ => locale.localize_number(&format_value(v)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_each_variant() {
        assert_eq!(format_value(&Value::Str("a<b".into())), "a<b");
        assert_eq!(format_value(&Value::Bool(true)), "true");
        assert_eq!(format_value(&Value::Integer(-42)), "-42");
        assert_eq!(format_value(&Value::Float(2.0)), "2");
        assert_eq!(format_value(&Value::Float(0.12345)), "0.123");
        assert_eq!(format_value(&Value::DateTime(3_661)), "1970-01-01 01:01:01");
        assert_eq!(format_value(&Value::Na), "");
    }

    #[test]
    fn formats_each_variant_with_locale() {
        let de = Locale::de_de();
        assert_eq!(format_value_locale(&Value::Str("1.5".into()), &de), "1.5");
        assert_eq!(format_value_locale(&Value::Integer(-4200), &de), "-4.200");
        assert_eq!(format_value_locale(&Value::Float(0.5), &de), "0,5");
        assert_eq!(format_value_locale(&Value::Na, &de), "");
        assert_eq!(
            format_value_locale(&Value::DateTime(1_792_022_400), &Locale::en_us()),
            "Oct 15, 2026"
        );
        assert_eq!(
            format_value_locale(&Value::Float(1234.5), &Locale::en_us()),
            "1,234.5"
        );
    }
}
