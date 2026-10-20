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
        ("ymin", floats(&[1.0, 1.0])),
        ("ymax", floats(&[8.0, 8.0])),
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

/// Regression for review #158 finding 4: a SQL NULL (`Value::Na`, as
/// opposed to a `Float(NaN)`) in a boxplot position column used to be
/// silently rendered as 0 with no warning at all (the reviewer's repro: a
/// `lower` of `Value::Na` drew a box titled "B: median 4 (0–5)"). It must
/// now be dropped with the exact same "non-finite values" warning a NaN
/// gets — row B must not appear at all, and the warning must mention it.
#[test]
fn na_lower_row_sql_null_is_dropped_not_drawn_at_zero() {
    let (svg, warnings) = GGPlot::new(frame(vec![
        ("x", strs(&["A", "B"])),
        ("lower", vec![Value::Float(3.0), Value::Na]),
        ("middle", floats(&[5.0, 4.0])),
        ("upper", floats(&[7.0, 5.0])),
        ("ymin", floats(&[1.0, 1.0])),
        ("ymax", floats(&[8.0, 8.0])),
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
    .render_svg_native_with_warnings(300, 300)
    .unwrap();
    // The dropped row's box must not render at all — specifically not as
    // a box "from 0" (the pre-fix bug).
    assert!(
        !svg.contains("median 4 (0"),
        "row B must not silently render from 0: {svg}"
    );
    assert!(
        !svg.contains(">B<"),
        "row B's category must not appear: {svg}"
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("geom_boxplot") && w.contains("non-finite values")),
        "{warnings:?}"
    );
}

/// A `Value::Na` in a boxplot's `middle`/`upper` is dropped the same way.
#[test]
fn na_middle_and_upper_rows_are_also_dropped() {
    let (svg, warnings) = GGPlot::new(frame(vec![
        ("x", strs(&["A", "B", "C"])),
        ("lower", floats(&[3.0, 3.0, 3.0])),
        (
            "middle",
            vec![Value::Float(5.0), Value::Na, Value::Float(5.0)],
        ),
        (
            "upper",
            vec![Value::Float(7.0), Value::Float(7.0), Value::Na],
        ),
        ("ymin", floats(&[1.0, 1.0, 1.0])),
        ("ymax", floats(&[8.0, 8.0, 8.0])),
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
    .render_svg_native_with_warnings(300, 300)
    .unwrap();
    assert!(svg.contains(">A<"), "row A (fully numeric) must survive");
    assert!(!svg.contains(">B<"), "row B (Na middle) must be dropped");
    assert!(!svg.contains(">C<"), "row C (Na upper) must be dropped");
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
        ("ymin", floats(&[1.0])),
        ("ymax", floats(&[8.0])),
        ("grade", strs(&["B"])),
    ]))
    .aes(
        Aes::new()
            .x("x")
            .lower("lower")
            .middle("middle")
            .upper("upper")
            .ymin("ymin")
            .ymax("ymax")
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

/// Review #158 finding 5: under `stat = "identity"`, ggplot2 4.0.3 also
/// requires `ymin`/`ymax` (plan §0's R table), not just `lower`/`middle`/
/// `upper` — a layer missing them used to render whisker-less boxes with
/// no error at all.
#[test]
fn missing_ymin_ymax_under_identity_is_a_validation_error() {
    let result = GGPlot::new(frame(vec![
        ("x", strs(&["A"])),
        ("lower", floats(&[3.0])),
        ("middle", floats(&[5.0])),
        ("upper", floats(&[7.0])),
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
    .render_svg_native_with_size(300, 300);
    assert!(
        matches!(result, Err(GGError::ValidationError(_))),
        "{result:?}"
    );
}

/// A numeric (not comma-list-string) `outliers` cell is a single outlier,
/// not silently ignored (review #158 finding 5).
#[test]
fn numeric_outliers_cell_is_treated_as_a_single_outlier() {
    let (svg, domain) = render_with_ydomain(
        GGPlot::new(frame(vec![
            ("x", strs(&["A"])),
            ("lower", floats(&[3.0])),
            ("middle", floats(&[5.0])),
            ("upper", floats(&[7.0])),
            ("ymin", floats(&[1.0])),
            ("ymax", floats(&[8.0])),
            ("outliers", floats(&[50.0])),
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
    assert_eq!(svg.matches("<circle ").count(), 1, "{svg}");
    assert!(
        domain.1 >= 50.0 - 1e-6,
        "a numeric outliers cell must still widen y: {domain:?}"
    );
}

/// Review #158 finding 5 (the `ymin_final`/`ymax_final` leak): these
/// internal outlier-extent columns are trained into the y scale at build
/// Step 8b for *every* layer. Before routing them through an internal-
/// prefix name, a raw source column that happened to be named
/// `ymin_final` — unrelated to any boxplot — silently widened an
/// unrelated geom's y domain (the reviewer's repro: `geom_point` with a
/// raw `ymin_final = -500` gave a y domain of roughly -527..77). A raw
/// column with that exact name must now be inert.
#[test]
fn raw_ymin_final_column_does_not_leak_into_an_unrelated_geoms_y_domain() {
    let (_, domain) = render_with_ydomain(
        GGPlot::new(frame(vec![
            ("x", floats(&[1.0, 2.0, 3.0])),
            ("y", floats(&[10.0, 20.0, 30.0])),
            ("ymin_final", floats(&[-500.0, -500.0, -500.0])),
        ]))
        .aes(Aes::new().x("x").y("y"))
        .geom_point(),
        300,
        300,
    );
    assert!(
        domain.0 > -100.0,
        "a raw, unmapped `ymin_final` column must not widen geom_point's y domain: {domain:?}"
    );
}

/// Regression for review #158's re-review (HIGH): `lower`/`middle`/`upper`
/// landed in `POSITION_COLS`/`Y_FAMILY`/the stat_aes training list
/// unconditionally — not just for `geom_boxplot` — so a plain `geom_line`/
/// `geom_point` layer over forecast-shaped data with unmapped `lower`/
/// `upper` confidence-bound columns (`Na` on the history rows, as in the
/// reviewer's repro) lost its history rows to the non-finite-value drop.
/// All 6 history points must survive, with no warning, and the y domain
/// must reflect only the mapped `y` data — not the unmapped bounds.
#[test]
fn forecast_shaped_unmapped_lower_upper_columns_do_not_affect_a_plain_line() {
    let y = [10.0, 9.9, 11.2, 10.5, 12.1, 11.8];
    let n = y.len();
    let (svg, warnings) = GGPlot::new(frame(vec![
        ("x", floats(&(0..n).map(|i| i as f64).collect::<Vec<_>>())),
        ("y", floats(&y)),
        ("lower", vec![Value::Na; n]),
        ("upper", vec![Value::Na; n]),
    ]))
    .aes(Aes::new().x("x").y("y"))
    .geom_line()
    .geom_point()
    .render_svg_native_with_warnings(300, 200)
    .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(
        svg.matches("<circle ").count(),
        n,
        "all {n} history points must survive: {svg}"
    );
    let dom = svg
        .split("data-ydomain=\"")
        .nth(1)
        .and_then(|r| r.split('"').next())
        .unwrap();
    let hi: f64 = dom.split(' ').nth(1).unwrap().parse().unwrap();
    assert!(
        hi < 20.0,
        "the y domain must reflect only the mapped y data (9.9-12.1-ish), not a stretch: {dom}"
    );
}

/// The same, with unmapped `lower`/`upper` holding real (not `Na`) but
/// wildly out-of-range values — the y domain must still come from the
/// mapped `y` column alone, not be stretched by the unmapped bounds.
#[test]
fn forecast_shaped_unmapped_wide_lower_upper_values_do_not_stretch_y() {
    let y = [10.0, 10.5, 11.0, 11.5, 12.0];
    let n = y.len();
    let (_, domain) = render_with_ydomain(
        GGPlot::new(frame(vec![
            ("x", floats(&(0..n).map(|i| i as f64).collect::<Vec<_>>())),
            ("y", floats(&y)),
            ("lower", floats(&vec![-100.0; n])),
            ("upper", floats(&vec![500.0; n])),
        ]))
        .aes(Aes::new().x("x").y("y"))
        .geom_line(),
        300,
        200,
    );
    assert!(
        domain.0 > -20.0 && domain.1 < 30.0,
        "unmapped lower=-100/upper=500 must not stretch y from ~9.9-12.1: {domain:?}"
    );
}

/// Regression for review #158's final check: round 3's universal
/// `MAPPED_ONLY_COLS` drop removed a raw, unmapped `upper` column from the
/// layer's data entirely — so `facet_wrap("upper")` on a forecast-shaped
/// plot found no such column by the time faceting ran, and rendered a
/// single, unfaceted panel instead of splitting on it. The column must
/// stay in the data (it's just excluded from the position/non-finite/
/// training concerns the other two tests above cover), so a plain
/// `facet_wrap` on it works exactly like any other unmapped column.
#[test]
fn facet_wrap_on_an_unmapped_upper_column_still_splits_into_two_panels() {
    let built = GGPlot::new(frame(vec![
        ("x", floats(&[0.0, 1.0, 2.0, 3.0])),
        ("y", floats(&[10.0, 11.0, 10.5, 11.5])),
        ("upper", strs(&["A", "A", "B", "B"])),
    ]))
    .aes(Aes::new().x("x").y("y"))
    .geom_line()
    .geom_point()
    .facet_wrap("upper", None)
    .build();
    assert_eq!(
        built.panels.len(),
        2,
        "facet_wrap(\"upper\") must still split into 2 panels: {:?}",
        built.panels.iter().map(|p| &p.label).collect::<Vec<_>>()
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
