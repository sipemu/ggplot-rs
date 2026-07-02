//! Issue #12 — confidence ellipses render end-to-end.

use ggplot_rs::prelude::*;

fn cloud(groups: usize) -> Vec<(String, Vec<Value>)> {
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut g = Vec::new();
    for gi in 0..groups {
        for i in 0..40 {
            let t = i as f64 * 0.37 + gi as f64;
            x.push(Value::Float(
                gi as f64 * 5.0 + t.sin() * 2.0 + (i % 3) as f64,
            ));
            y.push(Value::Float(
                gi as f64 * 3.0 + t.cos() * 1.5 + (i % 2) as f64,
            ));
            g.push(Value::Str(format!("grp{gi}")));
        }
    }
    vec![
        ("x".to_string(), x),
        ("y".to_string(), y),
        ("g".to_string(), g),
    ]
}

#[test]
fn stat_ellipse_single_group_renders() {
    let svg = GGPlot::new(cloud(1))
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .stat_ellipse()
        .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}

#[test]
fn stat_ellipse_per_color_group_renders() {
    let svg = GGPlot::new(cloud(3))
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
        .stat_ellipse_level(0.9)
        .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}
