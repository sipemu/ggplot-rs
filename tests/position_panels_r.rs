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

/// #153: `position_dodge` now also runs per facet panel. R parity
/// (ggplot2 4.0.3, `layer_data()`): panel P has 3 groups at x = 1 (g1, g2,
/// g3; dodge(width = 0.9) centres them at 0.7, 1.0, 1.3), panel Q has only
/// g1 at the same x — pooled across panels (the pre-#153 bug) g1 would
/// still see {g1,g2,g3} present and land at 0.7; computed per panel, g1 is
/// the only group there and stays centred at 1.0 (unshifted).
#[test]
fn facet_dodge_matches_r_per_panel() {
    let data = vec![
        (
            "panel".to_string(),
            vec![
                Value::Str("P".into()),
                Value::Str("P".into()),
                Value::Str("P".into()),
                Value::Str("Q".into()),
            ],
        ),
        ("x".to_string(), vec![Value::Float(1.0); 4]),
        (
            "grp".to_string(),
            vec![
                Value::Str("g1".into()),
                Value::Str("g2".into()),
                Value::Str("g3".into()),
                Value::Str("g1".into()),
            ],
        ),
        ("y".to_string(), vec![Value::Float(1.0); 4]),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("grp"))
        .geom_point()
        .position(ggplot_rs::position::dodge::position_dodge(0.9))
        .tooltip("{x}")
        .facet_wrap("panel", None)
        .render_svg_native_with_size(600, 300)
        .unwrap();

    for expect in [
        "<title>0.7</title>",
        "<title>1</title>",
        "<title>1.3</title>",
    ] {
        assert!(svg.contains(expect), "missing {expect} in {svg}");
    }
    assert!(
        !svg.contains("<title>0.7</title><title>0.7</title>"),
        "panel Q's lone group must not duplicate panel P's offset"
    );
}

/// Review #167 finding 2: a dodge's group *rank* must come from the whole
/// layer, not from whatever order each panel's own rows happen to list
/// groups in. Panel P lists `g1` then `g2`; panel Q lists the same two
/// groups in the opposite row order (`g2` then `g1`). ggplot2 4.0.3
/// (`layer_data()`) and 0.22.2 both rank by the global `group` id, so
/// `g1` dodges left (x=0.75) and `g2` right (x=1.25) in *both* panels.
/// Before this fix, Q's `compute` re-derived "first appearance" from its
/// own rows and drew `g2` left / `g1` right there instead.
#[test]
fn facet_dodge_order_is_global_not_per_panel_first_appearance() {
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
        ("x".to_string(), vec![Value::Float(1.0); 4]),
        (
            "grp".to_string(),
            vec![
                // P: g1 then g2 (first-appearance order == global order).
                Value::Str("g1".into()),
                Value::Str("g2".into()),
                // Q: g2 then g1 — the opposite row order.
                Value::Str("g2".into()),
                Value::Str("g1".into()),
            ],
        ),
        ("y".to_string(), vec![Value::Float(1.0); 4]),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("grp"))
        .geom_point()
        .position(ggplot_rs::position::dodge::position_dodge(1.0))
        .tooltip("{grp}@{x}")
        .facet_wrap("panel", None)
        .render_svg_native_with_size(600, 300)
        .unwrap();

    for expect in ["<title>g1@0.75</title>", "<title>g2@1.25</title>"] {
        let n = svg.matches(expect).count();
        assert_eq!(
            n, 2,
            "expected {expect} once per panel (P and Q) — found {n} in {svg}"
        );
    }
    // The regression: Q's own row order would otherwise have drawn g2
    // left (0.75) and g1 right (1.25).
    assert!(
        !svg.contains("<title>g2@0.75</title>"),
        "g2 must not dodge left of g1 in any panel: {svg}"
    );
    assert!(
        !svg.contains("<title>g1@1.25</title>"),
        "g1 must not dodge right of g2 in any panel: {svg}"
    );
}

