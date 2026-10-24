//! #228: `geom_text`/`geom_label` fontface (bold/italic), fixed via
//! `.with_fontface(..)` and per row via `Aes::fontface`. Same `FontFace`
//! enum and SVG `font-weight`/`font-style` attributes `GeomTextRepel`
//! already uses (`src/render/svg_backend.rs`'s `draw_text_with_title`/
//! `draw_text_runs`) — this just wires `GeomText`/`GeomLabel` up to it.

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
fn default_fontface_is_plain_and_byte_identical() {
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("label", strs(&["hi"])),
    ]);
    let without_fontface = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    let with_explicit_plain = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text_with(GeomText {
            fontface: FontFace::Plain,
            ..Default::default()
        })
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert_eq!(without_fontface, with_explicit_plain);
    assert!(!without_fontface.contains("font-weight"));
    assert!(!without_fontface.contains("font-style"));
}

#[test]
fn with_fontface_bold_sets_font_weight_on_geom_text_and_geom_label() {
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("label", strs(&["hi"])),
    ]);
    let text_svg = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text_with(GeomText {
            fontface: FontFace::Bold,
            ..Default::default()
        })
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(text_svg.contains("font-weight=\"bold\""), "{text_svg}");
    assert!(!text_svg.contains("font-style"), "{text_svg}");

    let label_svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_label_with(GeomLabel {
            fontface: FontFace::Bold,
            ..Default::default()
        })
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(label_svg.contains("font-weight=\"bold\""), "{label_svg}");
}

#[test]
fn with_fontface_italic_sets_font_style_on_geom_text_and_geom_label() {
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("label", strs(&["hi"])),
    ]);
    let text_svg = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text_with(GeomText {
            fontface: FontFace::Italic,
            ..Default::default()
        })
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(text_svg.contains("font-style=\"italic\""), "{text_svg}");
    assert!(!text_svg.contains("font-weight"), "{text_svg}");

    let label_svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_label_with(GeomLabel {
            fontface: FontFace::Italic,
            ..Default::default()
        })
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(label_svg.contains("font-style=\"italic\""), "{label_svg}");
}

#[test]
fn with_fontface_builder_method_sets_the_fixed_face() {
    // The tests above build GeomText/GeomLabel via a struct literal
    // (`fontface: FontFace::X, ..Default::default()`); this exercises the
    // `.with_fontface(..)` builder itself, the intended public entry
    // point for a fixed (non-per-row) face.
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("label", strs(&["hi"])),
    ]);
    let text_svg = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text_with(GeomText::default().with_fontface(FontFace::Bold))
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(text_svg.contains("font-weight=\"bold\""), "{text_svg}");

    let label_svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_label_with(GeomLabel::default().with_fontface(FontFace::Italic))
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(label_svg.contains("font-style=\"italic\""), "{label_svg}");
}

#[test]
fn mapped_fontface_wins_per_row_over_the_fixed_setting() {
    // Fixed setting is Italic; the per-row mapping overrides each row to
    // its own value — "bold" on row 0, "plain" on row 1 (no italic left
    // anywhere), an unparseable value on row 2 falls back to the fixed
    // Italic.
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0, 3.0])),
        ("y", floats(&[1.0, 2.0, 3.0])),
        ("label", strs(&["a", "b", "c"])),
        ("face", strs(&["bold", "plain", "not-a-face"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label").fontface("face"))
        .geom_text_with(GeomText {
            fontface: FontFace::Italic,
            ..Default::default()
        })
        .render_svg_native_with_size(300, 200)
        .unwrap();

    assert_eq!(svg.matches("font-weight=\"bold\"").count(), 1, "{svg}");
    assert_eq!(svg.matches("font-style=\"italic\"").count(), 1, "{svg}");
}

#[test]
fn mapped_fontface_also_wins_on_geom_label() {
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0])),
        ("y", floats(&[1.0, 2.0])),
        ("label", strs(&["a", "b"])),
        ("face", strs(&["italic", "bold"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label").fontface("face"))
        .geom_label()
        .render_svg_native_with_size(300, 200)
        .unwrap();

    assert_eq!(svg.matches("font-style=\"italic\"").count(), 1, "{svg}");
    assert_eq!(svg.matches("font-weight=\"bold\"").count(), 1, "{svg}");
}
