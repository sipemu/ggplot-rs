//! Safety net for 0.22 (D1): 25 fixed plots, rendered through the
//! plotters-free native SVG path, hashed with FNV-1a and compared against
//! the hash recorded at v0.21.0 (f5fc31e, the base of this branch).
//!
//! If a hash here needs to change, it must be in the §4 allowed list of
//! `plans/plan-ggplot-rs-022.md`, changed in the exact commit that causes
//! it, and documented with a CHANGELOG "Changed" line. Anything else that
//! flips a hash here is a regression — fix the code, not the test.
//!
//! Runs in every feature configuration (no polars/plotters/sf-only API), so
//! it is part of every `cargo test` job, including `--no-default-features`
//! and `--no-default-features --features sf`.

use ggplot_rs::data::Value;
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

/// 24 points, 3 groups, deterministic (no RNG).
fn xy_grouped() -> Cols {
    let n = 24;
    let x: Vec<f64> = (0..n).map(|i| i as f64 * 0.5).collect();
    let y: Vec<f64> = (0..n)
        .map(|i| (i as f64 * 0.3).sin() * 3.0 + i as f64 * 0.1)
        .collect();
    let g: Vec<&str> = (0..n).map(|i| ["A", "B", "C"][i % 3]).collect();
    vec![
        ("x".into(), f(&x)),
        ("y".into(), f(&y)),
        ("g".into(), s(&g)),
    ]
}

fn bar_data() -> Cols {
    vec![
        ("cat".into(), s(&["a", "b", "c", "d"])),
        ("val".into(), f(&[3.0, 7.0, 5.0, 9.0])),
        ("grp".into(), s(&["g1", "g1", "g2", "g2"])),
    ]
}

fn band_data() -> Cols {
    let n = 20;
    let x: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|v| (v * 0.4).sin() * 2.0 + 5.0).collect();
    let ymin: Vec<f64> = y.iter().map(|v| v - 1.0).collect();
    let ymax: Vec<f64> = y.iter().map(|v| v + 1.0).collect();
    vec![
        ("x".into(), f(&x)),
        ("y".into(), f(&y)),
        ("ymin".into(), f(&ymin)),
        ("ymax".into(), f(&ymax)),
    ]
}

fn seg_data() -> Cols {
    vec![
        ("x".into(), f(&[0.0, 1.0, 2.0])),
        ("y".into(), f(&[0.0, 1.0, 0.5])),
        ("xend".into(), f(&[1.0, 2.0, 3.0])),
        ("yend".into(), f(&[1.0, 0.0, 2.0])),
    ]
}

fn tile_data() -> Cols {
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut v = Vec::new();
    for i in 0..4 {
        for j in 0..4 {
            x.push(i as f64);
            y.push(j as f64);
            v.push((i * 4 + j) as f64);
        }
    }
    vec![
        ("x".into(), f(&x)),
        ("y".into(), f(&y)),
        ("v".into(), f(&v)),
    ]
}

fn boxplot_data() -> Cols {
    // Tight clusters per group so no outlier is produced — keeps this case
    // decoupled from the B3-5 "outliers widen y" change.
    let n_per = 10;
    let mut g = Vec::new();
    let mut y = Vec::new();
    for (gi, name) in ["A", "B", "C"].iter().enumerate() {
        for i in 0..n_per {
            g.push(*name);
            y.push(5.0 + gi as f64 * 2.0 + (i as f64 - n_per as f64 / 2.0) * 0.1);
        }
    }
    vec![("g".into(), s(&g)), ("y".into(), f(&y))]
}

fn hist_data() -> Cols {
    let n = 40;
    let y: Vec<f64> = (0..n)
        .map(|i| (i as f64 * 7.0).sin() * 5.0 + 10.0)
        .collect();
    vec![("y".into(), f(&y))]
}

fn facet_data() -> Cols {
    let n = 24;
    let x: Vec<f64> = (0..n).map(|i| (i % 8) as f64).collect();
    let y: Vec<f64> = (0..n).map(|i| ((i / 8 + 1) * (i % 8 + 1)) as f64).collect();
    let panel: Vec<&str> = (0..n).map(|i| ["P", "Q", "R"][i / 8]).collect();
    vec![
        ("x".into(), f(&x)),
        ("y".into(), f(&y)),
        ("panel".into(), s(&panel)),
    ]
}

