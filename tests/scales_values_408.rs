//! #408: manual / identity scales for size, alpha, linewidth and shape;
//! `scale_*_steps2`, `scale_*_viridis_b`, `scale_radius`.

use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    vec![
        (
            "x".into(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "y".into(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "g".into(),
            ["a", "b", "c"]
                .iter()
                .map(|s| Value::Str(s.to_string()))
                .collect(),
        ),
        (
            "s".into(),
            vec![Value::Float(2.0), Value::Float(5.0), Value::Float(9.0)],
        ),
        (
            "a".into(),
            vec![Value::Float(0.2), Value::Float(0.5), Value::Float(1.0)],
        ),
        (
            "p".into(),
            vec![Value::Integer(17), Value::Integer(15), Value::Integer(19)],
        ),
        (
            "z".into(),
            vec![Value::Float(-10.0), Value::Float(0.0), Value::Float(30.0)],
        ),
    ]
}

fn svg(p: GGPlot) -> String {
    p.render_svg_native_with_size(400, 300).unwrap()
}

fn radii(svg: &str) -> Vec<f64> {
    svg.split("<circle")
        .skip(1)
        .filter_map(|t| {
            let a = t.find(" r=\"")? + 4;
            t[a..].split('"').next()?.parse().ok()
        })
        .collect()
}

#[test]
fn size_manual_and_identity() {
    let manual = svg(GGPlot::new(data())
        .aes(Aes::new().x("x").y("y").size("g"))
        .geom_point()
        .scale_size_manual(vec![("a", 1.0), ("b", 4.0), ("c", 8.0)])
        .legend_position(LegendPosition::None));
    let r = radii(&manual);
    assert_eq!(r.len(), 3, "{r:?}");
    assert!(r[0] < r[1] && r[1] < r[2], "{r:?}");
    let identity = svg(GGPlot::new(data())
        .aes(Aes::new().x("x").y("y").size("s"))
        .geom_point()
        .scale_size_identity());
    let ri = radii(&identity);
    // No legend for an identity scale: only the three data points.
    assert_eq!(ri.len(), 3, "{ri:?}");
    assert!((ri[2] / ri[0] - 9.0 / 2.0).abs() < 0.05, "{ri:?}");
}

#[test]
fn alpha_manual_and_identity() {
    let s = svg(GGPlot::new(data())
        .aes(Aes::new().x("x").y("y").alpha("a"))
        .geom_point()
        .scale_alpha_identity());
    assert!(
        s.contains("opacity=\"0.2") || s.contains("opacity=\"0.20"),
        "{s}"
    );
    let m = svg(GGPlot::new(data())
        .aes(Aes::new().x("x").y("y").alpha("g"))
        .geom_point()
        .scale_alpha_manual(vec![("a", 0.3), ("b", 0.6), ("c", 0.9)]));
    assert!(
        m.contains("opacity=\"0.3") && m.contains("opacity=\"0.9"),
        "{m}"
    );
    assert!(m.contains(">b<"), "manual alpha keeps its legend");
}

#[test]
fn shape_identity_uses_pch_codes() {
    let s = svg(GGPlot::new(data())
        .aes(Aes::new().x("x").y("y").shape("p"))
        .geom_point()
        .scale_shape_identity());
    // 17 → triangle (a polygon), 15 → square (a rect / polygon), 19 → circle.
    assert_eq!(radii(&s).len(), 1, "{s}");
    assert!(s.contains("<polygon"), "{s}");
}

#[test]
fn steps2_and_viridis_b_build() {
    for p in [
        GGPlot::new(data())
            .aes(Aes::new().x("x").y("y").color("z"))
            .geom_point()
            .scale_color_steps2(
                RGBAColor::new(0, 0, 255),
                RGBAColor::new(255, 255, 255),
                RGBAColor::new(255, 0, 0),
                0.0,
                4,
            ),
        GGPlot::new(data())
            .aes(Aes::new().x("x").y("y").fill("z"))
            .geom_point()
            .scale_fill_viridis_b('C', 5),
        GGPlot::new(data())
            .aes(Aes::new().x("x").y("y").size("s"))
            .geom_point()
            .scale_radius(1.0, 10.0),
    ] {
        let s = svg(p);
        assert!(s.contains("<circle"));
    }
}
