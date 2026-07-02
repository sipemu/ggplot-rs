//! Issue #22 — labs(tag) and guide_axis(n.dodge).
use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    let g: Vec<Value> = (0..12)
        .map(|i| Value::Str(format!("Category-{i}")))
        .collect();
    let y: Vec<Value> = (0..12)
        .map(|i| Value::Float((i * 3 % 7) as f64 + 1.0))
        .collect();
    vec![("g".into(), g), ("y".into(), y)]
}

#[test]
fn tag_renders() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("g").y("y"))
        .geom_col()
        .tag("A")
        .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}

#[test]
fn x_label_dodge_renders() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("g").y("y"))
        .geom_col()
        .axis_text_x_dodge(2)
        .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}
