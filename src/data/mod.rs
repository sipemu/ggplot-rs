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
    Na,
}

impl Value {
    /// Try to extract as f64, coercing integers.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Integer(i) => Some(*i as f64),
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

    /// Convert to a string for display/grouping purposes.
    pub fn to_group_key(&self) -> String {
        match self {
            Value::Float(f) => format!("{f}"),
            Value::Integer(i) => format!("{i}"),
            Value::Str(s) => s.clone(),
            Value::Bool(b) => format!("{b}"),
            Value::Na => "NA".to_string(),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Na, Value::Na) => true,
            _ => false,
        }
    }
}
