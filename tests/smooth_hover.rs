//! geom_smooth fitted line carries hover tooltips (fitted y + CI) in the
//! native SVG backend.

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    let x: Vec<Value> = (0..40).map(|i| Value::Float(i as f64 * 0.2)).collect();
    let y: Vec<Value> = (0..40)
        .map(|i| Value::Float((i as f64 * 0.2).sin() * 2.0 + i as f64 * 0.05))
        .collect();
    vec![("x".to_string(), x), ("y".to_string(), y)]
}

#[test]
fn smooth_line_emits_hover_tooltips() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_smooth_with(GeomSmooth {
            method: SmoothMethod::Loess { span: 0.6 },
            ..Default::default()
        })
        .render_svg_native()
        .expect("render");
    // The fitted curve emits transparent hover marks tagged "ŷ = <value>".
    assert!(
        svg.contains("ŷ ="),
        "expected fitted-value tooltips on the smooth line"
    );
    // With se on, the tooltip also carries the confidence interval "[lo, hi]".
    assert!(svg.contains('['), "expected a CI in the smooth tooltip");
}
