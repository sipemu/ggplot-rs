//! #411: `coord_cartesian(expand = …)` per side — a side without expansion
//! puts the data extreme on that panel edge.

use ggplot_rs::coord::cartesian::CoordCartesian;
use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    vec![
        ("x".into(), (0..5).map(|i| Value::Float(i as f64)).collect()),
        (
            "y".into(),
            (0..5).map(|i| Value::Float((i * i) as f64)).collect(),
        ),
    ]
}

type Box4 = (f64, f64, f64, f64);

/// (panel x0, y0, x1, y1) and the points' (min cx, min cy, max cx, max cy).
fn geometry(c: CoordCartesian) -> (Box4, Box4) {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .coord_cartesian_with(c)
        .render_svg_native_with_size(400, 300)
        .unwrap();
    let attr = |t: &str, n: &str| -> f64 {
        let a = t.find(&format!(" {n}=\"")).unwrap() + n.len() + 3;
        t[a..].split('"').next().unwrap().parse().unwrap()
    };
    // The second <rect> is the panel background (first = plot background).
    let r = svg.split("<rect").nth(2).unwrap();
    let (x, y, w, h) = (
        attr(r, "x"),
        attr(r, "y"),
        attr(r, "width"),
        attr(r, "height"),
    );
    let pts: Vec<(f64, f64)> = svg
        .split("<circle")
        .skip(1)
        .map(|t| (attr(t, "cx"), attr(t, "cy")))
        .collect();
    let f = |g: fn(f64, f64) -> f64, k: usize, init: f64| {
        pts.iter()
            .map(|p| if k == 0 { p.0 } else { p.1 })
            .fold(init, g)
    };
    (
        (x, y, x + w, y + h),
        (
            f(f64::min, 0, f64::MAX),
            f(f64::min, 1, f64::MAX),
            f(f64::max, 0, f64::MIN),
            f(f64::max, 1, f64::MIN),
        ),
    )
}

#[test]
fn no_expand_puts_data_on_every_edge() {
    let (panel, pts) = geometry(CoordCartesian::new().expand(false, false, false, false));
    assert!((pts.0 - panel.0).abs() < 0.05, "{panel:?} {pts:?}");
    assert!((pts.1 - panel.1).abs() < 0.05, "top");
    assert!((pts.2 - panel.2).abs() < 0.05, "right");
    assert!((pts.3 - panel.3).abs() < 0.05, "bottom");
}

#[test]
fn per_side_expand_keeps_the_others() {
    // Only the left side flush.
    let (panel, pts) = geometry(CoordCartesian::new().expand(true, true, true, false));
    assert!((pts.0 - panel.0).abs() < 0.05);
    assert!(panel.2 - pts.2 > 5.0, "right keeps its padding");
    assert!(pts.1 - panel.1 > 5.0 && panel.3 - pts.3 > 5.0);
    // Reversed x: the left side is the data's upper end.
    let (panel, pts) = geometry(
        CoordCartesian::new()
            .reverse(true, false)
            .expand(true, true, true, false),
    );
    assert!((pts.0 - panel.0).abs() < 0.05, "{panel:?} {pts:?}");
    assert!(panel.2 - pts.2 > 5.0);
}
