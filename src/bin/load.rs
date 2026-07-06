//! Data loading for the CLI: run a DuckDB query and convert the Arrow result to
//! ggplot-rs's internal column format.

use duckdb::arrow::array::{
    Array, BooleanArray, Date32Array, Decimal128Array, Float32Array, Float64Array, Int16Array,
    Int32Array, Int64Array, Int8Array, LargeStringArray, StringArray, TimestampMicrosecondArray,
    TimestampMillisecondArray, TimestampSecondArray, UInt16Array, UInt32Array, UInt64Array,
    UInt8Array,
};
use duckdb::arrow::datatypes::{DataType, TimeUnit};
use duckdb::arrow::record_batch::RecordBatch;
use duckdb::Connection;
use ggplot_rs::prelude::Value;

/// Turn the chosen input into a DuckDB SQL query.
pub fn resolve_query(
    sql: &Option<String>,
    parquet: &Option<String>,
    csv: &Option<String>,
) -> Option<String> {
    if let Some(q) = sql {
        Some(q.clone())
    } else if let Some(p) = parquet {
        Some(format!(
            "SELECT * FROM read_parquet('{}')",
            p.replace('\'', "''")
        ))
    } else {
        csv.as_ref()
            .map(|c| format!("SELECT * FROM read_csv_auto('{}')", c.replace('\'', "''")))
    }
}

/// Run `query` against DuckDB (in-memory unless `db` is given) and return the
/// result as named columns of `Value`s. When `spatial` is set, the DuckDB
/// `spatial` extension is installed/loaded first, so `ST_Read(...)` (shapefiles,
/// GeoJSON, GeoPackage, …) and `ST_AsText(geom)` are available.
pub fn load(
    db: &Option<String>,
    query: &str,
    spatial: bool,
) -> Result<Vec<(String, Vec<Value>)>, String> {
    let conn = match db {
        Some(path) => Connection::open(path),
        None => Connection::open_in_memory(),
    }
    .map_err(|e| format!("opening DuckDB: {e}"))?;

    if spatial {
        conn.execute_batch("INSTALL spatial; LOAD spatial;")
            .map_err(|e| format!("loading DuckDB spatial extension: {e}"))?;
    }

    let mut stmt = conn
        .prepare(query)
        .map_err(|e| format!("preparing query: {e}"))?;
    let batches: Vec<RecordBatch> = stmt
        .query_arrow([])
        .map_err(|e| format!("running query: {e}"))?
        .collect();

    let schema = match batches.first() {
        Some(b) => b.schema(),
        None => return Err("query returned no columns".into()),
    };

    let mut columns: Vec<(String, Vec<Value>)> = schema
        .fields()
        .iter()
        .map(|f| (f.name().to_string(), Vec::new()))
        .collect();

    for batch in &batches {
        for (ci, col) in columns.iter_mut().enumerate() {
            col.1.extend(array_to_values(batch.column(ci).as_ref()));
        }
    }
    Ok(columns)
}

/// Print the schema (column, inferred type, non-null / total) for discovery.
pub fn describe(columns: &[(String, Vec<Value>)]) {
    let nrows = columns.first().map(|c| c.1.len()).unwrap_or(0);
    println!("{nrows} rows, {} columns", columns.len());
    let width = columns.iter().map(|(n, _)| n.len()).max().unwrap_or(4);
    for (name, vals) in columns {
        let ty = vals
            .iter()
            .find_map(|v| match v {
                Value::Float(_) => Some("double"),
                Value::Integer(_) => Some("integer"),
                Value::Str(_) => Some("string"),
                Value::Bool(_) => Some("boolean"),
                Value::DateTime(_) => Some("datetime"),
                Value::Na => None,
            })
            .unwrap_or("null");
        let non_null = vals.iter().filter(|v| !matches!(v, Value::Na)).count();
        println!("  {name:<width$}  {ty:<8}  {non_null}/{nrows} non-null");
    }
}

macro_rules! prim {
    ($arr:expr, $n:expr, $ty:ty, $variant:ident, $cast:expr) => {{
        let a = $arr.as_any().downcast_ref::<$ty>().unwrap();
        (0..$n)
            .map(|i| {
                if a.is_null(i) {
                    Value::Na
                } else {
                    Value::$variant($cast(a.value(i)))
                }
            })
            .collect()
    }};
}

fn array_to_values(arr: &dyn Array) -> Vec<Value> {
    let n = arr.len();
    match arr.data_type() {
        DataType::Float64 => prim!(arr, n, Float64Array, Float, |v| v),
        DataType::Float32 => prim!(arr, n, Float32Array, Float, |v: f32| v as f64),
        DataType::Int64 => prim!(arr, n, Int64Array, Integer, |v| v),
        DataType::Int32 => prim!(arr, n, Int32Array, Integer, |v: i32| v as i64),
        DataType::Int16 => prim!(arr, n, Int16Array, Integer, |v: i16| v as i64),
        DataType::Int8 => prim!(arr, n, Int8Array, Integer, |v: i8| v as i64),
        DataType::UInt64 => prim!(arr, n, UInt64Array, Integer, |v: u64| v as i64),
        DataType::UInt32 => prim!(arr, n, UInt32Array, Integer, |v: u32| v as i64),
        DataType::UInt16 => prim!(arr, n, UInt16Array, Integer, |v: u16| v as i64),
        DataType::UInt8 => prim!(arr, n, UInt8Array, Integer, |v: u8| v as i64),
        DataType::Boolean => prim!(arr, n, BooleanArray, Bool, |v| v),
        // DuckDB DECIMAL(p, s) → scaled i128.
        DataType::Decimal128(_, scale) => {
            let a = arr.as_any().downcast_ref::<Decimal128Array>().unwrap();
            let div = 10f64.powi(*scale as i32);
            (0..n)
                .map(|i| {
                    if a.is_null(i) {
                        Value::Na
                    } else {
                        Value::Float(a.value(i) as f64 / div)
                    }
                })
                .collect()
        }
        DataType::Utf8 => prim!(arr, n, StringArray, Str, |v: &str| v.to_string()),
        DataType::LargeUtf8 => prim!(arr, n, LargeStringArray, Str, |v: &str| v.to_string()),
        // Date32 = days since epoch → seconds.
        DataType::Date32 => prim!(arr, n, Date32Array, DateTime, |v: i32| v as i64 * 86_400),
        DataType::Timestamp(unit, _) => match unit {
            TimeUnit::Second => prim!(arr, n, TimestampSecondArray, DateTime, |v| v),
            TimeUnit::Millisecond => {
                prim!(arr, n, TimestampMillisecondArray, DateTime, |v: i64| v
                    / 1_000)
            }
            TimeUnit::Microsecond => {
                prim!(arr, n, TimestampMicrosecondArray, DateTime, |v: i64| v
                    / 1_000_000)
            }
            TimeUnit::Nanosecond => prim!(arr, n, TimestampMicrosecondArray, DateTime, |v: i64| v
                / 1_000_000),
        },
        // Fallback: nulls (unsupported type). Keeps the column present.
        _ => (0..n).map(|_| Value::Na).collect(),
    }
}
