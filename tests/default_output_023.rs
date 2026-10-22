//! Safety net for 0.23 (per-panel dodge/jitter, negative stacking, legend
//! wrap for multiple guides, vertical legend fit): a battery of fixed
//! plots exercising the four deliberate changes plus their unaffected
//! (byte-identical) counterparts, rendered through the plotters-free
//! native SVG path, hashed with FNV-1a.
//!
//! If a hash here needs to change, it must be caused by one of #153,
//! #154, #155, #159 (or a documented follow-up), and the change must be
//! listed under CHANGELOG's `[0.23.0]` "Changed". Anything else that
//! flips a hash here is a regression — fix the code, not the test.
//!
//! Runs in every feature configuration (no polars/plotters/sf-only API), so
//! it is part of every `cargo test` job, including `--no-default-features`
//! and `--no-default-features --features sf`.

use ggplot_rs::data::Value;
use ggplot_rs::guide::config::GuideLegend;
use ggplot_rs::prelude::*;

/// FNV-1a, 64-bit. No dependency; good enough to catch byte-for-byte drift.
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn hash_svg(plot: GGPlot, w: u32, h: u32, what: &str) -> u64 {
    let svg = plot
        .render_svg_native_with_size(w, h)
        .unwrap_or_else(|e| panic!("{what}: render failed: {e}"));
    fnv1a(svg.as_bytes())
}

type Cols = Vec<(String, Vec<Value>)>;

fn f(v: &[f64]) -> Vec<Value> {
    v.iter().map(|x| Value::Float(*x)).collect()
}
fn s(v: &[&str]) -> Vec<Value> {
    v.iter().map(|x| Value::Str(x.to_string())).collect()
}

/// #153: 2 panels sharing x = 1; panel P has 3 groups there, panel Q has
/// only 1 — the per-panel dodge fix changes panel Q's offset.
fn dodge_facet_data() -> Cols {
    vec![
        ("panel".into(), s(&["P", "P", "P", "Q"])),
        ("x".into(), f(&[1.0, 1.0, 1.0, 1.0])),
        ("grp".into(), s(&["g1", "g2", "g3", "g1"])),
        ("y".into(), f(&[1.0, 2.0, 3.0, 1.0])),
    ]
}

/// A single-panel dodge: unaffected by #153 (nothing to partition).
fn dodge_single_panel_data() -> Cols {
    vec![
        ("x".into(), f(&[1.0, 1.0, 1.0])),
        ("grp".into(), s(&["g1", "g2", "g3"])),
        ("y".into(), f(&[1.0, 2.0, 3.0])),
    ]
}

/// #153: 2 panels with very different x spacing (resolution 1 vs 0.01),
/// same fixed facet scale — the per-panel jitter fix scales each panel's
/// default jitter by its own resolution.
fn jitter_facet_data() -> Cols {
    vec![
        ("panel".into(), s(&["P", "P", "P", "Q", "Q", "Q"])),
        ("x".into(), f(&[1.0, 2.0, 3.0, 10.00, 10.01, 10.02])),
        ("y".into(), f(&[1.0, 2.0, 3.0, 10.00, 10.01, 10.02])),
    ]
}

/// #154: a bar chart mixing positive and negative values at one x.
fn mixed_sign_data() -> Cols {
    vec![
        ("x".into(), f(&[1.0, 1.0, 1.0, 1.0])),
        ("grp".into(), s(&["a", "b", "c", "d"])),
        ("y".into(), f(&[3.0, -2.0, 4.0, -1.0])),
    ]
}

/// A bar chart with only positive values at each x: unaffected by #154
/// (the negative bucket is simply empty).
fn all_positive_stack_data() -> Cols {
    vec![
        ("x".into(), f(&[1.0, 1.0])),
        ("grp".into(), s(&["a", "b"])),
        ("y".into(), f(&[3.0, 5.0])),
    ]
}