fn facet_stack_data() -> Cols {
    vec![
        ("panel".into(), s(&["P", "P", "P", "P", "Q", "Q", "Q", "Q"])),
        ("x".into(), s(&["a", "a", "b", "b", "a", "a", "b", "b"])),
        (
            "grp".into(),
            s(&["g1", "g2", "g1", "g2", "g1", "g2", "g1", "g2"]),
        ),
        ("y".into(), f(&[3.0, 5.0, 4.0, 1.0, 5.0, 4.0, 5.0, 1.0])),
    ]
}

fn text_data() -> Cols {
    vec![
        ("x".into(), f(&[1.0, 2.0, 3.0, 4.0])),
        ("y".into(), f(&[2.0, 4.0, 1.0, 3.0])),
        ("lbl".into(), s(&["w", "x", "y", "z"])),
    ]
}

fn text_dodge_data() -> Cols {
    vec![
        ("x".into(), s(&["W1", "W1", "W2", "W2"])),
        ("grp".into(), s(&["G1", "G2", "G1", "G2"])),
        ("y".into(), f(&[3.0, 5.0, 4.0, 2.0])),
        ("lbl".into(), s(&["3", "5", "4", "2"])),
    ]
}

fn errorbar_data() -> Cols {
    vec![
        ("x".into(), f(&[1.0, 2.0, 3.0])),
        ("y".into(), f(&[3.0, 5.0, 4.0])),
        ("ymin".into(), f(&[2.5, 4.5, 3.5])),
        ("ymax".into(), f(&[3.5, 5.5, 4.5])),
    ]
}

fn errorbar_dodge_data() -> Cols {
    vec![
        ("x".into(), s(&["A", "A", "B", "B"])),
        ("grp".into(), s(&["G1", "G2", "G1", "G2"])),
        ("y".into(), f(&[3.0, 5.0, 4.0, 2.0])),
        ("ymin".into(), f(&[2.5, 4.5, 3.5, 1.5])),
        ("ymax".into(), f(&[3.5, 5.5, 4.5, 2.5])),
    ]
}

fn size_data() -> Cols {
    let n = 12;
    let x: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let y: Vec<f64> = (0..n).map(|i| (i as f64 * 0.7).cos() * 4.0).collect();
    let v: Vec<f64> = (0..n).map(|i| (i + 1) as f64).collect();
    vec![
        ("x".into(), f(&x)),
        ("y".into(), f(&y)),
        ("v".into(), f(&v)),
    ]
}

/// 20 discrete x values: long labels at a narrow width overflow the SVG
/// edge at the outer ticks under 0.21.0's no-op axis fit (B3-1 territory).
fn wide_axis_data() -> Cols {
    let labels = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
        "Thirteen",
        "Fourteen",
        "Fifteen",
        "Sixteen",
        "Seventeen",
        "Eighteen",
        "Nineteen",
        "Twenty",
    ];
    let n = labels.len();
    let x: Vec<&str> = labels.to_vec();
    let y: Vec<f64> = (0..n).map(|i| (i as f64 * 1.3).sin() * 5.0 + 6.0).collect();
    vec![("x".into(), s(&x)), ("y".into(), f(&y))]
}

/// 16 discrete groups: a one-row horizontal legend overflows a 320px-wide
/// panel under 0.21.0's single-line legend (B3-2 territory).
fn big_legend_data() -> Cols {
    let n = 16;
    let x: Vec<f64> = (0..n).map(|i| i as f64).collect();
    let y: Vec<f64> = (0..n).map(|i| (i as f64 * 0.5).sin()).collect();
    let g: Vec<String> = (0..n).map(|i| format!("Group {i:02}")).collect();
    let g: Vec<&str> = g.iter().map(|s| s.as_str()).collect();
    vec![
        ("x".into(), f(&x)),
        ("y".into(), f(&y)),
        ("g".into(), s(&g)),
    ]
}

