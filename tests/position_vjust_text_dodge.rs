//! B3-4: `position_stack`/`position_fill(vjust)`, and `geom_text`/
//! `geom_label` honouring a discrete `position_dodge` via `XMapper` (D8's
//! sibling change for text).

use ggplot_rs::position::Position;
use ggplot_rs::prelude::*;

fn render(plot: GGPlot, w: u32, h: u32) -> String {
    plot.render_svg_native_with_size(w, h)
        .expect("native svg render")
}

fn single_x_two_groups() -> Vec<(String, Vec<Value>)> {
    vec![
        ("x".into(), vec![Value::Str("a".into()); 2]),
        (
            "grp".into(),
            vec![Value::Str("g1".into()), Value::Str("g2".into())],
        ),
        ("y".into(), vec![Value::Float(3.0), Value::Float(2.0)]),
    ]
}

/// R parity (ggplot2 4.0.3): `position_stack(vjust = .5)`, x=1, y=(3,2)
/// gives y = 3.5, 1.0.
#[test]
fn stack_vjust_half_matches_r() {
    let svg = GGPlot::new(single_x_two_groups())
        .aes(Aes::new().x("x").y("y").fill("grp"))
        .geom_col()
        .position(PositionStack.with_vjust(0.5))
        .tooltip("{y}")
        .render_svg_native_with_size(300, 300)
        .unwrap();
    assert!(svg.contains("<title>3.5</title>"), "{svg}");
    assert!(svg.contains("<title>1</title>"), "{svg}");
}

/// R parity: `position_fill(vjust = .5)`, same data, gives y = .7, .2.
#[test]
fn fill_vjust_half_matches_r() {
    let svg = GGPlot::new(single_x_two_groups())
        .aes(Aes::new().x("x").y("y").fill("grp"))
        .geom_col()
        .position(PositionFill.with_vjust(0.5))
        .tooltip("{y}")
        .render_svg_native_with_size(300, 300)
        .unwrap();
    assert!(svg.contains("<title>0.7</title>"), "{svg}");
    assert!(svg.contains("<title>0.2</title>"), "{svg}");
}

/// `vjust` 1 (the default) is byte-identical: no `ymax` column, same `y`
/// as before B3-4.
#[test]
fn vjust_one_is_the_default_and_byte_identical() {
    let explicit = render(
        GGPlot::new(single_x_two_groups())
            .aes(Aes::new().x("x").y("y").fill("grp"))
            .geom_col()
            .position(PositionStack),
        300,
        300,
    );
    let via_builder = render(
        GGPlot::new(single_x_two_groups())
            .aes(Aes::new().x("x").y("y").fill("grp"))
            .geom_col()
            .position(position_stack()),
        300,
        300,
    );
    assert_eq!(explicit, via_builder);
}

/// The bare `PositionStack`/`PositionFill` consts still compile as values
/// (the dodge.rs precedent), now that the types have an explicit field.
#[test]
fn the_bare_consts_still_compile() {
    let _: Box<dyn Position> = Box::new(PositionStack);
    let _: Box<dyn Position> = Box::new(PositionFill);
}

fn facet_panel_data() -> Vec<(String, Vec<Value>)> {
    vec![
        (
            "panel".into(),
            vec![
                Value::Str("P".into()),
                Value::Str("P".into()),
                Value::Str("Q".into()),
                Value::Str("Q".into()),
            ],
        ),
        ("x".into(), vec![Value::Str("a".into()); 4]),
        (
            "grp".into(),
            vec![
                Value::Str("g1".into()),
                Value::Str("g2".into()),
                Value::Str("g1".into()),
                Value::Str("g2".into()),
            ],
        ),
        (
            "y".into(),
            vec![
                Value::Float(3.0),
                Value::Float(5.0),
                Value::Float(4.0),
                Value::Float(1.0),
            ],
        ),
    ]
}

