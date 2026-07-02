//! Issue #17 — coord_trans warps the axis at draw time (stats on raw data).

use ggplot_rs::prelude::*;

fn expo() -> Vec<(String, Vec<Value>)> {
    let x: Vec<Value> = (1..=6).map(|i| Value::Float(i as f64)).collect();
    let y: Vec<Value> = (1..=6).map(|i| Value::Float(10f64.powi(i - 1))).collect();
    vec![("x".to_string(), x), ("y".to_string(), y)]
}

#[test]
fn coord_trans_y_log_renders() {
    let svg = GGPlot::new(expo())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .coord_trans_y(ScaleTransform::Log10)
        .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}

#[test]
fn coord_trans_both_axes_and_sqrt() {
    let svg = GGPlot::new(expo())
        .aes(Aes::new().x("x").y("y"))
        .geom_line()
        .coord_trans(Some(ScaleTransform::Sqrt), Some(ScaleTransform::Log10))
        .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}
