//! B3-5: `geom_boxplot(stat = "identity")` — new `Lower`/`Middle`/`Upper`/
//! `Outliers` aesthetics, central validation (D3's mapped-only-column
//! drop), tooltips/data_attr, and the outlier-extent fix (allowed #6).

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;
use ggplot_rs::stat::boxplot::StatBoxplot;
use ggplot_rs::stat::identity::StatIdentity;
use ggplot_rs::stat::Stat;

fn floats(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&f| Value::Float(f)).collect()
}
fn strs(v: &[&str]) -> Vec<Value> {
    v.iter().map(|s| Value::Str(s.to_string())).collect()
}
fn frame(cols: Vec<(&str, Vec<Value>)>) -> Vec<(String, Vec<Value>)> {
    cols.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

fn raw_group_a() -> Vec<f64> {
    vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]
}

/// The identity path (user supplies the five-number summary) renders the
/// same box/whisker pixels as the default stat path (ggplot-rs computes
/// the summary itself) for the same underlying data.
#[test]
fn identity_path_matches_the_stat_paths_pixels() {
    let y = raw_group_a();
    let default_stat_svg = GGPlot::new(frame(vec![("x", strs(&["A"; 10])), ("y", floats(&y))]))
        .aes(Aes::new().x("x").y("y"))
        .geom_boxplot()
        .render_svg_native_with_size(300, 300)
        .unwrap();

    // Ground truth from the stat itself, fed back through the identity
    // path — these must render identically.
    let mut df = ggplot_rs::data::DataFrame::new();
    df.add_column("x".into(), vec![Value::Str("A".into()); 10]);
    df.add_column("y".into(), floats(&y));
    let computed = StatBoxplot.compute_group(&df, &ggplot_rs::scale::ScaleSet::new());
    let get = |col: &str| computed.column(col).unwrap()[0].as_f64().unwrap();
    let (lower, middle, upper, ymin, ymax) = (
        get("lower"),
        get("middle"),
        get("upper"),
        get("ymin"),
        get("ymax"),
    );

    let identity_svg = GGPlot::new(frame(vec![
        ("x", strs(&["A"])),
        ("lower", floats(&[lower])),
        ("middle", floats(&[middle])),
        ("upper", floats(&[upper])),
        ("ymin", floats(&[ymin])),
        ("ymax", floats(&[ymax])),
    ]))
    .aes(
        Aes::new()
            .x("x")
            .lower("lower")
            .middle("middle")
            .upper("upper")
            .ymin("ymin")
            .ymax("ymax"),
    )
    .geom_boxplot()
    .stat(StatIdentity)
    .render_svg_native_with_size(300, 300)
    .unwrap();

    assert_eq!(default_stat_svg, identity_svg);
}

/// Outliers "7, 9" (both outside the box's whiskers) draw as circles and
/// widen the trained y domain to include them (allowed #6 — the test that
/// confirms the bug before the fix: see `outlier_extent_is_the_confirmed_fix`
/// below for the before/after demonstration).
#[test]
fn outliers_draw_and_widen_y() {
    let (svg, domain) = render_with_ydomain(
        GGPlot::new(frame(vec![
            ("x", strs(&["A"])),
            ("lower", floats(&[3.0])),
            ("middle", floats(&[5.0])),
            ("upper", floats(&[7.0])),
            ("ymin", floats(&[1.0])),
            ("ymax", floats(&[8.0])),
            ("outliers", strs(&["7, 9"])),
        ]))
        .aes(
            Aes::new()
                .x("x")
                .lower("lower")
                .middle("middle")
                .upper("upper")
                .ymin("ymin")
                .ymax("ymax")
                .outliers("outliers"),
        )
        .geom_boxplot()
        .stat(StatIdentity),
        300,
        300,
    );
    assert_eq!(svg.matches("<circle ").count(), 2, "{svg}");
    assert!(
        domain.1 >= 9.0 - 1e-6,
        "y domain should widen to 9: {domain:?}"
    );
}

/// Confirms the pre-B3-5 bug directly: an outlier beyond ymax is NOT
/// included in the trained y domain unless `setup_data` adds
/// `ymin_final`/`ymax_final` — i.e. this is the "write a failing test
/// first" check for allowed #6, kept in the suite as a regression guard
/// (it now passes because the fix landed).
#[test]
fn outlier_extent_is_the_confirmed_fix() {
    let (_, domain) = render_with_ydomain(
        GGPlot::new(frame(vec![
            ("x", strs(&["A"])),
            ("lower", floats(&[3.0])),
            ("middle", floats(&[5.0])),
            ("upper", floats(&[7.0])),
            ("ymin", floats(&[1.0])),
            ("ymax", floats(&[8.0])),
            ("outliers", strs(&["100"])),
        ]))
        .aes(
            Aes::new()
                .x("x")
                .lower("lower")
                .middle("middle")
                .upper("upper")
                .ymin("ymin")
                .ymax("ymax")
                .outliers("outliers"),
        )
        .geom_boxplot()
        .stat(StatIdentity),
        300,
        300,
    );
    // Without the fix the domain tops out at ymax (8); with it, at 100.
    assert!(
        domain.1 >= 100.0 - 1e-6,
        "the y domain must widen to the far outlier: {domain:?}"
    );
}

/// A raw, unmapped `lower` column under `stat = "identity"` doesn't
/// satisfy the requirement — D3's mapped-only-column drop removes it
/// before validation even sees it, so this is a `ValidationError`, not a
/// silent misread.
#[test]
fn raw_unmapped_lower_under_identity_is_a_validation_error() {
    let result = GGPlot::new(frame(vec![
        ("x", strs(&["A"])),
        ("lower", floats(&[3.0])), // present, but never mapped
        ("middle", floats(&[5.0])),
        ("upper", floats(&[7.0])),
    ]))
    .aes(Aes::new().x("x"))
    .geom_boxplot()
    .stat(StatIdentity)
    .render_svg_native_with_size(300, 300);
    assert!(
        matches!(result, Err(GGError::ValidationError(_))),
        "{result:?}"
    );
}

