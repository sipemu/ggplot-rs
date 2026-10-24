//! B2-3: `GGPlot::svg_attr`/`svg_attrs`/`svg_warnings_attr` and the same API
//! on `PlotGrid` — caller-supplied root `<svg>` attributes, strictly
//! allow-listed and never duplicating what ggplot-rs itself writes.

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;

fn floats(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&f| Value::Float(f)).collect()
}
fn frame(cols: Vec<(&str, Vec<Value>)>) -> Vec<(String, Vec<Value>)> {
    cols.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}
fn data() -> Vec<(String, Vec<Value>)> {
    frame(vec![
        ("x", floats(&[1.0, 2.0, 3.0])),
        ("y", floats(&[1.0, 2.0, 3.0])),
    ])
}

/// The value of attribute `name` on the root `<svg …>` tag.
fn root_attr<'a>(svg: &'a str, name: &str) -> Option<&'a str> {
    let head = &svg[..svg.find('>').unwrap()];
    let key = format!(" {name}=\"");
    let i = head.find(&key)? + key.len();
    Some(&head[i..i + head[i..].find('"').unwrap()])
}

fn base_plot() -> GGPlot {
    GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
}

#[test]
fn an_allowed_attribute_round_trips_on_the_root_svg() {
    for (name, value) in [
        ("data-host", "dashboard-7"),
        ("aria-label", "sales trend"),
        ("role", "img"),
        ("id", "chart-1"),
        ("class", "widget"),
        ("lang", "de-DE"),
    ] {
        let svg = base_plot()
            .svg_attr(name, value)
            .render_svg_native_with_size(300, 200)
            .unwrap();
        assert_eq!(root_attr(&svg, name), Some(value), "{name}: {svg}");
    }
}

#[test]
fn svg_attrs_sets_several_at_once() {
    let svg = base_plot()
        .svg_attrs(vec![
            ("data-a".to_string(), "1".to_string()),
            ("data-b".to_string(), "2".to_string()),
        ])
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert_eq!(root_attr(&svg, "data-a"), Some("1"));
    assert_eq!(root_attr(&svg, "data-b"), Some("2"));
}

#[test]
fn a_disallowed_name_is_ignored_with_a_warning() {
    for bad in ["onclick", "style", "data-Upper", "tabindex", "xlink:href"] {
        let (svg, warnings) = base_plot()
            .svg_attr(bad, "x")
            .render_svg_native_with_warnings(300, 200)
            .unwrap();
        assert_eq!(root_attr(&svg, bad), None, "{bad} leaked into: {svg}");
        assert!(
            warnings.iter().any(|w| w.contains(bad)),
            "{bad}: no warning in {warnings:?}"
        );
    }
}

#[test]
fn a_name_ggplot_rs_already_writes_is_ignored_not_overwritten() {
    // data-plot is written automatically (svg_root_data defaults to true);
    // the caller's value must not clobber it.
    let (svg, warnings) = base_plot()
        .svg_attr("data-plot", "forged")
        .render_svg_native_with_warnings(300, 200)
        .unwrap();
    assert_ne!(root_attr(&svg, "data-plot"), Some("forged"));
    assert!(warnings.iter().any(|w| w.contains("data-plot")));
}

#[test]
fn a_second_svg_attr_call_for_the_same_name_blames_the_caller_not_ggplot_rs() {
    // Review PR#141 nit #9: a *second* svg_attr("class", ...) collides with
    // the caller's own earlier call, not with something ggplot-rs itself
    // writes to the root — the warning text must say so, distinctly from
    // `a_name_ggplot_rs_already_writes_is_ignored_not_overwritten` above.
    let (svg, warnings) = base_plot()
        .svg_attr("class", "first")
        .svg_attr("class", "second")
        .render_svg_native_with_warnings(300, 200)
        .unwrap();
    assert_eq!(root_attr(&svg, "class"), Some("first"));
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("class") && w.contains("already set by an earlier svg_attr")),
        "{warnings:?}"
    );
    assert!(
        !warnings
            .iter()
            .any(|w| w.contains("class") && w.contains("ggplot-rs already writes")),
        "must not blame ggplot-rs for the caller's own duplicate: {warnings:?}"
    );
}

#[test]
fn values_are_escaped() {
    let svg = base_plot()
        .svg_attr("data-note", "<script>\"&'")
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(!svg.contains("<script>"));
    assert!(svg.contains("&lt;script&gt;"));
}

#[test]
fn svg_warnings_attr_emits_a_json_array_off_by_default() {
    let data = frame(vec![
        ("x", floats(&[1.0, f64::NAN, 3.0])),
        ("y", floats(&[1.0, 2.0, 3.0])),
    ]);
    let without = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert_eq!(root_attr(&without, "data-warnings"), None);

    let with = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .svg_warnings_attr(true)
        .render_svg_native_with_size(300, 200)
        .unwrap();
    let json = root_attr(&with, "data-warnings").expect("data-warnings present");
    assert!(json.starts_with('['), "{json}");
    assert!(json.contains("non-finite"), "{json}");
}

#[test]
fn svg_warnings_attr_is_empty_array_when_there_are_no_warnings() {
    let svg = base_plot()
        .svg_warnings_attr(true)
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert_eq!(root_attr(&svg, "data-warnings"), Some("[]"));
}