/// `(name, expected FNV-1a hash)`. Recorded at v0.21.0 (f5fc31e).
/// To re-bless an entry: confirm it is in plan §4's allowed list, change it
/// in the commit that causes it, and add a CHANGELOG "Changed" line.
const EXPECTED: &[(&str, u64)] = &[
    ("point", 0x35b9a09e98ae7ef2),
    ("line", 0x8fb3a52646bc0c5d),
    ("segment", 0x68fcc015406c8eb9),
    ("bar", 0x14377a5e4f3d4e62),
    ("col", 0xc5738d84e879e5f8),
    ("histogram", 0x509a73b585ca7049),
    ("boxplot", 0x1e4e90592a30f608),
    ("area", 0xacba2a5cebd1c91b),
    ("ribbon", 0x7bf3af705d10271f),
    ("step", 0x4cceadf6cd3d5452),
    ("density", 0x055db381e3b7d8e5),
    ("tile", 0x873b1d2d936ab090),
    ("smooth", 0xf2498f530195746b),
    ("polar", 0xe35758d04bde28b5),
    ("facet", 0xfd878f16570d2b78),
    ("facet_stack", 0xbdd92015dcb54b59),
    ("flip", 0x94fdd6d55d9a7be1),
    ("text", 0x9fc585729f348a51),
    ("text_dodge", 0xb73b3b9f8236fe35),
    ("violin", 0x68cd485311890ec4),
    ("errorbar", 0xe7abdb6d7e210a85),
    ("errorbar_dodge", 0xb84cd4b0733c711c),
    ("size", 0xa5314ef4d0585c40),
    ("wide_axis", 0x9d4188248632b19f),
    ("big_legend", 0x0a625a0466236f03),
];

