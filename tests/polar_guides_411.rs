//! #411: polar / radial guides — rings at the r breaks, spokes and labels
//! at the theta breaks — and `coord_radial`'s `r.axis.inside`,
//! `rotate.angle` and `reverse`.

use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    vec![
        (
            "x".into(),
            ["a", "b", "c", "d"]
                .iter()
                .map(|s| Value::Str(s.to_string()))
                .collect(),
        ),
        (
            "y".into(),
            (1..5).map(|i| Value::Float(i as f64 * 5.0)).collect(),
        ),
    ]
}

fn svg(c: CoordPolar) -> String {
    GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_col()
        .coord_polar_with(c)
        .render_svg_native_with_size(400, 400)
        .unwrap()
}

/// `(x, y, transform)` of each `<text>` whose content is `label`.
fn text(s: &str, label: &str) -> (f64, f64, String) {
    let t = s
        .split("<text")
        .find(|t| {
            t.split('>')
                .nth(1)
                .is_some_and(|c| c.starts_with(&format!("{label}<")))
        })
        .unwrap_or_else(|| panic!("no {label}"));
    let tag = &t[..t.find('>').unwrap()];
    let attr = |n: &str| -> Option<String> {
        let a = tag.find(&format!(" {n}=\""))? + n.len() + 3;
        Some(tag[a..].split('"').next()?.to_string())
    };
    (
        attr("x").unwrap().parse().unwrap(),
        attr("y").unwrap().parse().unwrap(),
        attr("transform").unwrap_or_default(),
    )
}

#[test]
fn theta_labels_surround_the_circle() {
    let s = svg(CoordPolar::new());
    // Four theta labels at four different places around the centre.
    let pts: Vec<(f64, f64)> = ["a", "b", "c", "d"]
        .iter()
        .map(|l| {
            let t = text(&s, l);
            (t.0, t.1)
        })
        .collect();
    let (cx, cy) = (
        pts.iter().map(|p| p.0).sum::<f64>() / 4.0,
        pts.iter().map(|p| p.1).sum::<f64>() / 4.0,
    );
    let r: Vec<f64> = pts
        .iter()
        .map(|p| ((p.0 - cx).powi(2) + (p.1 - cy).powi(2)).sqrt())
        .collect();
    assert!(r.iter().all(|&d| d > 100.0), "labels on a ring: {pts:?}");
    // Rings drawn as polylines with many points.
    assert!(s.matches("<polyline").count() >= 4);
    // r labels left of the panel.
    let (x20, ..) = text(&s, "20");
    assert!(x20 < pts.iter().map(|p| p.0).fold(f64::MAX, f64::min));
}

#[test]
fn radial_options() {
    let rot = svg(CoordPolar::new().rotate_angle(true));
    assert!(text(&rot, "b").2.contains("rotate("), "turned theta labels");
    let inside = svg(CoordPolar::new().r_axis_inside(true));
    let outside = svg(CoordPolar::new());
    assert!(
        text(&inside, "20").0 > text(&outside, "20").0,
        "r labels along the spoke"
    );
    // reverse(theta): "b" mirrors to the other side of the circle.
    let rev = svg(CoordPolar::new().reverse(true, false));
    let (b0, b1) = (text(&outside, "b").0, text(&rev, "b").0);
    let mid = 200.0;
    assert!((b0 - mid) * (b1 - mid) < 0.0, "{b0} vs {b1}");
    // reverse(r): the tallest bar now ends near the centre.
    let rr = svg(CoordPolar::new().reverse(false, true));
    assert_ne!(rr, outside);
}
