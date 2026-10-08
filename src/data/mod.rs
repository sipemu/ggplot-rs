mod dataframe;
mod source;

pub use dataframe::DataFrame;
pub use source::GGData;

/// Dynamic value type for DataFrame columns.
#[derive(Clone, Debug)]
pub enum Value {
    Float(f64),
    Integer(i64),
    Str(String),
    Bool(bool),
    /// Seconds since Unix epoch (1970-01-01 00:00:00 UTC).
    DateTime(i64),
    Na,
}

impl Value {
    /// Try to extract as f64, coercing integers and datetimes.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Integer(i) => Some(*i as f64),
            Value::DateTime(secs) => Some(*secs as f64),
            _ => None,
        }
    }

    /// Try to extract as string representation.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Check if this is NA/missing.
    pub fn is_na(&self) -> bool {
        matches!(self, Value::Na)
    }

    /// Check if this is a DateTime value.
    pub fn is_datetime(&self) -> bool {
        matches!(self, Value::DateTime(_))
    }

    /// Create a DateTime from seconds since Unix epoch.
    pub fn from_timestamp(secs: i64) -> Self {
        Value::DateTime(secs)
    }

    /// Convert to a string for display and discrete-scale level purposes.
    ///
    /// Note this is *not* injective: `Value::Na` and the string `"NA"` (and
    /// `Float(1.0)`/`Integer(1)`/`Str("1")`) share a key. Use
    /// [`group_key`](Self::group_key) to split rows into groups, and
    /// [`key_str`](Self::key_str) for allocation-free lookups.
    pub fn to_group_key(&self) -> String {
        match self {
            Value::Float(f) => format!("{f}"),
            Value::Integer(i) => format!("{i}"),
            Value::Str(s) => s.clone(),
            Value::Bool(b) => format!("{b}"),
            Value::DateTime(secs) => format_epoch_secs(*secs),
            Value::Na => "NA".to_string(),
        }
    }
}

/// A grouping key for one value: [`Value::group_key`]. Unlike
/// [`Value::to_group_key`] it keeps a missing value (`Na`) distinct from the
/// literal string `"NA"`, and borrows string data instead of cloning it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GroupKey<'a> {
    /// A missing value.
    Na,
    /// Any present value, keyed by its display string (so `Float(1.0)` and
    /// `Integer(1)` still group together).
    Key(std::borrow::Cow<'a, str>),
}

impl Value {
    /// Same string as [`to_group_key`](Self::to_group_key), but borrowed for
    /// `Str` values — use it for hot-path scale lookups.
    pub fn key_str(&self) -> std::borrow::Cow<'_, str> {
        match self {
            Value::Str(s) => std::borrow::Cow::Borrowed(s.as_str()),
            Value::Na => std::borrow::Cow::Borrowed("NA"),
            other => std::borrow::Cow::Owned(other.to_group_key()),
        }
    }

    /// Injective-on-missingness grouping key: `Na` never collides with the
    /// string `"NA"`. Borrows `Str` data (no allocation).
    pub fn group_key(&self) -> GroupKey<'_> {
        match self {
            Value::Na => GroupKey::Na,
            other => GroupKey::Key(other.key_str()),
        }
    }
}

/// Format epoch seconds as a human-readable date/time string.
pub fn format_epoch_secs(secs: i64) -> String {
    // Simple UTC date/time formatting without external dependencies
    const SECS_PER_DAY: i64 = 86400;
    const SECS_PER_HOUR: i64 = 3600;
    const SECS_PER_MINUTE: i64 = 60;

    // Euclidean division: floor for negative timestamps, and total over the
    // whole i64 range (no `secs - 86399` overflow at i64::MIN).
    let mut days = secs.div_euclid(SECS_PER_DAY);
    let rem = secs.rem_euclid(SECS_PER_DAY);

    let hour = rem / SECS_PER_HOUR;
    let minute = (rem % SECS_PER_HOUR) / SECS_PER_MINUTE;
    let second = rem % SECS_PER_MINUTE;

    // Days since 1970-01-01 to Y-M-D (civil calendar)
    days += 719_468; // shift epoch from 1970-01-01 to 0000-03-01
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let doe = (days - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    if hour == 0 && minute == 0 && second == 0 {
        format!("{y:04}-{m:02}-{d:02}")
    } else {
        format!("{y:04}-{m:02}-{d:02} {hour:02}:{minute:02}:{second:02}")
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::DateTime(a), Value::DateTime(b)) => a == b,
            (Value::Na, Value::Na) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn na_and_literal_na_have_distinct_group_keys() {
        let na = Value::Na;
        let lit = Value::Str("NA".to_string());
        assert_ne!(na.group_key(), lit.group_key());
        // Display/level keys stay "NA" for both (what an axis or legend shows).
        assert_eq!(na.to_group_key(), "NA");
        assert_eq!(lit.to_group_key(), "NA");
        // Numeric types still group by value.
        assert_eq!(Value::Float(1.0).group_key(), Value::Integer(1).group_key());
        assert!(matches!(
            Value::Str("x".into()).key_str(),
            std::borrow::Cow::Borrowed("x")
        ));
    }

    #[test]
    fn format_epoch_secs_is_total() {
        assert_eq!(format_epoch_secs(0), "1970-01-01");
        assert_eq!(format_epoch_secs(-1), "1969-12-31 23:59:59");
        assert_eq!(format_epoch_secs(-86_400), "1969-12-31");
        assert_eq!(format_epoch_secs(951_782_400), "2000-02-29");
        // Extremes must not overflow (they used to panic in debug builds).
        let lo = format_epoch_secs(i64::MIN);
        let hi = format_epoch_secs(i64::MAX);
        assert!(lo.starts_with('-') && lo.contains(':'), "{lo}");
        assert!(hi.contains(':'), "{hi}");
    }
}