/// A raw `outliers` column under the *default* (non-identity) boxplot
/// stat is dropped (D3) and ignored — the stat computes its own.
#[test]
fn raw_outliers_under_the_default_stat_is_ignored() {
    let svg = GGPlot::new(frame(vec![
        ("x", strs(&["A"; 10])),
        ("y", floats(&raw_group_a())),
        ("outliers", strs(&["not,real,data"; 10])),
    ]))
    .aes(Aes::new().x("x").y("y"))
    .geom_boxplot()
    .render_svg_native_with_size(300, 300)
    .unwrap();
    assert!(svg.starts_with("<svg"));
    assert!(!svg.contains("NaN"));
}

/// An NA `lower` row under identity is dropped with the same "removed N
/// row(s) containing non-finite values" warning every other geom's
/// position columns get.
#[test]
fn na_lower_row_is_dropped_with_the_standard_warning() {
    let (svg, warnings) = GGPlot::new(frame(vec![
        ("x", strs(&["A", "B"])),
        ("lower", vec![Value::Float(f64::NAN), Value::Float(3.0)]),
        ("middle", floats(&[5.0, 5.0])),
        ("upper", floats(&[7.0, 7.0])),
    ]))
    .aes(
        Aes::new()
            .x("x")
            .lower("lower")
            .middle("middle")
            .upper("upper"),
    )
    .geom_boxplot()
    .stat(StatIdentity)
    .render_svg_native_with_warnings(300, 300)
    .unwrap();
    assert!(!svg.contains("NaN"));
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("geom_boxplot") && w.contains("non-finite values")),
        "{warnings:?}"
    );
}

/// `tooltip()` and `data_attr()` both reach the box mark now that
/// `consumes_tooltips` is true and `draw` calls `set_row_extras`.
#[test]
fn tooltip_and_data_attr_reach_the_box() {
    let svg = GGPlot::new(frame(vec![
        ("x", strs(&["A"])),
        ("lower", floats(&[3.0])),
        ("middle", floats(&[5.0])),
        ("upper", floats(&[7.0])),
        ("grade", strs(&["B"])),
    ]))
    .aes(
        Aes::new()
            .x("x")
            .lower("lower")
            .middle("middle")
            .upper("upper")
            .data_attr("grade", "grade"),
    )
    .geom_boxplot()
    .stat(StatIdentity)
    .tooltip("box {x}: {middle}")
    .render_svg_native_with_size(300, 300)
    .unwrap();
    assert!(svg.contains("<title>box A: 5</title>"), "{svg}");
    assert!(svg.contains("data-grade=\"B\""), "{svg}");
}

/// The five-number summary, read back through a tooltip template, formats
/// every position placeholder through the y scale (D9 allowed #7) — shown
/// here under `scale_y_log10`, where the raw built values are in log
/// space but the template must show the human-readable (inverse-
/// transformed) numbers, the same way `y`/`ymin` already did.
#[test]
fn five_number_template_formats_through_the_y_scale_including_log10() {
    let plain = GGPlot::new(frame(vec![
        ("x", strs(&["A"])),
        ("lower", floats(&[10.0])),
        ("middle", floats(&[100.0])),
        ("upper", floats(&[1000.0])),
        ("ymin", floats(&[1.0])),
        ("ymax", floats(&[10000.0])),
    ]))
    .aes(
        Aes::new()
            .x("x")
            .lower("lower")
            .middle("middle")
            .upper("upper")
            .ymin("ymin")
            .ymax("ymax"),
    )
    .geom_boxplot()
    .stat(StatIdentity)
    .tooltip("{ymin}/{lower}/{middle}/{upper}/{ymax}")
    .render_svg_native_with_size(300, 300)
    .unwrap();
    assert!(
        plain.contains("<title>1/10/100/1000/10000</title>"),
        "{plain}"
    );

    let log10 = GGPlot::new(frame(vec![
        ("x", strs(&["A"])),
        ("lower", floats(&[10.0])),
        ("middle", floats(&[100.0])),
        ("upper", floats(&[1000.0])),
        ("ymin", floats(&[1.0])),
        ("ymax", floats(&[10000.0])),
    ]))
    .aes(
        Aes::new()
            .x("x")
            .lower("lower")
            .middle("middle")
            .upper("upper")
            .ymin("ymin")
            .ymax("ymax"),
    )
    .geom_boxplot()
    .stat(StatIdentity)
    .scale_y_log10()
    .tooltip("{ymin}/{lower}/{middle}/{upper}/{ymax}")
    .render_svg_native_with_size(300, 300)
    .unwrap();
    // Still the human-readable numbers (not 0/1/2/3/4, the log10 of each).
    assert!(
        log10.contains("<title>1/10/100/1000/10000</title>"),
        "{log10}"
    );
}

/// Render helper that also extracts `data-ydomain="y0 y1"` (the trained y
/// range actually used to position marks).
fn render_with_ydomain(plot: GGPlot, w: u32, h: u32) -> (String, (f64, f64)) {
    let svg = plot.render_svg_native_with_size(w, h).unwrap();
    let start = svg.find("data-ydomain=\"").unwrap() + "data-ydomain=\"".len();
    let rest = &svg[start..];
    let end = rest.find('"').unwrap();
    let mut it = rest[..end]
        .split_whitespace()
        .map(|s| s.parse::<f64>().unwrap());
    let domain = (it.next().unwrap(), it.next().unwrap());
    (svg, domain)
}
