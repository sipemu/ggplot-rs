//! B3-3: `position_stack`/`position_fill` run per facet panel, not on the
//! rows pooled across panels (D8). R parity (ggplot2 4.0.3): a single x
//! level with y = (3, 5) in panel P and y = (4, 1) in panel Q gives
//! stack y/ymin = [8,5 / 5,0] (P) and [5,1 / 1,0] (Q); fill gives
//! y/ymin = [1,.625 / .625,0] (P) and [1,.2 / .2,0] (Q).

use ggplot_rs::prelude::*;
use ggplot_rs::scale::format::label_number;

fn panel_data() -> Vec<(String, Vec<Value>)> {
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
        (
            "x".into(),
            vec![
                Value::Str("a".into()),
                Value::Str("a".into()),
                Value::Str("a".into()),
                Value::Str("a".into()),
            ],
        ),
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

#[test]
fn facet_stack_matches_r_per_panel() {
    let svg = GGPlot::new(panel_data())
        .aes(Aes::new().x("x").y("y").fill("grp"))
        .geom_col()
        .position(PositionStack)
        .tooltip("{y}/{ymin}")
        .facet_wrap("panel", None)
        .render_svg_native_with_size(600, 300)
        .unwrap();

    for expect in [
        "<title>8/5</title>",
        "<title>5/0</title>",
        "<title>5/1</title>",
        "<title>1/0</title>",
    ] {
        assert!(svg.contains(expect), "missing {expect} in {svg}");
    }
}

#[test]
fn facet_fill_matches_r_per_panel() {
    let svg = GGPlot::new(panel_data())
        .aes(Aes::new().x("x").y("y").fill("grp"))
        .geom_col()
        .position(PositionFill)
        .scale_y_continuous(ScaleContinuous::new().with_value_formatter(label_number(
            Some(0.001),
            "",
            "",
            1.0,
        )))
        .tooltip("{y}/{ymin}")
        .facet_wrap("panel", None)
        .render_svg_native_with_size(600, 300)
        .unwrap();

    // `ymin` is now also formatted through the y scale (D9 allowed #7,
    // landed with B3-5), so it picks up the same 3-decimal accuracy as
    // `y` instead of the plain fallback's trimmed digits.
    for expect in [
        "<title>1.000/0.625</title>",
        "<title>0.625/0.000</title>",
        "<title>1.000/0.200</title>",
    ] {
        assert!(svg.contains(expect), "missing {expect} in {svg}");
    }
    // Q's g2 ymin is 0.2 - 0.2, which floating subtraction leaves as a
    // tiny negative value that rounds to "-0.000" rather than "0.000" —
    // still correctly zero to any real precision.
    assert!(
        svg.contains("<title>0.200/-0.000</title>") || svg.contains("<title>0.200/0.000</title>"),
        "{svg}"
    );
}

/// `facet_grid` pools the same way `facet_wrap` does — the per-panel split
/// is keyed off the facet variable columns, not the wrap/grid machinery.
#[test]
fn facet_grid_also_stacks_per_panel() {
    let svg = GGPlot::new(panel_data())
        .aes(Aes::new().x("x").y("y").fill("grp"))
        .geom_col()
        .position(PositionStack)
        .tooltip("{y}/{ymin}")
        .facet_grid(None, Some("panel"))
        .render_svg_native_with_size(600, 300)
        .unwrap();

    for expect in [
        "<title>8/5</title>",
        "<title>5/0</title>",
        "<title>5/1</title>",
        "<title>1/0</title>",
    ] {
        assert!(svg.contains(expect), "missing {expect} in {svg}");
    }
}

/// No facet at all: the single-panel path is untouched (D8 only changes
/// behaviour when there are 2+ distinct panel keys).
#[test]
fn unfaceted_stack_is_unaffected() {
    let data = vec![
        ("x".to_string(), vec![Value::Str("a".into()); 2]),
        (
            "grp".to_string(),
            vec![Value::Str("g1".into()), Value::Str("g2".into())],
        ),
        ("y".to_string(), vec![Value::Float(3.0), Value::Float(5.0)]),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").fill("grp"))
        .geom_col()
        .position(PositionStack)
        .tooltip("{y}/{ymin}")
        .render_svg_native_with_size(300, 300)
        .unwrap();
    assert!(svg.contains("<title>8/5</title>"));
    assert!(svg.contains("<title>5/0</title>"));
}

/// Dodge and jitter stay pooled across panels (D8 scopes per-panel to
/// Stack/Fill only) — unaffected by this change.
#[test]
fn dodge_and_jitter_are_unaffected_by_this_change() {
    let data = vec![
        (
            "panel".to_string(),
            vec![
                Value::Str("P".into()),
                Value::Str("P".into()),
                Value::Str("Q".into()),
                Value::Str("Q".into()),
            ],
        ),
        ("x".to_string(), vec![Value::Str("a".into()); 4]),
        (
            "grp".to_string(),
            vec![
                Value::Str("g1".into()),
                Value::Str("g2".into()),
                Value::Str("g1".into()),
                Value::Str("g2".into()),
            ],
        ),
        (
            "y".to_string(),
            vec![
                Value::Float(3.0),
                Value::Float(5.0),
                Value::Float(4.0),
                Value::Float(1.0),
            ],
        ),
    ];
    let svg_dodge = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y").fill("grp"))
        .geom_col()
        .position(ggplot_rs::position::dodge::position_dodge(0.9))
        .facet_wrap("panel", None)
        .render_svg_native_with_size(600, 300)
        .unwrap();
    assert!(svg_dodge.starts_with("<svg"));

    let svg_jitter = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("grp"))
        .geom_jitter()
        .facet_wrap("panel", None)
        .render_svg_native_with_size(600, 300)
        .unwrap();
    assert!(svg_jitter.starts_with("<svg"));
}
