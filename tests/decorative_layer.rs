//! #161: `GGPlot::decorative()` — a layer whose marks carry no per-mark
//! metadata (`<title>`, `data-x`/`data-series`/`data-value`/`data-fvalue`,
//! `data_attr`), while keeping their own visual style. Meant for a layer
//! that merely annotates another layer's mark (e.g. a boxplot's own mean
//! dot) rather than drawing data in its own right.

use ggplot_rs::prelude::*;

fn floats(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&f| Value::Float(f)).collect()
}
fn strs(v: &[&str]) -> Vec<Value> {
    v.iter().map(|s| Value::Str(s.to_string())).collect()
}
fn frame(cols: Vec<(&str, Vec<Value>)>) -> Vec<(String, Vec<Value>)> {
    cols.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

#[test]
fn decorative_point_has_no_title_or_data_attrs() {
    let data = frame(vec![("x", floats(&[1.0])), ("y", floats(&[2.0]))]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .decorative()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(!svg.contains("<title>"), "{svg}");
    assert!(!svg.contains("data-x="), "{svg}");
    assert!(!svg.contains("data-series="), "{svg}");
    assert!(!svg.contains("data-value="), "{svg}");
}

#[test]
fn decorative_point_still_draws_its_own_visual_style() {
    // The wrapper mutes metadata, not drawing: the mark itself must still
    // be there, at the geom's own styling.
    let data = frame(vec![("x", floats(&[1.0])), ("y", floats(&[2.0]))]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point_with(GeomPoint {
            color: (255, 0, 0),
            size: 9.0,
            ..Default::default()
        })
        .decorative()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("<circle"), "{svg}");
    assert!(svg.contains("#FF0000"), "{svg}");
}

#[test]
fn only_the_decorative_layer_loses_its_metadata() {
    // A second, ordinary point layer in the same plot keeps its own
    // `<title>`/`data-*` — the mute wrapper must not leak past its own
    // layer's draw call.
    let data = frame(vec![("x", floats(&[1.0])), ("y", floats(&[2.0]))]);
    let decorative_only = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .decorative()
        .geom_point()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert_eq!(
        decorative_only.matches("<title>").count(),
        1,
        "{decorative_only}"
    );

    let both_plain = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_point()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert_eq!(both_plain.matches("<title>").count(), 2, "{both_plain}");
}

#[test]
fn decorative_data_attr_is_suppressed() {
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[2.0])),
        ("tag", strs(&["mean"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .layer_aes(Aes::new().x("x").y("y").data_attr("tag", "tag"))
        .decorative()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(!svg.contains("data-tag"), "{svg}");
}

#[test]
fn tooltip_on_a_decorative_layer_has_no_effect_and_warns() {
    let data = frame(vec![("x", floats(&[1.0])), ("y", floats(&[2.0]))]);
    let (svg, warnings) = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point() // GeomPoint::consumes_tooltips() is normally true
        .decorative()
        .tooltip("{y}")
        .render_svg_native_with_warnings(300, 200)
        .unwrap();
    assert!(!svg.contains("<title>"), "{svg}");
    assert!(
        warnings.iter().any(|w| w.contains("tooltip")),
        "{warnings:?}"
    );
}

/// Whether a legend was drawn: the legend's own key labels are the only
/// `<text>` elements with `text-anchor="start"` (axis labels use `"middle"`/
/// `"end"`) — a marker independent of any per-mark metadata, which the
/// decorative wrapper deliberately changes.
fn has_legend(svg: &str) -> bool {
    svg.contains("text-anchor=\"start\"")
}

#[test]
fn decorative_defaults_out_of_the_legend() {
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0])),
        ("y", floats(&[1.0, 2.0])),
        ("g", strs(&["a", "b"])),
    ]);
    let plot = |decorative: bool, show_legend: Option<bool>| {
        let mut p = GGPlot::new(data.clone())
            .aes(Aes::new().x("x").y("y").color("g"))
            .geom_point();
        if decorative {
            p = p.decorative();
        }
        if let Some(show) = show_legend {
            p = p.show_legend(show);
        }
        p.render_svg_native_with_size(400, 300).unwrap()
    };

    assert!(
        !has_legend(&plot(true, None)),
        "decorative() should hide the legend by default"
    );
    assert!(
        !has_legend(&plot(false, Some(false))),
        "show_legend(false) should hide the legend, for comparison"
    );
    assert!(
        has_legend(&plot(false, None)),
        "a non-decorative mapped colour normally gets a legend"
    );
    assert!(
        has_legend(&plot(true, Some(true))),
        "show_legend(true) after decorative() must still show the legend"
    );
}

#[test]
fn decorative_required_aes_and_name_are_delegated() {
    // Sanity: the wrapper doesn't break ordinary build-time plumbing that
    // reads `Geom::required_aes`/`Geom::name` (validation, warnings).
    let data = frame(vec![("x", floats(&[1.0]))]); // y missing
    let result = GGPlot::new(data)
        .aes(Aes::new().x("x"))
        .geom_point()
        .decorative()
        .render_svg_native_with_size(300, 200);
    assert!(result.is_err(), "{result:?}");
}