/// #155: `colour` and `shape` both mapped to discrete variables, with
/// enough entries that a bottom legend overflows a one-line layout.
fn two_discrete_guides_data() -> Cols {
    let n = 8;
    let x: Vec<Value> = (0..n).map(|i| Value::Float(i as f64)).collect();
    let y: Vec<Value> = (0..n).map(|i| Value::Float((i % 3) as f64)).collect();
    let g: Vec<Value> = (0..n)
        .map(|i| Value::Str(format!("Colour {:02}", i + 1)))
        .collect();
    let sh: Vec<Value> = (0..n)
        .map(|i| Value::Str(format!("Shape {:02}", i + 1)))
        .collect();
    vec![
        ("x".into(), x),
        ("y".into(), y),
        ("g".into(), g),
        ("sh".into(), sh),
    ]
}

/// A single discrete guide, overflowing a one-line bottom legend: the
/// 0.22 auto-wrap path, unaffected by #155 (only one guide present).
fn one_discrete_guide_data() -> Cols {
    let n = 10;
    let x: Vec<Value> = (0..n).map(|i| Value::Float(i as f64)).collect();
    let y: Vec<Value> = (0..n).map(|i| Value::Float((i % 3) as f64)).collect();
    let g: Vec<Value> = (0..n)
        .map(|i| Value::Str(format!("Group {:02}", i + 1)))
        .collect();
    vec![("x".into(), x), ("y".into(), y), ("g".into(), g)]
}

/// #159: a vertical `ncol(2)` grid that already fits — unaffected (no
/// reduction, no truncation, no warning).
fn ncol_fits_data() -> Cols {
    let x: Vec<Value> = (0..4).map(|i| Value::Float(i as f64)).collect();
    let y: Vec<Value> = (0..4).map(|i| Value::Float((i % 3) as f64)).collect();
    let g: Vec<Value> = (0..4).map(|i| Value::Str(format!("G{}", i + 1))).collect();
    vec![("x".into(), x), ("y".into(), y), ("g".into(), g)]
}

/// #159: an `ncol(4)` grid with long labels on a narrow canvas — column
/// reduction alone resolves the overflow.
fn ncol_reduces_data() -> Cols {
    let n = 12;
    let x: Vec<Value> = (0..n).map(|i| Value::Float(i as f64)).collect();
    let y: Vec<Value> = (0..n).map(|i| Value::Float((i % 3) as f64)).collect();
    let g: Vec<Value> = (0..n)
        .map(|i| Value::Str(format!("A Very Long Group Label Number {}", i + 1)))
        .collect();
    vec![("x".into(), x), ("y".into(), y), ("g".into(), g)]
}

/// #159: an `ncol(6)` grid with extremely long labels — column reduction
/// alone isn't enough, so labels truncate with an ellipsis.
fn ncol_truncates_data() -> Cols {
    let n = 12;
    let x: Vec<Value> = (0..n).map(|i| Value::Float(i as f64)).collect();
    let y: Vec<Value> = (0..n).map(|i| Value::Float((i % 3) as f64)).collect();
    let g: Vec<Value> = (0..n)
        .map(|i| {
            Value::Str(format!(
                "An Extremely Very Super Duper Long Group Label Number {} \
                 With Lots More Text Appended Here To Make It Even Longer",
                i + 1
            ))
        })
        .collect();
    vec![("x".into(), x), ("y".into(), y), ("g".into(), g)]
}

fn expected() -> Vec<(&'static str, u64)> {
    vec![
        ("dodge_facet", 0x87ca741c10e5d81f),
        ("dodge_single_panel", 0x60eede539d041c9f),
        ("jitter_facet", 0x2041b5e10e321c52),
        ("stack_mixed_sign", 0xa3dd3e684c64f693),
        ("all_positive_stack", 0x92ba5f6f174412ae),
        ("two_discrete_guides", 0x1c9247192286f120),
        ("one_discrete_guide", 0x98da47e69ace53db),
        ("ncol_fits", 0x4bbbea598fcaaceb),
        ("ncol_reduces", 0xfb022ae18760d3b7),
        ("ncol_truncates", 0x74f922dad5ba6460),
    ]
}

