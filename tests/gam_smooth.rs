//! GAM smoothing (`method = "gam"`) via anofox-regression's P-spline.
#![cfg(feature = "regression")]

use ggplot_rs::data::{DataFrame, Value};
use ggplot_rs::prelude::*;
use ggplot_rs::scale::ScaleSet;
use ggplot_rs::stat::smooth::StatSmooth;
use ggplot_rs::stat::Stat;

/// A noisy sine wave — a trend a straight line cannot capture.
fn sine_frame() -> DataFrame {
    let mut df = DataFrame::new();
    let x: Vec<Value> = (0..60)
        .map(|i| Value::Float(i as f64 / 59.0 * std::f64::consts::TAU))
        .collect();
    let y: Vec<Value> = (0..60)
        .map(|i| {
            let xv = i as f64 / 59.0 * std::f64::consts::TAU;
            Value::Float(xv.sin() + 0.05 * ((i * 7 % 5) as f64 - 2.0))
        })
        .collect();
    df.add_column("x".into(), x);
    df.add_column("y".into(), y);
    df
}

fn smooth(method: SmoothMethod, data: &DataFrame) -> DataFrame {
    let stat = StatSmooth {
        n_points: 40,
        se: true,
        method,
    };
    stat.compute_group(data, &ScaleSet::new())
}

#[test]
fn gam_produces_line_and_ribbon() {
    let out = smooth(SmoothMethod::Gam, &sine_frame());
    assert_eq!(out.nrows(), 40);
    assert!(out.column("ymin").is_some() && out.column("ymax").is_some());
    // The ribbon must bracket the fitted line everywhere.
    let y: Vec<f64> = out
        .column("y")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();
    let lo: Vec<f64> = out
        .column("ymin")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();
    let hi: Vec<f64> = out
        .column("ymax")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();
    assert!((0..y.len()).all(|i| lo[i] <= y[i] + 1e-9 && y[i] <= hi[i] + 1e-9));
}

#[test]
fn gam_tracks_nonlinear_trend_better_than_lm() {
    let data = sine_frame();
    // RMSE of each fit's y against the true sin at the same grid x.
    let rmse_to_sin = |df: &DataFrame| -> f64 {
        let xs: Vec<f64> = df
            .column("x")
            .unwrap()
            .iter()
            .filter_map(|v| v.as_f64())
            .collect();
        let ys: Vec<f64> = df
            .column("y")
            .unwrap()
            .iter()
            .filter_map(|v| v.as_f64())
            .collect();
        (xs.iter()
            .zip(&ys)
            .map(|(x, y)| (y - x.sin()).powi(2))
            .sum::<f64>()
            / xs.len() as f64)
            .sqrt()
    };
    let gam = rmse_to_sin(&smooth(SmoothMethod::Gam, &data));
    let lm = rmse_to_sin(&smooth(SmoothMethod::Lm, &data));
    assert!(gam < lm * 0.5, "gam rmse {gam} should beat lm rmse {lm}");
    assert!(gam < 0.1, "gam should closely track the sine (rmse {gam})");
}

#[test]
fn gam_builder_renders_end_to_end() {
    let df = sine_frame();
    let data = vec![
        ("x".to_string(), df.column("x").unwrap().to_vec()),
        ("y".to_string(), df.column("y").unwrap().to_vec()),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_smooth_with(GeomSmooth::default().gam())
        .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}

#[test]
fn gam_falls_back_to_line_for_tiny_groups() {
    // Fewer than 6 points → straight-line fallback, still returns a curve.
    let mut df = DataFrame::new();
    df.add_column("x".into(), (0..4).map(|i| Value::Float(i as f64)).collect());
    df.add_column(
        "y".into(),
        (0..4).map(|i| Value::Float(2.0 * i as f64 + 1.0)).collect(),
    );
    let out = smooth(SmoothMethod::Gam, &df);
    assert!(out.nrows() >= 2);
    let y: Vec<f64> = out
        .column("y")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();
    assert!(y[y.len() - 1] > y[0], "fallback line should increase");
}
