//! `annotation_attribution()`'s stacked multi-line lines (#250): a
//! `GeomAttribution` can hold several lines, each its own run list
//! (`.runs(..)` sets row 0, each `.line(..)` call appends another row),
//! drawn stacked from the corner — bottom corners stack upward, top
//! corners downward — so a data licence note needing several lines (e.g.
//! BKG's dl-de/by-2-0 note for German boundaries, DataZooDE/
//! anofox-visualization#420) no longer has to overlap itself.
#![cfg(feature = "sf")]

use ggplot_rs::data::{DataFrame, Value};
use ggplot_rs::geom::annotation_scale::{GeomAttribution, MapCorner};
use ggplot_rs::prelude::*;

const W: u32 = 300;
const H: u32 = 300;

fn df() -> DataFrame {
    let mut df = DataFrame::new();
    df.add_column(
        "geometry".into(),
        vec![Value::Str("POLYGON ((0 0, 1 0, 1 1, 0 1, 0 0))".into())],
    );
    df
}

/// Every `<text ... data-geom="attribution" ...>` element's `y`, in
/// document order. No regex dependency: attribute values never contain
/// `>`, so each opening tag ends at its first `>`.
fn attribution_ys(svg: &str) -> Vec<f64> {
    let mut ys = Vec::new();
    for tag_start in svg.match_indices("<text ").map(|(i, _)| i) {
        let tag_end = svg[tag_start..].find('>').map(|i| tag_start + i).unwrap();
        let tag = &svg[tag_start..tag_end];
        if !tag.contains("data-geom=\"attribution\"") {
            continue;
        }
        let y_attr = " y=\"";
        let y_start = tag.find(y_attr).expect("text tag has a y attribute") + y_attr.len();
        let y_end = tag[y_start..].find('"').map(|i| y_start + i).unwrap();
        ys.push(tag[y_start..y_end].parse::<f64>().unwrap());
    }
    ys
}

fn render(geom: GeomAttribution) -> String {
    GGPlot::new(df())
        .aes(Aes::new())
        .geom_sf()
        .annotation_attribution_with(geom)
        .render_svg_native_with_size(W, H)
        .unwrap()
}

#[test]
fn single_line_is_still_exactly_one_text_element_unchanged() {
    // No `.line(..)` call: row 0 alone, same path as before #250.
    let svg = render(GeomAttribution::new("Made with Natural Earth"));
    let ys = attribution_ys(&svg);
    assert_eq!(ys.len(), 1, "{svg}");
    assert!(svg.contains("Made with Natural Earth"));
}

#[test]
fn two_lines_stack_upward_at_bottom_right_without_overlap() {
    let geom = GeomAttribution::new("ignored")
        .corner(MapCorner::BottomRight)
        .runs(vec![("line one", None)])
        .line(vec![("line two", None)]);
    let svg = render(geom);
    let ys = attribution_ys(&svg);
    assert_eq!(ys.len(), 2, "{svg}");
    // Row 0 (closest to the bottom edge) has the larger y; row 1 sits
    // above it (smaller y) — strictly monotone, never equal.
    assert!(ys[0] > ys[1], "ys = {ys:?}");
    let gap = ys[0] - ys[1];
    assert!(gap >= 7.5 * 1.3 - 0.01, "gap too small: {gap} ({ys:?})");
    for y in &ys {
        assert!((0.0..=H as f64).contains(y), "y out of canvas: {y}");
    }
    assert!(svg.contains("line one"));
    assert!(svg.contains("line two"));
}

#[test]
fn three_lines_stack_upward_at_bottom_right_monotone_and_on_canvas() {
    let geom = GeomAttribution::new("ignored")
        .corner(MapCorner::BottomRight)
        .runs(vec![
            ("© BKG ", None),
            ("(2026)", Some("https://www.bkg.bund.de")),
        ])
        .line(vec![(
            "https://example.com/src",
            Some("https://example.com/src"),
        )])
        .line(vec![("EuroGeographics NUTS", None)]);
    let svg = render(geom);
    let ys = attribution_ys(&svg);
    assert_eq!(ys.len(), 3, "{svg}");
    // Strictly decreasing y (each row further from the bottom edge, i.e.
    // visually higher) and every row stays on the canvas.
    assert!(ys[0] > ys[1] && ys[1] > ys[2], "ys = {ys:?}");
    for y in &ys {
        assert!((0.0..=H as f64).contains(y), "y out of canvas: {y}");
    }
    // Each line's own link survived independently.
    assert!(svg.contains("<a href=\"https://www.bkg.bund.de\""), "{svg}");
    assert!(svg.contains("<a href=\"https://example.com/src\""), "{svg}");
    assert!(svg.contains("EuroGeographics NUTS"));
}

#[test]
fn two_lines_stack_downward_at_top_left_without_overlap() {
    let geom = GeomAttribution::new("ignored")
        .corner(MapCorner::TopLeft)
        .runs(vec![("top line", None)])
        .line(vec![("second line", None)]);
    let svg = render(geom);
    let ys = attribution_ys(&svg);
    assert_eq!(ys.len(), 2, "{svg}");
    // Row 0 (closest to the top edge) has the smaller y; row 1 sits
    // below it (larger y), opposite direction from the bottom corners.
    assert!(ys[0] < ys[1], "ys = {ys:?}");
    for y in &ys {
        assert!((0.0..=H as f64).contains(y), "y out of canvas: {y}");
    }
}

#[test]
fn three_lines_stack_downward_at_top_left_monotone_and_on_canvas() {
    let geom = GeomAttribution::new("ignored")
        .corner(MapCorner::TopLeft)
        .runs(vec![("one", None)])
        .line(vec![("two", Some("https://example.com/two"))])
        .line(vec![("three", None)]);
    let svg = render(geom);
    let ys = attribution_ys(&svg);
    assert_eq!(ys.len(), 3, "{svg}");
    assert!(ys[0] < ys[1] && ys[1] < ys[2], "ys = {ys:?}");
    for y in &ys {
        assert!((0.0..=H as f64).contains(y), "y out of canvas: {y}");
    }
    assert!(svg.contains("<a href=\"https://example.com/two\""), "{svg}");
}

#[test]
fn an_empty_extra_line_is_skipped_without_shifting_the_count() {
    // An all-empty-text extra line draws nothing (same "skip" rule row 0
    // already has) but doesn't panic or desync the row index for a
    // subsequent non-empty line's position.
    let geom = GeomAttribution::new("ignored")
        .corner(MapCorner::BottomRight)
        .runs(vec![("first", None)])
        .line(vec![("", None)])
        .line(vec![("third", None)]);
    let svg = render(geom);
    let ys = attribution_ys(&svg);
    assert_eq!(ys.len(), 2, "{svg}");
    assert!(svg.contains("first"));
    assert!(svg.contains("third"));
}
