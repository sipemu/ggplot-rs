//! #466: `from_polars!` expands against the caller's polars — here the
//! crate's own polars 0.46 (`polars` feature); `examples/polars_compat`
//! checks a newer polars in CI-free local runs.
#![cfg(feature = "polars")]

use ggplot_rs::prelude::*;
use polars::prelude::*;

#[test]
fn converts_every_basic_dtype() {
    let mut df = polars::df!(
        "i" => [1i32, 2, 3],
        "f" => [Some(1.5), None, Some(3.0)],
        "s" => ["a", "b", "c"],
        "b" => [true, false, true],
        "d" => [0i32, 1, 365],
        "t" => [0i64, 1_500, 86_400_000],
    )
    .unwrap();
    df.apply("d", |s| s.cast(&DataType::Date).unwrap()).unwrap();
    df.apply("t", |s| {
        s.cast(&DataType::Datetime(TimeUnit::Milliseconds, None))
            .unwrap()
    })
    .unwrap();
    let cols = ggplot_rs::from_polars!(&df).unwrap();
    let col = |n: &str| cols.iter().find(|(c, _)| c == n).unwrap().1.clone();
    assert_eq!(col("i")[2], Value::Integer(3));
    assert_eq!(col("f")[1], Value::Na);
    assert_eq!(col("s")[0], Value::Str("a".into()));
    assert_eq!(col("b")[1], Value::Bool(false));
    assert_eq!(col("d")[2], Value::DateTime(365 * 86_400));
    assert_eq!(col("t")[1], Value::DateTime(1));
    assert_eq!(col("t")[2], Value::DateTime(86_400));
}

#[test]
fn plugs_into_ggplot() {
    let df = polars::df!("x" => [1.0, 2.0], "y" => [3.0, 4.0]).unwrap();
    let svg = GGPlot::new(ggplot_rs::from_polars!(&df).unwrap())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert_eq!(svg.matches("<circle").count(), 2);
}