/// B3-3 (per-panel stack) composed with B3-4 (vjust): each panel's own
/// stack midpoint, not one pooled across panels. ggplot2 4.0.3: P's
/// segments are [5,8] and [0,5] -> midpoints 6.5, 2.5; Q's are [1,5] and
/// [0,1] -> midpoints 3.0, 0.5.
#[test]
fn facet_stack_vjust_half_midpoints_match_r() {
    let svg = GGPlot::new(facet_panel_data())
        .aes(Aes::new().x("x").y("y").fill("grp"))
        .geom_col()
        .position(PositionStack.with_vjust(0.5))
        .tooltip("{y}")
        .facet_wrap("panel", None)
        .render_svg_native_with_size(600, 300)
        .unwrap();
    for expect in [
        "<title>6.5</title>",
        "<title>2.5</title>",
        "<title>3</title>",
        "<title>0.5</title>",
    ] {
        assert!(svg.contains(expect), "missing {expect} in {svg}");
    }
}

/// A non-finite vjust warns and falls back to 1 (the top-anchored path),
/// rather than panicking or producing NaN positions.
#[test]
fn non_finite_vjust_warns_and_falls_back() {
    let (svg, warnings) = GGPlot::new(single_x_two_groups())
        .aes(Aes::new().x("x").y("y").fill("grp"))
        .geom_col()
        .position(PositionStack.with_vjust(f64::NAN))
        .render_svg_native_with_warnings(300, 300)
        .unwrap();
    assert!(!svg.contains("NaN"), "{svg}");
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("vjust") && w.contains("finite")),
        "{warnings:?}"
    );
}

fn dodge_two_groups_two_x() -> Vec<(String, Vec<Value>)> {
    vec![
        (
            "x".into(),
            vec![
                Value::Str("W1".into()),
                Value::Str("W1".into()),
                Value::Str("W2".into()),
                Value::Str("W2".into()),
            ],
        ),
        (
            "grp".into(),
            vec![
                Value::Str("G1".into()),
                Value::Str("G2".into()),
                Value::Str("G1".into()),
                Value::Str("G2".into()),
            ],
        ),
        (
            "y".into(),
            vec![
                Value::Float(3.0),
                Value::Float(5.0),
                Value::Float(4.0),
                Value::Float(2.0),
            ],
        ),
        (
            "lbl".into(),
            vec![
                Value::Str("L3".into()),
                Value::Str("L5".into()),
                Value::Str("L4".into()),
                Value::Str("L2".into()),
            ],
        ),
    ]
}

/// `geom_text` under a discrete `position_dodge(.9)` lands at the same x
/// as `geom_col`'s dodged bar centres for the identical data (R: x =
/// .775, 1.225, 1.775, 2.225 for 2 groups on W1/W2).
#[test]
fn text_dodge_x_matches_col_dodge_centres() {
    let data = dodge_two_groups_two_x();
    let bars = render(
        GGPlot::new(data.clone())
            .aes(Aes::new().x("x").y("y").fill("grp").group("grp"))
            .geom_col()
            .position(position_dodge(0.9)),
        400,
        300,
    );
    let labels = render(
        GGPlot::new(data)
            .aes(
                Aes::new()
                    .x("x")
                    .y("y")
                    .fill("grp")
                    .group("grp")
                    .label("lbl"),
            )
            .geom_text()
            .position(position_dodge(0.9)),
        400,
        300,
    );

    // Bar centres: x + width/2, in document order (4 bars).
    let mut bar_centres = Vec::new();
    for (start, _) in bars.match_indices("<rect ") {
        let tag_end = bars[start..].find('/').map(|e| start + e).unwrap_or(start);
        let tag = &bars[start..tag_end];
        if !tag.contains("data-series") {
            continue; // background/panel rects
        }
        let x: f64 = tag
            .split("x=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        let w: f64 = tag
            .split("width=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        bar_centres.push(x + w / 2.0);
    }

    let mut label_x = Vec::new();
    for lbl in ["L3", "L5", "L4", "L2"] {
        let needle = format!(">{lbl}<");
        let end = labels.find(&needle).unwrap();
        let start = labels[..end].rfind("<text ").unwrap();
        let x: f64 = labels[start..end]
            .split("x=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        label_x.push(x);
    }

    assert_eq!(bar_centres.len(), 4, "{bars}");
    assert_eq!(label_x.len(), 4);
    for (b, l) in bar_centres.iter().zip(label_x.iter()) {
        assert!((b - l).abs() < 0.5, "bar centre {b} vs label x {l}");
    }
}
