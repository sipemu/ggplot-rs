use std::collections::HashMap;

use super::{DataFrame, Value};

/// Trait for types that can be converted into our internal DataFrame.
pub trait GGData {
    fn into_dataframe(self) -> DataFrame;
}

/// Convert a polars AnyValue to our internal Value enum.
#[cfg(feature = "polars")]
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
        AnyValue::Date(d) => Value::DateTime(d as i64 * 86400),
        AnyValue::Datetime(us, _, _) => Value::DateTime(us / 1_000_000),
        AnyValue::Duration(us, _) => Value::Integer(us),
        AnyValue::Time(ns) => Value::Integer(ns / 1_000_000_000),
        other => Value::Str(format!("{:?}", other)),
    }
}

/// polars DataFrame input: convert each column to our internal format.
#[cfg(feature = "polars")]
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

/// Extract a single Arrow array into a column of our internal `Value`s.
///
/// Nulls become `Value::Na`. Unsupported/complex types fall back to their
/// `Debug` string so no data is silently dropped.
#[cfg(feature = "arrow")]
fn arrow_array_to_values(array: &dyn arrow::array::Array) -> Vec<Value> {
    use arrow::array::{
        Array, BooleanArray, Date32Array, Date64Array, Float32Array, Float64Array, Int16Array,
        Int32Array, Int64Array, Int8Array, LargeStringArray, StringArray,
        TimestampMicrosecondArray, TimestampMillisecondArray, TimestampNanosecondArray,
        TimestampSecondArray, UInt16Array, UInt32Array, UInt64Array, UInt8Array,
    };
    use arrow::datatypes::{DataType, TimeUnit};

    let n = array.len();
    // Helper: downcast, then map each row (honouring nulls) via `f`.
    macro_rules! map_col {
        ($ty:ty, $f:expr) => {{
            let a = array.as_any().downcast_ref::<$ty>().unwrap();
            (0..n)
                .map(|i| {
                    if a.is_null(i) {
                        Value::Na
                    } else {
                        $f(a.value(i))
                    }
                })
                .collect()
        }};
    }

    match array.data_type() {
        DataType::Float64 => map_col!(Float64Array, Value::Float),
        DataType::Float32 => map_col!(Float32Array, |v: f32| Value::Float(v as f64)),
        DataType::Int64 => map_col!(Int64Array, Value::Integer),
        DataType::Int32 => map_col!(Int32Array, |v: i32| Value::Integer(v as i64)),
        DataType::Int16 => map_col!(Int16Array, |v: i16| Value::Integer(v as i64)),
        DataType::Int8 => map_col!(Int8Array, |v: i8| Value::Integer(v as i64)),
        DataType::UInt64 => map_col!(UInt64Array, |v: u64| Value::Integer(v as i64)),
        DataType::UInt32 => map_col!(UInt32Array, |v: u32| Value::Integer(v as i64)),
        DataType::UInt16 => map_col!(UInt16Array, |v: u16| Value::Integer(v as i64)),
        DataType::UInt8 => map_col!(UInt8Array, |v: u8| Value::Integer(v as i64)),
        DataType::Boolean => map_col!(BooleanArray, Value::Bool),
        DataType::Utf8 => map_col!(StringArray, |v: &str| Value::Str(v.to_string())),
        DataType::LargeUtf8 => map_col!(LargeStringArray, |v: &str| Value::Str(v.to_string())),
        // Date32 = days since epoch, Date64 = ms since epoch → seconds.
        DataType::Date32 => map_col!(Date32Array, |v: i32| Value::DateTime(v as i64 * 86_400)),
        DataType::Date64 => map_col!(Date64Array, |v: i64| Value::DateTime(v / 1_000)),
        DataType::Timestamp(unit, _) => match unit {
            TimeUnit::Second => map_col!(TimestampSecondArray, Value::DateTime),
            TimeUnit::Millisecond => {
                map_col!(TimestampMillisecondArray, |v: i64| Value::DateTime(
                    v / 1_000
                ))
            }
            TimeUnit::Microsecond => {
                map_col!(TimestampMicrosecondArray, |v: i64| Value::DateTime(
                    v / 1_000_000
                ))
            }
            TimeUnit::Nanosecond => {
                map_col!(TimestampNanosecondArray, |v: i64| Value::DateTime(
                    v / 1_000_000_000
                ))
            }
        },
        // Anything else (nested, decimal, etc.): keep the data as a string.
        _ => (0..n)
            .map(|i| {
                if array.is_null(i) {
                    Value::Na
                } else {
                    Value::Str(
                        arrow::util::display::array_value_to_string(array, i).unwrap_or_default(),
                    )
                }
            })
            .collect(),
    }
}

/// Arrow `RecordBatch` input: convert each column to our internal format.
///
/// This is the natural bridge for Arrow-native producers such as DuckDB, which
/// can emit query results directly as `RecordBatch`es without a polars detour.
#[cfg(feature = "arrow")]
impl GGData for arrow::record_batch::RecordBatch {
    fn into_dataframe(self) -> DataFrame {
        let mut df = DataFrame::new();
        let schema = self.schema();
        for (i, field) in schema.fields().iter().enumerate() {
            let values = arrow_array_to_values(self.column(i).as_ref());
            df.add_column(field.name().to_string(), values);
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

    #[cfg(feature = "arrow")]
    #[test]
    fn test_from_arrow_record_batch() {
        use arrow::array::{Float64Array, Int64Array, StringArray};
        use arrow::record_batch::RecordBatch;
        use std::sync::Arc;

        let batch = RecordBatch::try_from_iter(vec![
            (
                "x",
                Arc::new(Float64Array::from(vec![Some(1.0), None, Some(3.0)])) as _,
            ),
            ("n", Arc::new(Int64Array::from(vec![10, 20, 30])) as _),
            ("g", Arc::new(StringArray::from(vec!["a", "b", "c"])) as _),
        ])
        .unwrap();

        let df = batch.into_dataframe();
        assert_eq!(df.nrows(), 3);
        assert_eq!(df.ncols(), 3);
        assert!(df.has_column("x"));
        assert!(df.has_column("n"));
        assert!(df.has_column("g"));
        // Null in the float column becomes Value::Na.
        assert_eq!(df.column("x").unwrap()[1], Value::Na);
        assert_eq!(df.column("n").unwrap()[0], Value::Integer(10));
        assert_eq!(df.column("g").unwrap()[2], Value::Str("c".to_string()));
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
