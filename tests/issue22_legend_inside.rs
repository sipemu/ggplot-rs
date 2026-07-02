//! Issue #22 — legend at arbitrary panel coordinates.
use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    let x: Vec<Value> = (0..30).map(|i| Value::Float(i as f64)).collect();
    let y: Vec<Value> = (0..30)
        .map(|i| Value::Float((i as f64 * 0.3).sin()))
        .collect();
    let g: Vec<Value> = (0..30)
        .map(|i| Value::Str(["a", "b", "c"][i % 3].into()))
        .collect();
    vec![("x".into(), x), ("y".into(), y), ("g".into(), g)]
}

#[test]
fn legend_inside_renders() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
        .legend_position_inside(0.85, 0.85)
        .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}

#[test]
fn legend_inside_via_theme() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
        .theme(Theme::default().set_legend_position(LegendPosition::Inside(0.1, 0.9)))
        .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}
