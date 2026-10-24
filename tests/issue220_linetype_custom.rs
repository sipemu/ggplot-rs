//! #220: `Linetype::Custom(Vec<(dash, gap)>)` — caller-supplied dash
//! patterns, rendered through the same `stroke-dasharray` path as the named
//! linetypes and mappable per group through the linetype scale. Driving use
//! case: round-dot "flow" edges (`stroke-dasharray="0 10"` + round caps) in
//! DataZooDE/anofox-visualization#421.

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::geom::curve::GeomCurve;
use ggplot_rs::geom::line::GeomLine;
use ggplot_rs::geom::path::GeomPath;
use ggplot_rs::geom::segment::GeomSegment;
use ggplot_rs::render::backend::{LineEnd, Linetype};
use ggplot_rs::theme::presets::theme_void;
use ggplot_rs::GGPlot;

fn fcol(name: &str, v: &[f64]) -> (String, Vec<Value>) {
    (name.into(), v.iter().map(|&x| Value::Float(x)).collect())
}

fn scol(name: &str, v: &[&str]) -> (String, Vec<Value>) {
    (
        name.into(),
        v.iter().map(|&s| Value::Str(s.into())).collect(),
    )
}

/// Two groups `a`/`b`, two points each.
fn two_groups() -> Vec<(String, Vec<Value>)> {
    vec![
        fcol("x", &[0.0, 1.0, 0.0, 1.0]),
        fcol("y", &[0.0, 1.0, 1.0, 0.0]),
        scol("g", &["a", "a", "b", "b"]),
    ]
}

fn path_svg(values: Vec<(&str, Linetype)>, lineend: LineEnd) -> String {
    GGPlot::new(two_groups())
        .aes(Aes::new().x("x").y("y").linetype("g"))
        .geom_path_with(GeomPath::default().lineend(lineend))
        .scale_linetype_manual(values)
        .theme(theme_void())
        .render_svg_native_with_size(200, 200)
        .unwrap()
}

/// Every data-mark `<polyline>` element in `svg` (legend keys excluded).
fn polylines(svg: &str) -> Vec<&str> {
    svg.split("<polyline")
        .skip(1)
        .map(|s| &s[..s.find("/>").unwrap_or(s.len())])
        .filter(|s| s.contains("gg-mark"))
        .collect()
}

/// How many data polylines carry `attr`.
fn marks_with(svg: &str, attr: &str) -> usize {
    tag_marks_with(svg, "polyline", attr)
}

/// How many data-mark `<tag>` elements carry `attr`.
fn tag_marks_with(svg: &str, tag: &str, attr: &str) -> usize {
    svg.split(&format!("<{tag} "))
        .skip(1)
        .map(|s| &s[..s.find("/>").unwrap_or(s.len())])
        .filter(|s| s.contains("gg-mark") && s.contains(attr))
        .count()
}

#[test]
fn custom_pattern_round_trips_into_stroke_dasharray() {
    let lt = Linetype::Custom(vec![(5.0, 2.5), (1.0, 3.0)]);
    let svg = path_svg(vec![("a", lt.clone()), ("b", lt)], LineEnd::Butt);
    assert_eq!(
        marks_with(&svg, "stroke-dasharray=\"5,2.5,1,3\""),
        2,
        "{svg}"
    );
}

#[test]
fn zero_dash_with_round_caps_draws_dots() {
    let svg = path_svg(
        vec![
            ("a", Linetype::Custom(vec![(0.0, 10.0)])),
            ("b", Linetype::Custom(vec![(0.0, 10.0)])),
        ],
        LineEnd::Round,
    );
    let lines = polylines(&svg);
    assert_eq!(lines.len(), 2, "{svg}");
    for l in lines {
        // A zero-length dash every 10 px: with a round cap each dash is a
        // dot of the stroke width, the gaps keep it from being a line.
        assert!(l.contains("stroke-dasharray=\"0,10\""), "{l}");
        assert!(l.contains("stroke-linecap=\"round\""), "{l}");
    }
}

#[test]
fn per_group_mapping_gives_each_group_its_own_custom_pattern() {
    let svg = path_svg(
        vec![
            ("a", Linetype::Custom(vec![(0.0, 10.0)])),
            ("b", Linetype::Custom(vec![(12.0, 4.0)])),
        ],
        LineEnd::Round,
    );
    // Two groups -> two data polylines, one per custom pattern (the legend
    // keys draw the same patterns, but are not data marks).
    assert_eq!(polylines(&svg).len(), 2, "{svg}");
    assert_eq!(marks_with(&svg, "stroke-dasharray=\"0,10\""), 1, "{svg}");
    assert_eq!(marks_with(&svg, "stroke-dasharray=\"12,4\""), 1, "{svg}");
}

#[test]
fn custom_mixes_with_named_linetypes_in_one_manual_scale() {
    let svg = path_svg(
        vec![
            ("a", Linetype::Dashed),
            ("b", Linetype::Custom(vec![(0.0, 6.0)])),
        ],
        LineEnd::Round,
    );
    assert_eq!(marks_with(&svg, "stroke-dasharray=\"6,3\""), 1, "{svg}");
    assert_eq!(marks_with(&svg, "stroke-dasharray=\"0,6\""), 1, "{svg}");
}

