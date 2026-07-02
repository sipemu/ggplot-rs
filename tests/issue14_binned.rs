//! Issue #14 — binned (stepped) colour scales render with a stepped legend.

use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    let n = 60;
    vec![
        (
            "x".to_string(),
            (0..n).map(|i| Value::Float(i as f64)).collect(),
        ),
        (
            "y".to_string(),
            (0..n)
                .map(|i| Value::Float((i as f64 * 0.3).sin()))
                .collect(),
        ),
        (
            "z".to_string(),
            (0..n).map(|i| Value::Float(i as f64)).collect(),
        ),
    ]
}

#[test]
fn scale_color_steps_renders() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y").color("z"))
        .geom_point()
        .scale_color_steps(RGBAColor::new(0, 40, 120), RGBAColor::new(220, 40, 40), 5)
        .render_svg()
        .expect("render");
    assert!(svg.contains("<svg"));
}

#[test]
fn scale_color_fermenter_renders() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y").color("z"))
        .geom_point()
        .scale_color_fermenter(PaletteName::Blues, 6)
        .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}

#[test]
fn scale_fill_steps_renders() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y").fill("z"))
        .geom_tile()
        .scale_fill_steps(
            RGBAColor::new(240, 240, 240),
            RGBAColor::new(20, 20, 120),
            4,
        )
        .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}
