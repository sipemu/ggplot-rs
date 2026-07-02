//! Issue #10 — geom_smooth glm/rlm methods via anofox-regression.
#![cfg(feature = "regression")]

use ggplot_rs::data::{DataFrame, Value};
use ggplot_rs::prelude::*;
use ggplot_rs::scale::ScaleSet;
use ggplot_rs::stat::smooth::StatSmooth;
use ggplot_rs::stat::Stat;

fn frame() -> DataFrame {
    let mut df = DataFrame::new();
    let x: Vec<Value> = (0..40).map(|i| Value::Float(i as f64 * 0.25)).collect();
    let y: Vec<Value> = (0..40)
        .map(|i| Value::Float(1.0 + 0.8 * (i as f64 * 0.25) + ((i * 7 % 5) as f64 - 2.0)))
        .collect();
    df.add_column("x".into(), x);
    df.add_column("y".into(), y);
    df
}

fn smooth(method: SmoothMethod) -> DataFrame {
    let stat = StatSmooth {
        n_points: 30,
        se: true,
        method,
    };
    stat.compute_group(&frame(), &ScaleSet::new())
}

#[test]
fn gaussian_glm_produces_line_and_ribbon() {
    let out = smooth(SmoothMethod::Glm {
        family: SmoothFamily::Gaussian,
    });
    assert_eq!(out.nrows(), 30);
    assert!(out.column("ymin").is_some() && out.column("ymax").is_some());
    // Fitted line should rise with x (positive slope in the data).
    let ys: Vec<f64> = out
        .column("y")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();
    assert!(ys[ys.len() - 1] > ys[0], "line should increase");
}

#[test]
fn rlm_renders_on_continuous_data() {
    let data = vec![
        ("x".to_string(), frame().column("x").unwrap().to_vec()),
        ("y".to_string(), frame().column("y").unwrap().to_vec()),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_smooth_with(GeomSmooth {
            method: SmoothMethod::Rlm,
            se: true,
            ..Default::default()
        })
        .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}

#[test]
fn poisson_glm_renders_on_counts() {
    // Poisson needs non-negative count responses.
    let x: Vec<Value> = (0..40).map(|i| Value::Float(i as f64 * 0.2)).collect();
    let y: Vec<Value> = (0..40)
        .map(|i| Value::Float(((i as f64 * 0.2).exp().min(50.0)).round()))
        .collect();
    let data = vec![("x".to_string(), x), ("y".to_string(), y)];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_smooth_with(GeomSmooth {
            method: SmoothMethod::Glm {
                family: SmoothFamily::Poisson,
            },
            se: false,
            ..Default::default()
        })
        .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}