#[test]
fn custom_works_on_line_segment_and_curve_too() {
    let values = || {
        vec![
            ("a", Linetype::Custom(vec![(0.0, 8.0)])),
            ("b", Linetype::Custom(vec![(4.0, 4.0)])),
        ]
    };
    let line = GGPlot::new(two_groups())
        .aes(Aes::new().x("x").y("y").linetype("g"))
        .geom_line_with(GeomLine::default().lineend(LineEnd::Round))
        .scale_linetype_manual(values())
        .theme(theme_void())
        .render_svg_native_with_size(200, 200)
        .unwrap();
    assert_eq!(marks_with(&line, "stroke-dasharray=\"0,8\""), 1, "{line}");
    assert_eq!(marks_with(&line, "stroke-dasharray=\"4,4\""), 1, "{line}");

    let seg_data = || {
        vec![
            fcol("x", &[0.0, 0.0]),
            fcol("y", &[0.0, 1.0]),
            fcol("xend", &[1.0, 1.0]),
            fcol("yend", &[0.0, 1.0]),
            scol("g", &["a", "b"]),
        ]
    };
    let seg = GGPlot::new(seg_data())
        .aes(
            Aes::new()
                .x("x")
                .y("y")
                .xend("xend")
                .yend("yend")
                .linetype("g"),
        )
        .geom_segment_with(GeomSegment::default().lineend(LineEnd::Round))
        .scale_linetype_manual(values())
        .theme(theme_void())
        .render_svg_native_with_size(200, 200)
        .unwrap();
    assert_eq!(marks_with(&seg, "stroke-dasharray=\"0,8\""), 1, "{seg}");
    assert_eq!(marks_with(&seg, "stroke-dasharray=\"4,4\""), 1, "{seg}");

    let curve = GGPlot::new(seg_data())
        .aes(
            Aes::new()
                .x("x")
                .y("y")
                .xend("xend")
                .yend("yend")
                .linetype("g"),
        )
        .geom_curve_with(GeomCurve::default().lineend(LineEnd::Round))
        .scale_linetype_manual(values())
        .theme(theme_void())
        .render_svg_native_with_size(200, 200)
        .unwrap();
    assert_eq!(
        tag_marks_with(&curve, "path", "stroke-dasharray=\"0,8\""),
        1,
        "{curve}"
    );
    assert_eq!(
        tag_marks_with(&curve, "path", "stroke-dasharray=\"4,4\""),
        1,
        "{curve}"
    );
}

#[test]
fn dash_pattern_sanitises_degenerate_input() {
    // Empty, all-zero, zero-gap and all-non-finite patterns draw solid.
    assert!(Linetype::Custom(vec![]).dash_pattern().is_empty());
    assert!(Linetype::Custom(vec![(0.0, 0.0)]).dash_pattern().is_empty());
    assert!(Linetype::Custom(vec![(5.0, 0.0)]).dash_pattern().is_empty());
    assert!(
        Linetype::Custom(vec![(f64::NAN, 3.0), (2.0, f64::INFINITY)])
            .dash_pattern()
            .is_empty()
    );
    assert!(Linetype::Custom(vec![(-4.0, -2.0)])
        .dash_pattern()
        .is_empty());
    // Non-finite pairs are dropped, negatives clamped to 0.
    assert_eq!(
        Linetype::Custom(vec![(f64::NAN, 1.0), (-1.0, 6.0), (3.0, -2.0)]).dash_pattern(),
        vec![(0.0, 6.0), (3.0, 0.0)]
    );
    // Named variants are unchanged.
    assert_eq!(Linetype::Dashed.dash_pattern(), vec![(6.0, 3.0)]);
    assert!(Linetype::Solid.dash_pattern().is_empty());
}

#[test]
fn degenerate_custom_patterns_render_solid_without_panicking() {
    for lt in [
        Linetype::Custom(vec![]),
        Linetype::Custom(vec![(0.0, 0.0)]),
        Linetype::Custom(vec![(f64::NAN, f64::NAN)]),
        Linetype::Custom(vec![(f64::NEG_INFINITY, 2.0)]),
        Linetype::Custom(vec![(-3.0, -3.0)]),
    ] {
        let svg = path_svg(vec![("a", lt.clone()), ("b", lt.clone())], LineEnd::Round);
        assert_eq!(polylines(&svg).len(), 2, "{lt:?}: {svg}");
        assert!(!svg.contains("stroke-dasharray"), "{lt:?}: {svg}");
    }
    // A partly-bad pattern keeps only its usable, non-negative part.
    let svg = path_svg(
        vec![
            ("a", Linetype::Custom(vec![(f64::NAN, 1.0), (-2.0, 9.0)])),
            ("b", Linetype::Solid),
        ],
        LineEnd::Round,
    );
    assert_eq!(marks_with(&svg, "stroke-dasharray=\"0,9\""), 1, "{svg}");
}

/// The raster (plotters) dashed stroker must terminate on zero-length
/// dashes and degenerate cycles.
#[cfg(feature = "plotters")]
#[test]
fn raster_dashing_handles_custom_and_degenerate_patterns() {
    for lt in [
        Linetype::Custom(vec![(0.0, 10.0)]),
        Linetype::Custom(vec![(0.0, 0.0)]),
        Linetype::Custom(vec![(f64::NAN, 1.0)]),
        Linetype::Custom(vec![(0.0, 0.0), (3.0, 2.0)]),
    ] {
        let png = GGPlot::new(two_groups())
            .aes(Aes::new().x("x").y("y").linetype("g"))
            .geom_path_with(GeomPath::default().lineend(LineEnd::Round))
            .scale_linetype_manual(vec![("a", lt.clone()), ("b", lt)])
            .render_png_with_size(120, 120);
        assert!(png.is_ok());
    }
}