#[test]
fn default_output_is_byte_identical_to_v0_21_0() {
    let cases: Vec<(&str, u64)> = vec![
        (
            "point",
            hash_svg(
                GGPlot::new(xy_grouped())
                    .aes(Aes::new().x("x").y("y").color("g"))
                    .geom_point(),
                480,
                320,
                "point",
            ),
        ),
        (
            "line",
            hash_svg(
                GGPlot::new(xy_grouped())
                    .aes(Aes::new().x("x").y("y").color("g"))
                    .geom_line(),
                480,
                320,
                "line",
            ),
        ),
        (
            "segment",
            hash_svg(
                GGPlot::new(seg_data())
                    .aes(Aes::new().x("x").y("y").xend("xend").yend("yend"))
                    .geom_segment(),
                480,
                320,
                "segment",
            ),
        ),
        (
            "bar",
            hash_svg(
                GGPlot::new(bar_data()).aes(Aes::new().x("cat")).geom_bar(),
                480,
                320,
                "bar",
            ),
        ),
        (
            "col",
            hash_svg(
                GGPlot::new(bar_data())
                    .aes(Aes::new().x("cat").y("val").fill("grp"))
                    .geom_col(),
                480,
                320,
                "col",
            ),
        ),
        (
            "histogram",
            hash_svg(
                GGPlot::new(hist_data())
                    .aes(Aes::new().x("y"))
                    .geom_histogram(),
                480,
                320,
                "histogram",
            ),
        ),
        (
            "boxplot",
            hash_svg(
                GGPlot::new(boxplot_data())
                    .aes(Aes::new().x("g").y("y"))
                    .geom_boxplot(),
                480,
                320,
                "boxplot",
            ),
        ),
        (
            "area",
            hash_svg(
                GGPlot::new(band_data())
                    .aes(Aes::new().x("x").y("y"))
                    .geom_area(),
                480,
                320,
                "area",
            ),
        ),
        (
            "ribbon",
            hash_svg(
                GGPlot::new(band_data())
                    .aes(Aes::new().x("x").ymin("ymin").ymax("ymax"))
                    .geom_ribbon(),
                480,
                320,
                "ribbon",
            ),
        ),
        (
            "step",
            hash_svg(
                GGPlot::new(band_data())
                    .aes(Aes::new().x("x").y("y"))
                    .geom_step(),
                480,
                320,
                "step",
            ),
        ),
        (
            "density",
            hash_svg(
                GGPlot::new(hist_data())
                    .aes(Aes::new().x("y"))
                    .geom_density(),
                480,
                320,
                "density",
            ),
        ),
        (
            "tile",
            hash_svg(
                GGPlot::new(tile_data())
                    .aes(Aes::new().x("x").y("y").fill("v"))
                    .geom_tile(),
                480,
                320,
                "tile",
            ),
        ),
        (
            "smooth",
            hash_svg(
                GGPlot::new(xy_grouped())
                    .aes(Aes::new().x("x").y("y"))
                    .geom_point()
                    .geom_smooth(),
                480,
                320,
                "smooth",
            ),
        ),
        (
            "polar",
            hash_svg(
                GGPlot::new(bar_data())
                    .aes(Aes::new().x("cat").y("val"))
                    .geom_col()
                    .coord_polar(),
                480,
                320,
                "polar",
            ),
        ),
        (
            "facet",
            hash_svg(
                GGPlot::new(facet_data())
                    .aes(Aes::new().x("x").y("y"))
                    .geom_point()
                    .facet_wrap("panel", None),
                600,
                320,
                "facet",
            ),
        ),
        (
            "facet_stack",
            hash_svg(
                GGPlot::new(facet_stack_data())
                    .aes(Aes::new().x("x").y("y").fill("grp"))
                    .geom_col()
                    .position(ggplot_rs::position::stack::PositionStack)
                    .facet_wrap("panel", None),
                600,
                320,
                "facet_stack",
            ),
        ),
        (
            "flip",
            hash_svg(
                GGPlot::new(bar_data())
                    .aes(Aes::new().x("cat").y("val"))
                    .geom_col()
                    .coord_flip(),
                480,
                320,
                "flip",
            ),
        ),
        (
            "text",
            hash_svg(
                GGPlot::new(text_data())
                    .aes(Aes::new().x("x").y("y").label("lbl"))
                    .geom_text(),
                480,
                320,
                "text",
            ),
        ),
        (
            "text_dodge",
            hash_svg(
                GGPlot::new(text_dodge_data())
                    .aes(
                        Aes::new()
                            .x("x")
                            .y("y")
                            .fill("grp")
                            .group("grp")
                            .label("lbl"),
                    )
                    .geom_col()
                    .position(ggplot_rs::position::dodge::position_dodge(0.9))
                    .geom_text()
                    .position(ggplot_rs::position::dodge::position_dodge(0.9)),
                480,
                320,
                "text_dodge",
            ),
        ),
        (
            "violin",
            hash_svg(
                GGPlot::new(boxplot_data())
                    .aes(Aes::new().x("g").y("y"))
                    .geom_violin(),
                480,
                320,
                "violin",
            ),
        ),
        (
            "errorbar",
            hash_svg(
                GGPlot::new(errorbar_data())
                    .aes(Aes::new().x("x").ymin("ymin").ymax("ymax"))
                    .geom_errorbar(),
                480,
                320,
                "errorbar",
            ),
        ),
        (
            "errorbar_dodge",
            hash_svg(
                GGPlot::new(errorbar_dodge_data())
                    .aes(
                        Aes::new()
                            .x("x")
                            .ymin("ymin")
                            .ymax("ymax")
                            .group("grp")
                            .color("grp"),
                    )
                    .geom_errorbar()
                    .position(ggplot_rs::position::dodge::position_dodge(0.9)),
                480,
                320,
                "errorbar_dodge",
            ),
        ),
        (
            "size",
            hash_svg(
                GGPlot::new(size_data())
                    .aes(Aes::new().x("x").y("y").size("v"))
                    .geom_point(),
                480,
                320,
                "size",
            ),
        ),
        (
            "wide_axis",
            hash_svg(
                GGPlot::new(wide_axis_data())
                    .aes(Aes::new().x("x").y("y"))
                    .geom_col(),
                300,
                260,
                "wide_axis",
            ),
        ),
        (
            "big_legend",
            hash_svg(
                GGPlot::new(big_legend_data())
                    .aes(Aes::new().x("x").y("y").color("g"))
                    .geom_point(),
                320,
                400,
                "big_legend",
            ),
        ),
    ];

    assert_eq!(
        cases.len(),
        EXPECTED.len(),
        "case count drifted from the recorded table"
    );
    assert_eq!(cases.len(), 25, "D1 calls for 25 fixed plots");

    let mut mismatches = Vec::new();
    for ((name, got), (exp_name, exp_hash)) in cases.iter().zip(EXPECTED.iter()) {
        assert_eq!(name, exp_name, "case order drifted from the recorded table");
        if got != exp_hash {
            mismatches.push(format!(
                "{name}: expected {exp_hash:#018x}, got {got:#018x}"
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "default-output hash mismatch (see plan §4 for the allowed list):\n{}",
        mismatches.join("\n")
    );
}
