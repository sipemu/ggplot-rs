//! #408: `scale_x_binned` — positions binned before the stat, so
//! `geom_bar()` counts per bin (a histogram) and the axis breaks are the
//! bin edges.

use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    // Round bins of 2.5 over 0.5–9: (0, 2.5] holds 4 values, (2.5, 5] 4, …
    let xs = [0.5, 1.0, 1.5, 2.5, 3.0, 3.5, 4.0, 5.0, 7.5, 9.0];
    vec![("x".into(), xs.iter().map(|&v| Value::Float(v)).collect())]
}

#[test]
fn bar_on_binned_x_counts_per_bin() {
    let built = GGPlot::new(data())
        .aes(Aes::new().x("x"))
        .geom_bar()
        .scale_x_binned(5)
        .build();
    let d = &built.layers[0].data;
    let xs: Vec<f64> = d
        .column("x")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();
    let counts: Vec<f64> = d
        .column("count")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();
    let mut pairs: Vec<(f64, f64)> = xs.into_iter().zip(counts).collect();
    pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
    // Edges 0, 2.5, 5, 7.5, 10 → midpoints; right-closed (2.5 and 5 fall
    // in the lower bin, 7.5 too).
    assert_eq!(
        pairs,
        vec![(1.25, 4.0), (3.75, 4.0), (6.25, 1.0), (8.75, 1.0)]
    );
    let labels: Vec<String> = built
        .scales
        .get(&Aesthetic::X)
        .unwrap()
        .breaks()
        .into_iter()
        .map(|(_, l)| l)
        .collect();
    assert_eq!(labels, ["0", "2.5", "5", "7.5", "10"]);
}

#[test]
fn points_snap_to_bin_centres_and_render() {
    let svg = GGPlot::new(vec![
        (
            "x".to_string(),
            vec![Value::Float(0.3), Value::Float(0.6), Value::Float(9.9)],
        ),
        (
            "y".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
    ])
    .aes(Aes::new().x("x").y("y"))
    .geom_point()
    .scale_x_binned(5)
    .render_svg_native_with_size(400, 300)
    .unwrap();
    let cx: Vec<String> = svg
        .split("<circle")
        .skip(1)
        .map(|t| {
            let a = t.find(" cx=\"").unwrap() + 5;
            t[a..].split('"').next().unwrap().to_string()
        })
        .collect();
    assert_eq!(cx[0], cx[1], "0.3 and 0.6 share a bin");
    assert_ne!(cx[0], cx[2]);
}

#[test]
fn size_binned_legend_shows_bins() {
    let n = 20;
    let svg = GGPlot::new(vec![
        (
            "x".to_string(),
            (0..n).map(|i| Value::Float(i as f64)).collect(),
        ),
        (
            "y".to_string(),
            (0..n).map(|i| Value::Float(i as f64)).collect(),
        ),
        (
            "s".to_string(),
            (0..n).map(|i| Value::Float(i as f64 * 5.0)).collect(),
        ),
    ])
    .aes(Aes::new().x("x").y("y").size("s"))
    .geom_point()
    .scale_size_binned(5)
    .render_svg_native_with_size(400, 300)
    .unwrap();
    // s spans 0–95: bins 0–25, …, 75–100, labelled at the inner edges
    // by the default guide_bins (#409).
    assert!(svg.contains(">25<") && svg.contains(">75<"), "{svg}");
    let radii: std::collections::BTreeSet<String> = svg
        .split("<circle")
        .skip(1)
        .map(|t| {
            let a = t.find(" r=\"").unwrap() + 4;
            t[a..].split('"').next().unwrap().to_string()
        })
        .collect();
    assert_eq!(radii.len(), 4, "one size per bin: {radii:?}");
}
