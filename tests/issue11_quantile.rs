//! Issue #11 — quantile regression (stat_quantile) via anofox-regression.
#![cfg(feature = "regression")]

use ggplot_rs::data::{DataFrame, Value};
use ggplot_rs::prelude::*;
use ggplot_rs::scale::ScaleSet;
use ggplot_rs::stat::Stat;

fn linear_cloud() -> (Vec<f64>, Vec<f64>) {
    // y ≈ 2 + 3x with a little deterministic spread.
    let x: Vec<f64> = (0..60).map(|i| i as f64 * 0.5).collect();
    let y: Vec<f64> = x
        .iter()
        .enumerate()
        .map(|(i, &xv)| 2.0 + 3.0 * xv + ((i * 37 % 11) as f64 - 5.0))
        .collect();
    (x, y)
}

#[test]
fn stat_quantile_recovers_slope() {
    let (x, y) = linear_cloud();
    let mut df = DataFrame::new();
    df.add_column("x".into(), x.iter().map(|v| Value::Float(*v)).collect());
    df.add_column("y".into(), y.iter().map(|v| Value::Float(*v)).collect());

    let out = StatQuantile::new(0.5).compute_group(&df, &ScaleSet::new());
    assert!(out.nrows() >= 2);
    let xs: Vec<f64> = out
        .column("x")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();
    let ys: Vec<f64> = out
        .column("y")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();
    // Estimated slope over the fitted line should be near the true slope (3).
    let slope = (ys[ys.len() - 1] - ys[0]) / (xs[xs.len() - 1] - xs[0]);
    assert!((slope - 3.0).abs() < 0.5, "slope {slope}");
}

#[test]
fn geom_quantile_renders() {
    let (x, y) = linear_cloud();
    let data = vec![
        (
            "x".to_string(),
            x.iter().map(|v| Value::Float(*v)).collect(),
        ),
        (
            "y".to_string(),
            y.iter().map(|v| Value::Float(*v)).collect(),
        ),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_quantile()
        .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}
