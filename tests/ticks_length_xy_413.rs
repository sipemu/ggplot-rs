//! #413: `axis_ticks_length_x` / `_y` — per-axis tick lengths.

use ggplot_rs::prelude::*;

fn segs(svg: &str, vertical: bool) -> Vec<f64> {
    svg.split("<polyline points=\"")
        .skip(1)
        .filter_map(|t| {
            let p: Vec<(f64, f64)> = t
                .split('"')
                .next()?
                .split(' ')
                .filter_map(|xy| {
                    let (x, y) = xy.split_once(',')?;
                    Some((x.parse().ok()?, y.parse().ok()?))
                })
                .collect();
            if p.len() != 2 {
                return None;
            }
            let (dx, dy) = ((p[1].0 - p[0].0).abs(), (p[1].1 - p[0].1).abs());
            if vertical && dx < 1e-6 && dy < 20.0 {
                Some(dy)
            } else if !vertical && dy < 1e-6 && dx < 20.0 {
                Some(dx)
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn x_and_y_ticks_differ() {
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
    .theme_with(|t| {
        t.axis_ticks_length_x = Some(8.0);
        t.axis_ticks_length_y = Some(1.0);
    })
    .render_svg_native_with_size(400, 300)
    .unwrap();
    assert!(
        segs(&svg, true).iter().any(|l| (l - 8.0).abs() < 0.02),
        "x ticks 8"
    );
    assert!(
        segs(&svg, false).iter().any(|l| (l - 1.0).abs() < 0.02),
        "y ticks 1"
    );
}
