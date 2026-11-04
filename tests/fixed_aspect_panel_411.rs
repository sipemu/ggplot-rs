//! #411: a fixed-aspect coord shrinks the panel itself (ggplot2) instead of
//! letterboxing the data inside a full-size panel.

use ggplot_rs::prelude::*;

#[test]
fn coord_fixed_panel_has_the_aspect() {
    let svg = GGPlot::new(vec![
        (
            "x".to_string(),
            (0..5).map(|i| Value::Float(i as f64)).collect(),
        ),
        (
            "y".to_string(),
            (0..5).map(|i| Value::Float(i as f64)).collect(),
        ),
    ])
    .aes(Aes::new().x("x").y("y"))
    .geom_point()
    .coord_fixed(1.0)
    .render_svg_native_with_size(600, 300)
    .unwrap();
    let r = svg.split("<rect").nth(2).unwrap();
    let attr = |n: &str| -> f64 {
        let a = r.find(&format!(" {n}=\"")).unwrap() + n.len() + 3;
        r[a..].split('"').next().unwrap().parse().unwrap()
    };
    let (w, h) = (attr("width"), attr("height"));
    assert!((h / w - 1.0).abs() < 0.01, "panel {w} x {h}");
}
