//! #413: `element_geom(pointsize =, linewidth =)` — theme-wide geom defaults.

use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    vec![
        ("x".into(), (0..3).map(|i| Value::Float(i as f64)).collect()),
        ("y".into(), (0..3).map(|i| Value::Float(i as f64)).collect()),
    ]
}

#[test]
fn pointsize_and_linewidth_defaults() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_line()
        .theme_with(|t| {
            t.geom = ElementGeom::default()
                .with_pointsize(7.0)
                .with_linewidth(4.0)
        })
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains(" r=\"7.00\""), "point size");
    assert!(svg.contains("stroke-width=\"4.00\""), "line width");
    // An explicit size still wins.
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point_with(GeomPoint {
            size: 2.0,
            ..Default::default()
        })
        .theme_with(|t| t.geom = ElementGeom::default().with_pointsize(7.0))
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains(" r=\"2.00\""));
}
