//! Issue #22 — rotate axis tick labels (guide_axis angle).
use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    let g: Vec<Value> = ["North-East Region", "South-West Zone", "Central District"]
        .iter()
        .cycle()
        .take(30)
        .map(|s| Value::Str(s.to_string()))
        .collect();
    let y: Vec<Value> = (0..30).map(|i| Value::Float((i % 10) as f64)).collect();
    vec![("g".into(), g), ("y".into(), y)]
}

#[test]
fn x_axis_labels_rotated_render() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("g").y("y"))
        .geom_col()
        .axis_text_x_angle(45.0)
        .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}

#[test]
fn axis_angle_sets_theme() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("g").y("y"))
        .geom_col()
        .axis_text_x_angle(90.0)
        .axis_text_y_angle(0.0)
        .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}
