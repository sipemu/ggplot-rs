//! B2-6: geom text inherits the theme font. Resolution order: the geom's
//! own `fontfamily`, then `theme.text.family`, else `"sans-serif"`.
//! Applies to text, label, repel, bracket and annotations.

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

fn theme_with_family(family: &str) -> Theme {
    Theme::default().set_text(ElementText {
        family: family.to_string(),
        ..Default::default()
    })
}

#[test]
fn default_output_is_byte_identical_with_no_family_set_anywhere() {
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("label", strs(&["hi"])),
    ]);
    let without_theme_call = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    let with_default_theme = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text()
        .theme(Theme::default())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert_eq!(without_theme_call, with_default_theme);
    assert!(without_theme_call.contains("font-family=\"sans-serif\""));
}

#[test]
fn theme_text_family_changes_geom_text_and_geom_label() {
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("label", strs(&["hi"])),
    ]);
    for svg in [
        GGPlot::new(data.clone())
            .aes(Aes::new().x("x").y("y").label("label"))
            .geom_text()
            .theme(theme_with_family("Georgia"))
            .render_svg_native_with_size(300, 200)
            .unwrap(),
        GGPlot::new(data)
            .aes(Aes::new().x("x").y("y").label("label"))
            .geom_label()
            .theme(theme_with_family("Georgia"))
            .render_svg_native_with_size(300, 200)
            .unwrap(),
    ] {
        assert!(svg.contains("font-family=\"Georgia\""), "{svg}");
        assert!(!svg.contains("sans-serif"), "{svg}");
    }
}

#[test]
fn an_explicit_geom_fontfamily_wins_over_the_theme() {
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("label", strs(&["hi"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text_with(GeomText {
            fontfamily: "Courier".to_string(),
            ..Default::default()
        })
        .theme(theme_with_family("Georgia"))
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("font-family=\"Courier\""), "{svg}");
}

#[test]
fn theme_text_family_changes_text_repel_label_repel_and_bracket() {
    let repel_data = frame(vec![
        ("x", floats(&[1.0, 2.0, 1.1, 2.1])),
        ("y", floats(&[1.0, 2.0, 1.1, 2.1])),
        ("label", strs(&["a", "b", "c", "d"])),
    ]);
    let text_repel_svg = GGPlot::new(repel_data.clone())
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text_repel()
        .theme(theme_with_family("Georgia"))
        .render_svg_native_with_size(300, 300)
        .unwrap();
    assert!(
        text_repel_svg.contains("font-family=\"Georgia\""),
        "{text_repel_svg}"
    );

    let label_repel_svg = GGPlot::new(repel_data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_label_repel()
        .theme(theme_with_family("Georgia"))
        .render_svg_native_with_size(300, 300)
        .unwrap();
    assert!(
        label_repel_svg.contains("font-family=\"Georgia\""),
        "{label_repel_svg}"
    );

    let bracket_svg = GGPlot::new(frame(vec![
        ("x", strs(&["a", "b"])),
        ("y", floats(&[1.0, 2.0])),
    ]))
    .aes(Aes::new().x("x").y("y"))
    .geom_col()
    .geom_bracket("a", "b", 2.5, "p < 0.05")
    .theme(theme_with_family("Georgia"))
    .render_svg_native_with_size(300, 200)
    .unwrap();
    assert!(
        bracket_svg.contains("font-family=\"Georgia\""),
        "{bracket_svg}"
    );
}

#[test]
fn theme_text_family_changes_text_annotations() {
    let data = frame(vec![("x", floats(&[1.0])), ("y", floats(&[1.0]))]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .annotate_text("note", 1.0, 1.0)
        .theme(theme_with_family("Georgia"))
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("font-family=\"Georgia\""), "{svg}");
}

#[test]
fn theme_geom_family_sits_between_the_geom_and_theme_text_family() {
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("label", strs(&["hi"])),
    ]);
    // theme.geom.family wins over theme.text.family when both are set.
    let svg = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text()
        .theme(theme_with_family("Verdana").set_geom(ElementGeom::default().with_family("Courier")))
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("font-family=\"Courier\""), "{svg}");

    // theme.geom.family alone (no theme.text.family override) still applies.
    let svg2 = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text()
        .theme(Theme::default().set_geom(ElementGeom::default().with_family("Courier")))
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg2.contains("font-family=\"Courier\""), "{svg2}");
}
