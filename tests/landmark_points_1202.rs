//! anofox-visualization#1202: three depots (one fill category, large) among
//! a thousand customers (small) — largest-first put the depots at the
//! bottom, buried. Landmarks are drawn last, over a halo; a plain bubble map
//! keeps largest-first.
#![cfg(feature = "sf")]

use ggplot_rs::data::{DataFrame, Value};
use ggplot_rs::prelude::*;

fn map(depots: bool) -> String {
    let mut geom = Vec::new();
    let (mut kind, mut size) = (Vec::new(), Vec::new());
    for i in 0..1000 {
        let (x, y) = ((i % 40) as f64 * 0.25, (i / 40) as f64 * 0.4);
        geom.push(Value::Str(format!("POINT ({x} {y})")));
        kind.push(Value::Str("customer".into()));
        // a bubble map's own spread of sizes
        size.push(Value::Float(if depots {
            1.0
        } else {
            1.0 + (i % 7) as f64
        }));
    }
    if depots {
        for (x, y) in [(2.0, 2.0), (5.0, 5.0), (8.0, 8.0)] {
            geom.push(Value::Str(format!("POINT ({x} {y})")));
            kind.push(Value::Str("depot".into()));
            size.push(Value::Float(30.0));
        }
    }
    let mut df = DataFrame::new();
    df.add_column("geometry".into(), geom);
    df.add_column("kind".into(), kind);
    df.add_column("sz".into(), size);
    GGPlot::new(df)
        .aes(Aes::new().size("sz").fill("kind"))
        .geom_sf()
        .render_svg_native_with_size(500, 400)
        .unwrap()
}

/// Every `<circle`'s radius, in document (= drawing) order.
fn radii(svg: &str) -> Vec<f64> {
    svg.split("<circle")
        .skip(1)
        .filter_map(|c| {
            let k = c.find(" r=\"")? + 4;
            c[k..k + c[k..].find('"')?].parse().ok()
        })
        .collect()
}

#[test]
fn landmarks_are_drawn_last_over_a_halo() {
    let r = radii(&map(true));
    // the customers: the first 1000 circles' radius (all the same size)
    let small = r[0];
    let last_small = r.iter().rposition(|x| *x == small).unwrap();
    // each depot: a halo (radius + 1.5) right before it
    let halos: Vec<usize> = (0..r.len() - 1)
        .filter(|&i| r[i] > 2.0 * small && (r[i] - r[i + 1] - 1.5).abs() < 1e-6)
        .collect();
    assert_eq!(
        halos.len(),
        3,
        "three depots with a halo: {:?}",
        &r[r.len() - 14..]
    );
    assert!(
        halos.iter().all(|&i| i > last_small),
        "depots drawn after every customer"
    );
}

#[test]
fn a_bubble_map_keeps_largest_first() {
    let r = radii(&map(false));
    assert!(
        r.windows(2).take(950).all(|w| w[0] >= w[1]),
        "largest first"
    );
}
