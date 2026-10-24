//! #176's `corner_radius` through the plotters backend (feature `plotters`,
//! on by default): it has no native rounded-rect primitive, so
//! `plotters_backend::draw_rect`'s `corner_radius > 0.0` branch approximates
//! one as a polygon (`render::backend::rounded_rect_points`), fill via
//! `Polygon` and stroke via a closed `PathElement` loop — both exercised
//! here through `GGPlot::render_svg()` (the plotters-backed SVG path, not
//! the self-contained native one `svg_backend.rs` uses).
#![cfg(feature = "plotters")]

use ggplot_rs::data::Value;
use ggplot_rs::geom::rect::GeomRect;
use ggplot_rs::prelude::*;

fn one_box() -> Vec<(String, Vec<Value>)> {
    vec![
        ("xmin".to_string(), vec![Value::Float(0.0)]),
        ("xmax".to_string(), vec![Value::Float(10.0)]),
        ("ymin".to_string(), vec![Value::Float(0.0)]),
        ("ymax".to_string(), vec![Value::Float(10.0)]),
    ]
}

#[test]
fn rounded_rect_draws_a_polygon_not_a_plotters_rect() {
    let svg = GGPlot::new(one_box())
        .aes(
            Aes::new()
                .xmin("xmin")
                .xmax("xmax")
                .ymin("ymin")
                .ymax("ymax"),
        )
        .geom_rect_with(GeomRect::default().with_radius(3.0))
        .render_svg()
        .unwrap();
    assert_eq!(
        svg.matches("<polygon").count(),
        1,
        "the rounded box's fill should be one polygon: {svg}"
    );
    // Its stroke is a closed PathElement loop, which plotters' SVG backend
    // renders as a <polyline> — distinguish it from the plot's own
    // gridline polylines by its stroke colour (GeomRect's default #323232).
    assert!(
        svg.contains("<polyline") && svg.contains("stroke=\"#323232\""),
        "the rounded box's stroke should be a closed polyline: {svg}"
    );
}

#[test]
fn plain_rect_still_draws_a_plotters_rect_when_radius_is_zero() {
    let svg = GGPlot::new(one_box())
        .aes(
            Aes::new()
                .xmin("xmin")
                .xmax("xmax")
                .ymin("ymin")
                .ymax("ymax"),
        )
        .geom_rect_with(GeomRect::default())
        .render_svg()
        .unwrap();
    assert!(
        !svg.contains("<polygon"),
        "a radius-0 rect should not take the polygon-approximation path: {svg}"
    );
}
