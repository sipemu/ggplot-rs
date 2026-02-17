use std::collections::HashMap;

use super::{DataFrame, Value};

/// Trait for types that can be converted into our internal DataFrame.
pub trait GGData {
    fn into_dataframe(self) -> DataFrame;
}

/// Convert a polars AnyValue to our internal Value enum.
fn polars_anyvalue_to_value(v: polars::datatypes::AnyValue) -> Value {
    use polars::datatypes::AnyValue;
    match v {
        AnyValue::Float64(f) => Value::Float(f),
        AnyValue::Float32(f) => Value::Float(f as f64),
        AnyValue::Int64(i) => Value::Integer(i),
        AnyValue::Int32(i) => Value::Integer(i as i64),
        AnyValue::Int16(i) => Value::Integer(i as i64),
        AnyValue::Int8(i) => Value::Integer(i as i64),
        AnyValue::UInt64(i) => Value::Integer(i as i64),
        AnyValue::UInt32(i) => Value::Integer(i as i64),
        AnyValue::UInt16(i) => Value::Integer(i as i64),
        AnyValue::UInt8(i) => Value::Integer(i as i64),
        AnyValue::Boolean(b) => Value::Bool(b),
        AnyValue::String(s) => Value::Str(s.to_string()),
        AnyValue::StringOwned(s) => Value::Str(s.to_string()),
        AnyValue::Null => Value::Na,
        other => Value::Str(format!("{:?}", other)),
    }
}

/// polars DataFrame input: convert each column to our internal format.
impl GGData for polars::frame::DataFrame {
    fn into_dataframe(self) -> DataFrame {
        let mut df = DataFrame::new();
        for col in self.get_columns() {
            let name = col.name().to_string();
            let values: Vec<Value> = (0..col.len())
                .map(|i| polars_anyvalue_to_value(col.get(i).unwrap()))
                .collect();
            df.add_column(name, values);
        }
        df
    }
}

/// Row-oriented input: Vec of HashMaps.
impl GGData for Vec<HashMap<String, Value>> {
    fn into_dataframe(self) -> DataFrame {
        if self.is_empty() {
            return DataFrame::new();
        }

        // Collect all column names
        let mut col_names: Vec<String> = Vec::new();
        for row in &self {
            for key in row.keys() {
                if !col_names.contains(key) {
                    col_names.push(key.clone());
                }
            }
        }

        let mut df = DataFrame::new();
        for name in &col_names {
            let values: Vec<Value> = self
                .iter()
                .map(|row| row.get(name).cloned().unwrap_or(Value::Na))
                .collect();
            df.add_column(name.clone(), values);
        }
        df
    }
}

/// Column-oriented input: Vec of (name, values) pairs.
impl GGData for Vec<(String, Vec<Value>)> {
    fn into_dataframe(self) -> DataFrame {
        let mut df = DataFrame::new();
        for (name, values) in self {
            df.add_column(name, values);
        }
        df
    }
}

/// Identity: DataFrame passes through.
impl GGData for DataFrame {
    fn into_dataframe(self) -> DataFrame {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_hashmap_vec() {
        let data = vec![
            HashMap::from([
                ("x".to_string(), Value::Float(1.0)),
                ("y".to_string(), Value::Float(2.0)),
            ]),
            HashMap::from([
                ("x".to_string(), Value::Float(3.0)),
                ("y".to_string(), Value::Float(4.0)),
            ]),
        ];

        let df = data.into_dataframe();
        assert_eq!(df.nrows(), 2);
        assert!(df.has_column("x"));
        assert!(df.has_column("y"));
    }

    #[test]
    fn test_from_column_oriented() {
        let data = vec![
            ("x".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
            ("y".to_string(), vec![Value::Float(3.0), Value::Float(4.0)]),
        ];

        let df = data.into_dataframe();
        assert_eq!(df.nrows(), 2);
        assert_eq!(df.ncols(), 2);
    }
}