/// `position_dodge2` shares `dodge_rows` with `position_dodge`, so the
/// same per-panel rule applies to it (#153).
#[test]
fn facet_dodge2_runs_per_panel() {
    let data = vec![
        (
            "panel".to_string(),
            vec![
                Value::Str("P".into()),
                Value::Str("P".into()),
                Value::Str("Q".into()),
            ],
        ),
        ("x".to_string(), vec![Value::Float(1.0); 3]),
        (
            "grp".to_string(),
            vec![
                Value::Str("g1".into()),
                Value::Str("g2".into()),
                Value::Str("g1".into()),
            ],
        ),
        ("y".to_string(), vec![Value::Float(1.0); 3]),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("grp"))
        .geom_point()
        .position(ggplot_rs::position::dodge2::PositionDodge2::new(0.0).with_width(0.9))
        .tooltip("{x}")
        .facet_wrap("panel", None)
        .render_svg_native_with_size(600, 300)
        .unwrap();

    // Panel Q's lone g1 is unshifted (centred at 1), not offset the way
    // it would be if panel P's second group leaked into panel Q's count.
    assert!(svg.contains("<title>1</title>"), "{svg}");
}

/// #153: `position_jitter`'s default width (`resolution(x) * 0.4`) is also
/// measured per panel. R parity (ggplot2 4.0.3): panel P's x is spaced 1
/// apart (resolution 1, jitter up to ±0.4); panel Q's x is spaced 0.01
/// apart (resolution 0.01, jitter up to ±0.004) — even though both panels
/// share the same fixed facet scale. Pooled across panels (the pre-#153
/// bug), the whole layer's minimum gap (0.01) would clamp panel P's
/// jitter down to ±0.004 too.
#[test]
fn facet_jitter_resolution_is_measured_per_panel() {
    let data = vec![
        (
            "panel".to_string(),
            vec![
                Value::Str("P".into()),
                Value::Str("P".into()),
                Value::Str("P".into()),
                Value::Str("Q".into()),
                Value::Str("Q".into()),
                Value::Str("Q".into()),
            ],
        ),
        (
            "x".to_string(),
            vec![
                Value::Float(1.0),
                Value::Float(2.0),
                Value::Float(3.0),
                Value::Float(10.00),
                Value::Float(10.01),
                Value::Float(10.02),
            ],
        ),
        (
            "y".to_string(),
            vec![
                Value::Float(1.0),
                Value::Float(2.0),
                Value::Float(3.0),
                Value::Float(10.00),
                Value::Float(10.01),
                Value::Float(10.02),
            ],
        ),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_jitter()
        .facet_wrap("panel", None)
        .render_svg_native_with_size(600, 300)
        .unwrap();

    let cx: Vec<f64> = svg
        .match_indices("<circle cx=\"")
        .map(|(start, _)| {
            let rest = &svg[start + "<circle cx=\"".len()..];
            rest[..rest.find('"').unwrap()].parse().unwrap()
        })
        .collect();
    assert_eq!(cx.len(), 6, "expected 3 points in each of 2 panels: {svg}");
    let spread = |vs: &[f64]| {
        vs.iter().cloned().fold(f64::MIN, f64::max)
            - vs[..].iter().cloned().fold(f64::MAX, f64::min)
    };
    let panel_p_spread = spread(&cx[0..3]).abs();
    let panel_q_spread = spread(&cx[3..6]).abs();
    // Panel P's x is spaced 1 apart (resolution 1): its 3 jittered points
    // spread over a sizeable pixel range. Panel Q's x is spaced 0.01
    // apart (resolution 0.01): its jitter is ~100x smaller in data units,
    // and both panels share the same pixel width per data unit here, so
    // panel Q's spread must be much smaller than panel P's — only
    // possible if each panel's resolution (and jitter amount) was
    // measured from its own rows, not pooled across panels.
    assert!(
        panel_p_spread > panel_q_spread * 5.0,
        "panel P spread {panel_p_spread} should be much larger than panel Q spread {panel_q_spread}: {svg}"
    );
}
