//! B1-5 acceptance: `StatBinHex` on a fixed hex lattice + data-unit `GeomHex`.
#![cfg(feature = "sf")]

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;
use ggplot_rs::scale::ScaleSet;
use ggplot_rs::stat::binhex::{HexLattice, StatBinHex};
use ggplot_rs::stat::Stat;

fn grid_points() -> Vec<(String, Vec<Value>)> {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for i in 0..20 {
        for j in 0..20 {
            xs.push(Value::Float(i as f64 * 0.37));
            ys.push(Value::Float(j as f64 * 0.41));
        }
    }
    vec![("x".into(), xs), ("y".into(), ys)]
}

#[test]
fn hexbin_renders_with_no_nan_and_is_deterministic_over_5_runs() {
    let plot = || {
        GGPlot::new(grid_points())
            .aes(Aes::new().x("x").y("y"))
            .geom_hex()
    };
    let first = plot().render_svg_native_with_size(400, 400).unwrap();
    assert!(!first.contains("NaN"));
    assert!(first.contains("<polygon"));
    for _ in 0..5 {
        let again = plot().render_svg_native_with_size(400, 400).unwrap();
        assert_eq!(first, again);
    }
}

/// 4. The total is conserved (every point lands in exactly one cell).
#[test]
fn total_point_count_is_conserved() {
    let mut data = grid_points();
    let n = data[0].1.len();
    let stat = StatBinHex::new().bins_xy(10, 10);
    let scales = ScaleSet::default();
    let mut df = ggplot_rs::data::DataFrame::new();
    for (name, col) in data.drain(..) {
        df.add_column(name, col);
    }
    let result = stat.compute_group(&df, &scales);
    let total: f64 = result
        .column("count")
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .sum();
    assert_eq!(total, n as f64);
}

/// 5. Under coord_sf with square data, the hex edges are equal within 0.05 px.
#[test]
fn hex_edges_are_equal_under_coord_sf() {
    // A single point so the lattice yields exactly one cell, with a fixed
    // square (w == h) binwidth anchored at the origin.
    let svg = GGPlot::new(vec![
        ("x".into(), vec![Value::Float(0.0)]),
        ("y".into(), vec![Value::Float(0.0)]),
    ])
    .aes(Aes::new().x("x").y("y"))
    .geom_hex_with(GeomHex::default())
    .stat(StatBinHex::new().binwidth(1.0, 1.0).origin(0.0, 0.0))
    .coord_sf()
    .render_svg_native_with_size(400, 400)
    .unwrap();

    assert!(!svg.contains("NaN"));
    let poly = svg.split("<polygon").nth(1).expect("a hex polygon");
    let points_attr = poly
        .split("points=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let pts: Vec<(f64, f64)> = points_attr
        .split(' ')
        .map(|p| {
            let mut it = p.split(',');
            let x: f64 = it.next().unwrap().parse().unwrap();
            let y: f64 = it.next().unwrap().parse().unwrap();
            (x, y)
        })
        .collect();
    assert_eq!(pts.len(), 6);
    let edge = |a: (f64, f64), b: (f64, f64)| ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
    let edges: Vec<f64> = (0..6).map(|i| edge(pts[i], pts[(i + 1) % 6])).collect();
    let first = edges[0];
    for e in &edges {
        assert!((e - first).abs() < 0.05, "edges should be equal: {edges:?}");
    }
}

/// 6. `locate()` agrees with `compute_group` on 500 random points.
#[test]
fn locate_agrees_with_compute_group_on_random_points() {
    let mut state: u64 = 7;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % 20000) as f64 / 100.0 - 100.0
    };
    let xs: Vec<f64> = (0..500).map(|_| next()).collect();
    let ys: Vec<f64> = (0..500).map(|_| next()).collect();

    let mut df = ggplot_rs::data::DataFrame::new();
    df.add_column("x".into(), xs.iter().map(|&v| Value::Float(v)).collect());
    df.add_column("y".into(), ys.iter().map(|&v| Value::Float(v)).collect());

    let stat = StatBinHex::new().bins_xy(20, 20);
    let scales = ScaleSet::default();
    let lattice: HexLattice = stat.lattice(&xs, &ys);
    let result = stat.compute_group(&df, &scales);

    let produced: std::collections::HashSet<(i64, i64)> = result
        .column("q")
        .unwrap()
        .iter()
        .zip(result.column("r").unwrap().iter())
        .map(|(q, r)| (q.as_f64().unwrap() as i64, r.as_f64().unwrap() as i64))
        .collect();

    for i in 0..xs.len() {
        let qr = StatBinHex::locate(xs[i], ys[i], &lattice);
        assert!(produced.contains(&qr));
    }
}
