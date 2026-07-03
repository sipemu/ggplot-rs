//! Issue #50 — aspect.ratio, minor ticks, panel.ontop.
use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    vec![
        (
            "x".to_string(),
            (0..20).map(|i| Value::Float(i as f64)).collect(),
        ),
        (
            "y".to_string(),
            (0..20)
                .map(|i| Value::Float((i as f64 * 0.4).sin()))
                .collect(),
        ),
    ]
}

fn segments(svg: &str) -> usize {
    svg.matches("<line").count() + svg.matches("<polyline").count()
}

#[test]
fn minor_ticks_add_marks() {
    let plain = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_line()
        .render_svg()
        .unwrap();
    let minor = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_line()
        .axis_minor_ticks()
        .render_svg()
        .unwrap();
    assert!(
        segments(&minor) > segments(&plain),
        "minor ticks should add tick marks"
    );
}

#[test]
fn aspect_ratio_and_ontop_render() {
    assert!(GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_line()
        .aspect_ratio(1.0)
        .render_svg()
        .is_ok());
    assert!(GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_area()
        .panel_ontop()
        .render_svg()
        .is_ok());
}