#[test]
fn default_output_023_battery() {
    let cases: Vec<(&str, u64)> = vec![
        (
            "dodge_facet",
            hash_svg(
                GGPlot::new(dodge_facet_data())
                    .aes(Aes::new().x("x").y("y").color("grp"))
                    .geom_point()
                    .position(ggplot_rs::position::dodge::position_dodge(0.9))
                    .facet_wrap("panel", None),
                600,
                300,
                "dodge_facet",
            ),
        ),
        (
            "dodge_single_panel",
            hash_svg(
                GGPlot::new(dodge_single_panel_data())
                    .aes(Aes::new().x("x").y("y").color("grp"))
                    .geom_point()
                    .position(ggplot_rs::position::dodge::position_dodge(0.9)),
                480,
                320,
                "dodge_single_panel",
            ),
        ),
        (
            "jitter_facet",
            hash_svg(
                GGPlot::new(jitter_facet_data())
                    .aes(Aes::new().x("x").y("y"))
                    .geom_jitter()
                    .facet_wrap("panel", None),
                600,
                300,
                "jitter_facet",
            ),
        ),
        (
            "stack_mixed_sign",
            hash_svg(
                GGPlot::new(mixed_sign_data())
                    .aes(Aes::new().x("x").y("y").fill("grp"))
                    .geom_col(),
                320,
                400,
                "stack_mixed_sign",
            ),
        ),
        (
            "all_positive_stack",
            hash_svg(
                GGPlot::new(all_positive_stack_data())
                    .aes(Aes::new().x("x").y("y").fill("grp"))
                    .geom_col(),
                320,
                400,
                "all_positive_stack",
            ),
        ),
        (
            "two_discrete_guides",
            hash_svg(
                GGPlot::new(two_discrete_guides_data())
                    .aes(Aes::new().x("x").y("y").color("g").shape("sh"))
                    .geom_point()
                    .legend_position(LegendPosition::Bottom),
                640,
                500,
                "two_discrete_guides",
            ),
        ),
        (
            "one_discrete_guide",
            hash_svg(
                GGPlot::new(one_discrete_guide_data())
                    .aes(Aes::new().x("x").y("y").color("g"))
                    .geom_point()
                    .legend_position(LegendPosition::Bottom),
                640,
                400,
                "one_discrete_guide",
            ),
        ),
        (
            "ncol_fits",
            hash_svg(
                GGPlot::new(ncol_fits_data())
                    .aes(Aes::new().x("x").y("y").color("g"))
                    .geom_point()
                    .guides(GuideLegend::new().with_ncol(2)),
                500,
                400,
                "ncol_fits",
            ),
        ),
        (
            "ncol_reduces",
            hash_svg(
                GGPlot::new(ncol_reduces_data())
                    .aes(Aes::new().x("x").y("y").color("g"))
                    .geom_point()
                    .guides(GuideLegend::new().with_ncol(4)),
                300,
                400,
                "ncol_reduces",
            ),
        ),
        (
            "ncol_truncates",
            hash_svg(
                GGPlot::new(ncol_truncates_data())
                    .aes(Aes::new().x("x").y("y").color("g"))
                    .geom_point()
                    .guides(GuideLegend::new().with_ncol(6)),
                200,
                400,
                "ncol_truncates",
            ),
        ),
    ];

    let expected = expected();
    assert_eq!(
        cases.len(),
        expected.len(),
        "case count drifted from the recorded table"
    );

    let mut mismatches = Vec::new();
    for ((name, got), (exp_name, exp_hash)) in cases.iter().zip(expected.iter()) {
        assert_eq!(name, exp_name, "case order drifted from the recorded table");
        if got != exp_hash {
            mismatches.push(format!(
                "{name}: expected {exp_hash:#018x}, got {got:#018x}"
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "default-output hash mismatch — must be caused by #153/#154/#155/#159 and \
         documented in CHANGELOG [0.23.0]:\n{}",
        mismatches.join("\n")
    );
}
