//! `geom_sf` through the plotters-backed renderer (feature `sf` + the
//! default `plotters` feature): exercises `PlottersAdapter::
//! draw_polygon_rings` (#178) — the SVG backend's own hole/multipolygon
//! tests (`tests/sf.rs`) go through the self-contained `SvgBackend`
//! instead, which never touches this path.

#![cfg(all(feature = "sf", feature = "plotters"))]

use ggplot_rs::data::{DataFrame, Value};
use ggplot_rs::prelude::*;

fn strs(v: &[&str]) -> Vec<Value> {
    v.iter().map(|s| Value::Str(s.to_string())).collect()
}

const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];

#[test]
fn polygon_with_a_hole_renders_through_plotters() {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&["POLYGON ((0 0, 10 0, 10 10, 0 10, 0 0), (3 3, 7 3, 7 7, 3 7, 3 3))"]),
    );
    let png = GGPlot::new(df)
        .aes(Aes::new())
        .geom_sf()
        .render_png()
        .expect("plotters-backed sf render");
    assert!(png.len() > 100, "a real PNG, not an empty/error buffer");
    assert_eq!(&png[0..8], &PNG_MAGIC, "PNG magic bytes");
}

#[test]
fn multipolygon_with_a_hole_renders_through_plotters() {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&[
            "MULTIPOLYGON (((0 0, 4 0, 4 4, 0 4, 0 0)), ((10 0, 14 0, 14 4, 10 4, 10 0), (11 1, 13 1, 13 3, 11 3, 11 1)))",
        ]),
    );
    let png = GGPlot::new(df)
        .aes(Aes::new())
        .geom_sf()
        .render_png()
        .expect("plotters-backed sf render");
    assert!(png.len() > 100);
    assert_eq!(&png[0..8], &PNG_MAGIC);
}

/// `line_width: 0.0` turns the stroke branch off (vs. the default `0.5`,
/// exercised by the two tests above) — both branches of
/// `PlottersAdapter::draw_polygon_rings` get a run.
#[test]
fn a_hole_with_no_stroke_still_renders() {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        strs(&["POLYGON ((0 0, 10 0, 10 10, 0 10, 0 0), (3 3, 7 3, 7 7, 3 7, 3 3))"]),
    );
    let png = GGPlot::new(df)
        .aes(Aes::new())
        .geom_sf_with(ggplot_rs::geom::sf::GeomSf {
            line_width: 0.0,
            ..Default::default()
        })
        .render_png()
        .expect("plotters-backed sf render, no stroke");
    assert!(png.len() > 100);
}
