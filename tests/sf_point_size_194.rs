//! `geom_sf`'s `size` aesthetic on `Point`/`MultiPoint` rows (feature `sf`,
//! #194): graduated symbols, scaled per row and drawn largest-first so a
//! big circle never hides a smaller one at the same spot.
#![cfg(feature = "sf")]

use ggplot_rs::data::{DataFrame, Value};
use ggplot_rs::prelude::*;

fn points(coords: &[(f64, f64)]) -> DataFrame {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        coords
            .iter()
            .map(|(x, y)| Value::Str(format!("POINT ({x} {y})")))
            .collect(),
    );
    df
}

fn with_size(mut df: DataFrame, sizes: &[f64]) -> DataFrame {
    df.add_column(
        "size".into(),
        sizes.iter().map(|&s| Value::Float(s)).collect(),
    );
    df
}

/// Each `data-geom="sf"` circle's radius, in document order — i.e. the
/// point marks `GeomSf::draw` emits, not a size-legend swatch (no
/// `data-geom` attribute) or any other element whose attribute name
/// happens to end in `r` (`text-anchor="..."`, `dominant-baseline="..."`).
fn circle_radii(svg: &str) -> Vec<f64> {
    svg.split("<circle")
        .skip(1)
        .filter(|tag| tag.contains("data-geom=\"sf\""))
        .map(|tag| {
            let after = tag.split_once(" r=\"").unwrap().1;
            after.split_once('"').unwrap().0.parse::<f64>().unwrap()
        })
        .collect()
}

#[test]
fn a_mapped_size_scales_each_points_radius() {
    let df = with_size(points(&[(0.0, 0.0), (10.0, 10.0)]), &[1.0, 100.0]);
    let svg = GGPlot::new(df)
        .aes(Aes::new().size("size"))
        .geom_sf()
        .scale_size_area(20.0)
        .render_svg_native_with_size(400, 400)
        .expect("render");
    let radii = circle_radii(&svg);
    assert_eq!(radii.len(), 2, "{svg}");
    // scale_size_area: radius ∝ sqrt(value) — the 100x row draws ~10x the
    // 1x row's radius, not merely "somewhat bigger".
    let ratio =
        radii.iter().cloned().fold(0.0, f64::max) / radii.iter().cloned().fold(f64::MAX, f64::min);
    assert!((ratio - 10.0).abs() < 0.5, "{radii:?}");
}

#[test]
fn no_size_aesthetic_at_all_falls_back_to_point_size() {
    // No "size" column in the data at all — `aes()` has nothing to map and
    // `GeomSf::draw` has no `size` column to read, so every row keeps the
    // unmapped `point_size` default.
    let df = points(&[(0.0, 0.0), (10.0, 10.0)]);
    let svg = GGPlot::new(df)
        .aes(Aes::new())
        .geom_sf()
        .render_svg_native_with_size(400, 400)
        .expect("render");
    let radii = circle_radii(&svg);
    assert_eq!(radii.len(), 2);
    assert!((radii[0] - radii[1]).abs() < 1e-9, "{radii:?}");
    assert!((radii[0] - ggplot_rs::geom::sf::GeomSf::default().point_size).abs() < 1e-9);
}

#[test]
fn largest_point_draws_first_regardless_of_row_order() {
    // Row order is small-then-large; the SVG must still emit the larger
    // circle first, so the smaller one paints on top and stays visible.
    let df = with_size(points(&[(0.0, 0.0), (0.0, 0.0)]), &[1.0, 100.0]);
    let svg = GGPlot::new(df)
        .aes(Aes::new().size("size"))
        .geom_sf()
        .scale_size_area(20.0)
        .render_svg_native_with_size(400, 400)
        .expect("render");
    let radii = circle_radii(&svg);
    assert_eq!(radii.len(), 2, "{svg}");
    assert!(radii[0] > radii[1], "largest first: {radii:?}");
}

#[test]
fn draw_order_is_stable_for_equal_sizes() {
    // Three rows, all the same size: order stays the original row order (a
    // `fill` column tags each so we can tell them apart via its tooltip).
    let mut df = with_size(
        points(&[(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]),
        &[5.0, 5.0, 5.0],
    );
    df.add_column(
        "fill".into(),
        vec![
            Value::Str("a".into()),
            Value::Str("b".into()),
            Value::Str("c".into()),
        ],
    );
    let svg = GGPlot::new(df)
        .aes(Aes::new().size("size").fill("fill"))
        .geom_sf()
        .render_svg_native_with_size(400, 400)
        .expect("render");
    let a = svg.find("<title>a</title>");
    let b = svg.find("<title>b</title>");
    let c = svg.find("<title>c</title>");
    assert!(a < b && b < c, "a,b,c tooltips keep row order: {svg}");
}
