//! B2-5: `GGPlot::tooltip(template)` — `{column}` placeholders filled from
//! the built (post-stat) layer data and formatted through each column's
//! scale, replacing the geom's default `<title>`.

use ggplot_rs::locale::Locale;
use ggplot_rs::prelude::*;
use ggplot_rs::scale::format::label_currency_locale;

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
fn a_template_fills_placeholders_formatted_through_the_columns_scale() {
    let data = frame(vec![
        ("label", strs(&["Berlin"])),
        ("x", floats(&[1.0])),
        ("y", floats(&[1234.5])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_point()
        .scale_y_continuous(
            ScaleContinuous::new()
                .with_value_formatter(label_currency_locale(&Locale::de_de(), Some(0.01))),
        )
        .tooltip("{label}: {y}")
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("<title>Berlin: 1.234,50 €</title>"), "{svg}");
}

#[test]
fn a_hex_template_reads_the_stat_computed_count_column() {
    let data = frame(vec![
        ("x", floats(&[1.0, 1.0, 1.0])),
        ("y", floats(&[1.0, 1.0, 1.0])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_hex()
        .tooltip("{count} points")
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("<title>3 points</title>"), "{svg}");
}

#[test]
fn double_braces_are_literal() {
    let data = frame(vec![("x", floats(&[1.0])), ("y", floats(&[2.0]))]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .tooltip("{{x}} = {x}")
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("<title>{x} = 1</title>"), "{svg}");
}

#[test]
fn an_unknown_placeholder_renders_empty_not_literally() {
    let data = frame(vec![("x", floats(&[1.0])), ("y", floats(&[2.0]))]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .tooltip("[{nope}]")
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("<title>[]</title>"), "{svg}");
    assert!(!svg.contains("nope"), "{svg}");
}

#[test]
fn the_rendered_tooltip_is_escaped() {
    let data = frame(vec![
        ("label", strs(&["<b>&\"</b>"])),
        ("x", floats(&[1.0])),
        ("y", floats(&[2.0])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_point()
        .tooltip("{label}")
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(!svg.contains("<b>"), "{svg}");
    assert!(svg.contains("&lt;b&gt;&amp;&quot;&lt;/b&gt;"), "{svg}");
}

#[test]
fn tooltip_applies_to_the_most_recently_added_layer_only() {
    let data = frame(vec![("x", floats(&[1.0])), ("y", floats(&[2.0]))]);
    let svg = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y"))
        .geom_line()
        .geom_point()
        .tooltip("point: {y}")
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("<title>point: 2</title>"), "{svg}");
}

#[test]
fn without_a_tooltip_call_output_is_unaffected() {
    let data = frame(vec![("x", floats(&[1.0])), ("y", floats(&[2.0]))]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("<title>(1, 2)</title>"), "{svg}");
}

#[test]
fn an_unterminated_brace_is_kept_literally() {
    let data = frame(vec![("x", floats(&[1.0])), ("y", floats(&[2.0]))]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .tooltip("end {unterminated")
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("<title>end {unterminated</title>"), "{svg}");
}

#[test]
fn a_placeholder_for_an_na_value_renders_empty() {
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0])),
        ("y", floats(&[1.0, 2.0])),
        ("label", vec![Value::Na, Value::Str("b".into())]),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_point()
        .tooltip("[{label}]")
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("<title>[]</title>"), "{svg}");
    assert!(svg.contains("<title>[b]</title>"), "{svg}");
}

#[test]
fn a_template_on_a_geom_that_ignores_tooltips_warns() {
    // Review PR#141 finding 6 (LOW): tooltip() on a geom that never reads
    // the tooltip column (e.g. geom_area) silently did nothing.
    let data = frame(vec![("x", floats(&[1.0, 2.0])), ("y", floats(&[1.0, 2.0]))]);
    let (svg, warnings) = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_area()
        .tooltip("{y}")
        .render_svg_native_with_warnings(300, 200)
        .unwrap();
    assert!(!svg.contains("<title>"), "{svg}");
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("tooltip") && w.contains("no effect")),
        "{warnings:?}"
    );
}

#[test]
fn a_template_on_a_geom_that_reads_tooltips_does_not_warn() {
    let data = frame(vec![("x", floats(&[1.0])), ("y", floats(&[2.0]))]);
    let (_, warnings) = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .tooltip("{y}")
        .render_svg_native_with_warnings(300, 200)
        .unwrap();
    assert!(
        !warnings.iter().any(|w| w.contains("tooltip")),
        "{warnings:?}"
    );
}
