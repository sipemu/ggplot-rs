//! Issue #15 — filled contour bands render end-to-end.

use ggplot_rs::prelude::*;

fn surface() -> Vec<(String, Vec<Value>)> {
    let (mut x, mut y, mut z) = (Vec::new(), Vec::new(), Vec::new());
    for i in 0..25 {
        for j in 0..25 {
            let xv = i as f64 - 12.0;
            let yv = j as f64 - 12.0;
            x.push(Value::Float(xv));
            y.push(Value::Float(yv));
            // A smooth bump.
            z.push(Value::Float((-(xv * xv + yv * yv) / 40.0).exp()));
        }
    }
    vec![
        ("x".to_string(), x),
        ("y".to_string(), y),
        ("z".to_string(), z),
    ]
}

#[test]
fn geom_contour_filled_renders() {
    let svg = GGPlot::new(surface())
        .aes(Aes::new().x("x").y("y"))
        .geom_contour_filled()
        .scale_fill_viridis_c()
        .render_svg()
        .expect("render");
    // Filled bands are drawn as SVG polygons/paths.
    assert!(
        svg.contains("<polygon") || svg.contains("<path"),
        "expected filled polygons in svg"
    );
}
