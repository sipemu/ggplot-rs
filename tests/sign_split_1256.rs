//! DataZooDE/anofox-visualization#1256: an area, density or line coloured
//! by the side of a threshold — red below, green above — cut at the exact
//! crossing, so the parts meet without a gap or an overlap.

use ggplot_rs::data::{DataFrame, Value};
use ggplot_rs::geom::area::GeomArea;
use ggplot_rs::geom::density::GeomDensity;
use ggplot_rs::geom::line::GeomLine;
use ggplot_rs::position::identity::PositionIdentity;
use ggplot_rs::prelude::*;

const RED: (u8, u8, u8) = (192, 58, 98);
const GREEN: (u8, u8, u8) = (31, 122, 82);

fn hex(c: (u8, u8, u8)) -> String {
    format!("#{:02X}{:02X}{:02X}", c.0, c.1, c.2)
}

fn series() -> DataFrame {
    let mut df = DataFrame::new();
    df.add_column("x".into(), (0..4).map(|i| Value::Float(i as f64)).collect());
    df.add_column(
        "y".into(),
        [-2.0, 2.0, 4.0, -4.0]
            .into_iter()
            .map(Value::Float)
            .collect(),
    );
    df
}

/// The `points` of every `<polygon>` / `d` of every `<path>` with that fill or stroke.
fn shapes(svg: &str, attr: &str, colour: (u8, u8, u8)) -> usize {
    svg.matches(&format!("{attr}=\"{}\"", hex(colour))).count()
}

#[test]
fn an_area_crossing_zero_has_a_red_and_a_green_part() {
    let svg = GGPlot::new(series())
        .aes(Aes::new().x("x").y("y"))
        .geom_area_with(GeomArea {
            split: Some(SignSplit::new(RED, GREEN)),
            ..Default::default()
        })
        .position(PositionIdentity)
        .render_svg_native_with_size(400, 300)
        .unwrap();
    // below, above, below: two red parts and one green
    assert_eq!(shapes(&svg, "fill", RED), 2, "{svg}");
    assert_eq!(shapes(&svg, "fill", GREEN), 1);
}

#[test]
fn a_plain_area_is_one_shape() {
    let svg = GGPlot::new(series())
        .aes(Aes::new().x("x").y("y"))
        .geom_area()
        .position(PositionIdentity)
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert_eq!(shapes(&svg, "fill", RED) + shapes(&svg, "fill", GREEN), 0);
}

#[test]
fn a_density_of_outcomes_is_red_left_of_zero_and_green_right() {
    let mut df = DataFrame::new();
    df.add_column(
        "x".into(),
        (0..200)
            .map(|i| Value::Float(((i * 37) % 200) as f64 / 20.0 - 4.0))
            .collect(),
    );
    let svg = GGPlot::new(df)
        .aes(Aes::new().x("x"))
        .geom_density_with(GeomDensity {
            split: Some(SignSplit::new(RED, GREEN).axis(SplitAxis::X)),
            ..Default::default()
        })
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert_eq!(shapes(&svg, "fill", RED), 1, "{svg}");
    assert_eq!(shapes(&svg, "fill", GREEN), 1);
}

#[test]
fn a_line_changes_colour_at_the_crossing() {
    let svg = GGPlot::new(series())
        .aes(Aes::new().x("x").y("y"))
        .geom_line_with(GeomLine {
            split: Some(SignSplit::new(RED, GREEN)),
            ..Default::default()
        })
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert_eq!(shapes(&svg, "stroke", RED), 2, "{svg}");
    assert_eq!(shapes(&svg, "stroke", GREEN), 1);
}
