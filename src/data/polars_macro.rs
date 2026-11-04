//! [`from_polars!`](crate::from_polars): polars input that works with
//! *your* polars version (#466).

/// Convert a polars `DataFrame` into plot-ready columns using **your**
/// crate's `polars` (#466): the macro expands in your crate, against the
/// polars version you depend on, so it works whether you use polars 0.46,
/// 0.52, 0.53 or later — no second polars build, no type mismatch, and
/// ggplot-rs's own `polars` feature can stay off.
///
/// Returns `PolarsResult<Vec<(String, Vec<Value>)>>`, which is
/// [`GGData`](crate::data::GGData), so it goes straight into
/// [`GGPlot::new`](crate::GGPlot::new):
///
/// ```ignore
/// use ggplot_rs::prelude::*;
/// let df = polars::df!("x" => [1, 2, 3], "y" => [2.0, 1.0, 3.0])?;
/// let plot = GGPlot::new(ggplot_rs::from_polars!(&df)?)
///     .aes(Aes::new().x("x").y("y"))
///     .geom_point();
/// ```
///
/// Booleans, strings, integers, floats, `Date` (→ seconds) and `Datetime`
/// (any unit → seconds) convert natively; every other dtype (categoricals,
/// decimals, nested …) is cast to its string form. Needs `polars` in scope
/// as `::polars` with its (default) `dtype-date` / `dtype-datetime`
/// features.
#[macro_export]
macro_rules! from_polars {
    ($df:expr) => {{
        (|| -> ::polars::prelude::PolarsResult<::std::vec::Vec<(::std::string::String, ::std::vec::Vec<$crate::data::Value>)>> {
            use ::polars::prelude::{DataType as D, TimeUnit as U};
            use $crate::data::Value as V;
            let df = $df;
            let mut out = ::std::vec::Vec::new();
            for name in df.get_column_names() {
                let name = name.to_string();
                let c = df.column(&name)?;
                let values: ::std::vec::Vec<V> = match c.dtype() {
                    D::Boolean => c.bool()?.into_iter().map(|v| v.map_or(V::Na, V::Bool)).collect(),
                    D::String => c
                        .str()?
                        .into_iter()
                        .map(|v| v.map_or(V::Na, |s| V::Str(s.to_string())))
                        .collect(),
                    D::Float32 | D::Float64 => {
                        let cc = c.cast(&D::Float64)?;
                        let v = cc.f64()?.into_iter().map(|v| v.map_or(V::Na, V::Float)).collect();
                        v
                    }
                    D::Int8 | D::Int16 | D::Int32 | D::Int64 | D::UInt8 | D::UInt16 | D::UInt32
                    | D::UInt64 => {
                        let cc = c.cast(&D::Int64)?;
                        let v = cc.i64()?.into_iter().map(|v| v.map_or(V::Na, V::Integer)).collect();
                        v
                    }
                    D::Date => {
                        let cc = c.cast(&D::Int32)?;
                        let v = cc
                            .i32()?
                            .into_iter()
                            .map(|v| v.map_or(V::Na, |d| V::DateTime(d as i64 * 86_400)))
                            .collect();
                        v
                    }
                    D::Datetime(unit, _) => {
                        let per_second: i64 = match unit {
                            U::Nanoseconds => 1_000_000_000,
                            U::Microseconds => 1_000_000,
                            U::Milliseconds => 1_000,
                        };
                        let cc = c.cast(&D::Int64)?;
                        let v = cc
                            .i64()?
                            .into_iter()
                            .map(|v| v.map_or(V::Na, |t| V::DateTime(t.div_euclid(per_second))))
                            .collect();
                        v
                    }
                    _ => {
                        let cc = c.cast(&D::String)?;
                        let v = cc
                            .str()?
                            .into_iter()
                            .map(|v| v.map_or(V::Na, |s| V::Str(s.to_string())))
                            .collect();
                        v
                    }
                };
                out.push((name, values));
            }
            Ok(out)
        })()
    }};
}
