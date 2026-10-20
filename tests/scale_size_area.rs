//! B3-6: `scale_size_area` (ggplot2) — size scale where *area*, not
//! radius, is proportional to the value.

use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    vec![
        (
            "x".into(),
            vec![
                Value::Float(0.0),
                Value::Float(1.0),
                Value::Float(2.0),
                Value::Float(3.0),
            ],
        ),
        (
            "y".into(),
            vec![
                Value::Float(0.0),
                Value::Float(0.0),
                Value::Float(0.0),
                Value::Float(0.0),
            ],
        ),
        (
            "v".into(),
            vec![
                Value::Float(0.0),
                Value::Float(1.0),
                Value::Float(4.0),
                Value::Float(16.0),
            ],
        ),
    ]
}

#[test]
fn scale_size_area_renders_through_the_full_api() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y").size("v"))
        .geom_point()
        .scale_size_area(6.0)
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("<svg"));
    assert!(!svg.contains("NaN"));
}

#[test]
fn a_negative_max_size_warns_but_still_renders() {
    let (svg, warnings) = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y").size("v"))
        .geom_point()
        .scale_size_area(-1.0)
        .render_svg_native_with_warnings(300, 200)
        .unwrap();
    assert!(!svg.contains("NaN"));
    assert!(
        warnings.iter().any(|w| w.contains("max_size")),
        "{warnings:?}"
    );
}
