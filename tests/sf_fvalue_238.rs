//! `geom_sf` writes `data-fvalue` (B2-4, the mapped size/fill's scale
//! formatter applied to the row's raw value) the same way `GeomPoint`/
//! `GeomHex` already do (feature `sf`, #238). `fill` is `GeomSf`'s primary
//! data channel (a choropleth's measure, or a graduated symbol's colour), so
//! it wins when mapped; a `size`-only graduated-symbol map (no fill measure
//! — DataZooDE/anofox-visualization#475's motivating case) falls back to
//! `size`'s own formatter.
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

fn with_column(mut df: DataFrame, name: &str, values: &[f64]) -> DataFrame {
    df.add_column(
        name.to_string(),
        values.iter().map(|&v| Value::Float(v)).collect(),
    );
    df
}

#[test]
fn fill_only_fvalue_comes_from_the_fill_aesthetic() {
    let df = with_column(points(&[(0.0, 0.0), (1.0, 1.0)]), "fill", &[42.5, 10.0]);
    let svg = GGPlot::new(df)
        .aes(Aes::new().fill("fill"))
        .geom_sf()
        .scale_fill(
            ScaleColorGradient2::new(Aesthetic::Fill).with_value_formatter(|v| format!("{v:.1}%")),
        )
        .svg_formatted_values(true)
        .render_svg_native_with_size(400, 400)
        .expect("render");
    assert!(svg.contains("data-fvalue=\"42.5%\""), "{svg}");
    assert!(svg.contains("data-fvalue=\"10.0%\""), "{svg}");
}

#[test]
fn size_only_fvalue_comes_from_the_size_aesthetic() {
    // No `fill` column at all — `GeomSf::draw` has nothing to read for it,
    // so the mapped `size` (a graduated-symbol measure, e.g. a population
    // count) is the only channel left for `data-fvalue`.
    let df = with_column(points(&[(0.0, 0.0), (1.0, 1.0)]), "size", &[1234.5, 1.0]);
    let svg = GGPlot::new(df)
        .aes(Aes::new().size("size"))
        .geom_sf()
        .scale_size(ScaleSizeContinuous::new())
        .locale(Locale::from_tag("de-DE"))
        .svg_formatted_values(true)
        .render_svg_native_with_size(400, 400)
        .expect("render");
    assert!(svg.contains("data-fvalue=\"1.234,5\""), "{svg}");
}

#[test]
fn fill_and_size_fvalue_prefers_fill() {
    let df = with_column(
        with_column(points(&[(0.0, 0.0), (1.0, 1.0)]), "fill", &[42.5, 10.0]),
        "size",
        &[999.0, 1.0],
    );
    let svg = GGPlot::new(df)
        .aes(Aes::new().fill("fill").size("size"))
        .geom_sf()
        .scale_fill(
            ScaleColorGradient2::new(Aesthetic::Fill).with_value_formatter(|v| format!("{v:.1}%")),
        )
        .scale_size_area(20.0)
        .svg_formatted_values(true)
        .render_svg_native_with_size(400, 400)
        .expect("render");
    assert!(
        svg.contains("data-fvalue=\"42.5%\""),
        "fill's formatted value wins when both are mapped: {svg}"
    );
    assert!(
        !svg.contains("data-fvalue=\"999\""),
        "size's raw/formatted value must not leak through when fill is mapped: {svg}"
    );
}

#[test]
fn no_fvalue_without_a_mapped_fill_or_size() {
    let df = points(&[(0.0, 0.0), (1.0, 1.0)]);
    let svg = GGPlot::new(df)
        .aes(Aes::new())
        .geom_sf()
        .svg_formatted_values(true)
        .render_svg_native_with_size(400, 400)
        .expect("render");
    assert!(!svg.contains("data-fvalue"), "{svg}");
}