#[test]
fn a_user_data_warnings_attr_collides_with_the_automatic_one() {
    let (svg, warnings) = base_plot()
        .svg_attr("data-warnings", "forged")
        .svg_warnings_attr(true)
        .render_svg_native_with_warnings(300, 200)
        .unwrap();
    assert_ne!(root_attr(&svg, "data-warnings"), Some("forged"));
    assert!(warnings.iter().any(|w| w.contains("data-warnings")));
}

#[test]
fn plotgrid_has_the_same_svg_attr_api() {
    let p = || base_plot();
    let svg = (p() | p())
        .svg_attr("data-host", "dashboard")
        .render_svg_native_with_size(600, 300)
        .unwrap();
    assert_eq!(root_attr(&svg, "data-host"), Some("dashboard"));

    // data-grid is written automatically; the caller's value is ignored.
    let (svg, warnings) = (p() | p())
        .svg_attr("data-grid", "forged")
        .render_svg_native_with_warnings(600, 300)
        .unwrap();
    assert_ne!(root_attr(&svg, "data-grid"), Some("forged"));
    assert!(warnings.iter().any(|w| w.contains("data-grid")));
}

#[test]
fn plotgrid_svg_warnings_attr_works_too() {
    let p = || base_plot();
    let svg = (p() | p())
        .svg_warnings_attr(true)
        .render_svg_native_with_size(600, 300)
        .unwrap();
    assert_eq!(root_attr(&svg, "data-warnings"), Some("[]"));
}

#[test]
fn without_any_svg_attr_calls_output_is_unaffected() {
    let svg = base_plot().render_svg_native_with_size(300, 200).unwrap();
    assert!(!svg.contains("data-warnings"));
}

/// The value of attribute `name` on the `n`-th (0-based) `<g ...>` tag.
fn group_attr<'a>(svg: &'a str, n: usize, name: &str) -> Option<&'a str> {
    let tag_start = svg.match_indices("<g ").nth(n)?.0;
    let head = &svg[tag_start..tag_start + svg[tag_start..].find('>').unwrap()];
    let key = format!(" {name}=\"");
    let i = head.find(&key)? + key.len();
    Some(&head[i..i + head[i..].find('"').unwrap()])
}

/// sipemu/ggplot-rs#208: a cell's own `svg_attr` used to be silently
/// dropped once composed into a `PlotGrid` — found via
/// DataZooDE/anofox-visualization#422, where every `::TRACK` cell's
/// `data-track` attribute never reached the composed SVG, and the browser
/// (which reads it) never saw it. Each cell's own attributes now land on a
/// `<g>` wrapping that cell's content; the grid's own `svg_attr`s (set on
/// the `PlotGrid` itself, as in `plotgrid_has_the_same_svg_attr_api`
/// above) still land on the grid's root, and neither leaks into the
/// other.
#[test]
fn a_cells_own_svg_attr_survives_plotgrid_composition() {
    let svg = (base_plot().svg_attr("data-track", "true")
        | base_plot().svg_attr("data-track", "false"))
    .render_svg_native_with_size(600, 300)
    .unwrap();
    assert_eq!(group_attr(&svg, 0, "data-track"), Some("true"), "{svg}");
    assert_eq!(group_attr(&svg, 1, "data-track"), Some("false"), "{svg}");
    // Not duplicated on the grid's own root.
    assert_eq!(root_attr(&svg, "data-track"), None, "{svg}");
}

/// Two *different* names on two cells, to rule out one cell's attribute
/// leaking onto the other's `<g>` rather than just being present somewhere
/// in the document.
#[test]
fn distinct_per_cell_svg_attrs_each_land_on_their_own_cell_only() {
    let svg = (base_plot().svg_attr("data-a", "1") | base_plot().svg_attr("data-b", "2"))
        .render_svg_native_with_size(600, 300)
        .unwrap();
    assert_eq!(group_attr(&svg, 0, "data-a"), Some("1"), "{svg}");
    assert_eq!(
        group_attr(&svg, 0, "data-b"),
        None,
        "cell 0 got cell 1's attr: {svg}"
    );
    assert_eq!(group_attr(&svg, 1, "data-b"), Some("2"), "{svg}");
    assert_eq!(
        group_attr(&svg, 1, "data-a"),
        None,
        "cell 1 got cell 0's attr: {svg}"
    );
}

/// A cell without any `svg_attr` call gets no `<g>` wrapper at all — same
/// "unaffected when unused" guarantee as the plot-level API
/// (`without_any_svg_attr_calls_output_is_unaffected` above).
#[test]
fn a_cell_without_svg_attr_gets_no_group_wrapper() {
    let svg = (base_plot() | base_plot())
        .render_svg_native_with_size(600, 300)
        .unwrap();
    assert!(!svg.contains("<g "), "{svg}");
}

#[test]
fn plotgrid_svg_attrs_sets_several_at_once() {
    let p = || base_plot();
    let svg = (p() | p())
        .svg_attrs(vec![
            ("data-a".to_string(), "1".to_string()),
            ("data-b".to_string(), "2".to_string()),
        ])
        .render_svg_native_with_size(600, 300)
        .unwrap();
    assert_eq!(root_attr(&svg, "data-a"), Some("1"));
    assert_eq!(root_attr(&svg, "data-b"), Some("2"));
}
