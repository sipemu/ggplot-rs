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

    /// Convert to a string for display/grouping purposes.
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

/// Format epoch seconds as a human-readable date/time string.
pub fn format_epoch_secs(secs: i64) -> String {
    // Simple UTC date/time formatting without external dependencies
    const SECS_PER_DAY: i64 = 86400;
    const SECS_PER_HOUR: i64 = 3600;
    const SECS_PER_MINUTE: i64 = 60;

    let (mut days, rem) = if secs >= 0 {
        (secs / SECS_PER_DAY, secs % SECS_PER_DAY)
    } else {
        let d = (secs - SECS_PER_DAY + 1) / SECS_PER_DAY;
        (d, secs - d * SECS_PER_DAY)
    };

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
