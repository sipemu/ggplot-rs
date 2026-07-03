//! Issue #47 — element_text face (bold / italic).
use ggplot_rs::prelude::*;

fn title_attrs(face: FontFace) -> String {
    let d = vec![
        ("x".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("y".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
    ];
    let mut t = theme_minimal();
    t.title.face = face;
    let svg = GGPlot::new(d)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .theme(t)
        .title("FACEMARK")
        .render_svg()
        .unwrap();
    let i = svg.find("FACEMARK").unwrap();
    svg[svg[..i].rfind("<text").unwrap()..i].to_string()
}

#[test]
fn bold_title_emits_font_weight_bold() {
    assert!(title_attrs(FontFace::Bold).contains("font-weight=\"bold\""));
}

#[test]
fn italic_title_emits_italic_style() {
    let a = title_attrs(FontFace::Italic);
    assert!(
        a.contains("font-style=\"italic\"") || a.contains("italic"),
        "attrs: {a}"
    );
}

#[test]
fn plain_title_is_not_bold() {
    assert!(!title_attrs(FontFace::Plain).contains("font-weight=\"bold\""));
}
