//! #226: an SVG *attribute* value with `\n`/`\r`/`\t` must be entity-escaped
//! (`&#10;`/`&#13;`/`&#9;`), not left as a literal control character. XML
//! attribute-value normalisation collapses an unescaped one to a plain
//! space (the SVG spec defers to XML 1.0 §3.3.3 here, and every browser
//! follows it), so a consumer reading the attribute back through any
//! XML/DOM parser silently loses the original character. Element *content*
//! (`<text>`/`<title>`) is unaffected — this issue is attribute values only.

use ggplot_rs::data::Value;
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

/// None of `\t`, `\n` or `\r` appear literally anywhere in `svg` (every
/// writer path routes through the shared escaper, so this is a safe
/// whole-document check once the fix is in).
fn has_no_literal_control_whitespace(svg: &str) -> bool {
    !svg.contains('\t') && !svg.contains('\n') && !svg.contains('\r')
}

#[test]
fn root_svg_attr_entity_escapes_tab_newline_and_cr() {
    let data = frame(vec![("x", floats(&[1.0])), ("y", floats(&[1.0]))]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .svg_attr("data-note", "a\tb\nc\rd")
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(has_no_literal_control_whitespace(&svg), "{svg}");
    assert!(svg.contains("data-note=\"a&#9;b&#10;c&#13;d\""), "{svg}");
}

#[test]
fn layer_data_attr_on_a_mark_entity_escapes_a_mapped_newline_value() {
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("note", strs(&["line one\nline two"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").data_attr("note", "note"))
        .geom_point()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(has_no_literal_control_whitespace(&svg), "{svg}");
    assert!(svg.contains("data-note=\"line one&#10;line two\""), "{svg}");
}

#[test]
fn group_wrap_entity_escapes_a_newline_in_the_group_value() {
    // #221's group_wrap puts the row's raw value straight into
    // `<g data-<name>="<value>">` — unlike `Aes::class`'s tokens, this
    // value isn't restricted to a safe character set, so a data value
    // with a literal newline is a real path here.
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("node", strs(&["a\nb"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").group_wrap("of", "node"))
        .geom_point()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(has_no_literal_control_whitespace(&svg), "{svg}");
    assert!(svg.contains("data-of=\"a&#10;b\""), "{svg}");
}

#[test]
fn fontfamily_attribute_entity_escapes_an_embedded_newline() {
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("label", strs(&["hi"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text_with(GeomText::default().with_fontfamily("weird\nfamily"))
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(has_no_literal_control_whitespace(&svg), "{svg}");
    assert!(svg.contains("font-family=\"weird&#10;family\""), "{svg}");
}

#[test]
fn text_and_title_content_are_unaffected_a_literal_newline_stays_literal() {
    // #226 is attribute values only; `escape` (element content) keeps its
    // existing behaviour — a regression guard so a future refactor can't
    // accidentally widen the attribute fix onto content too. Exercised via
    // a tooltip's `<title>` content (not `geom_text`'s own `<text>`
    // content): since #249, `geom_text` deliberately *does* treat an
    // embedded `"\n"` as a forced line break rather than writing it
    // literally into one `<text>` — see `tests/geom_text_forced_linebreak_249.rs`
    // — so it no longer exercises this invariant, but `<title>` content
    // still writes a raw column value straight through `escape()`.
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("note", strs(&["a\nb"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .tooltip("{note}")
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("a\nb"), "{svg}");
    assert!(!svg.contains("a&#10;b"), "{svg}");
}
