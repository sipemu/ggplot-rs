//! A-grade #59 — serif/monospace families render real glyphs (not just the
//! SVG attribute): the rasterized PNG differs from sans-serif.
use ggplot_rs::prelude::*;

fn png_for_family(family: &str) -> Vec<u8> {
    let d = vec![
        ("x".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("y".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
    ];
    let mut t = theme_minimal();
    t.title.family = family.to_string();
    t.title.size = 30.0;
    GGPlot::new(d)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .theme(t)
        .title("FontProbe Ag")
        .render_png_with_size(300, 120)
        .unwrap()
}

#[test]
fn serif_and_mono_differ_from_sans() {
    let sans = png_for_family("sans-serif");
    assert_ne!(
        png_for_family("serif"),
        sans,
        "serif should render different glyphs"
    );
    assert_ne!(
        png_for_family("monospace"),
        sans,
        "monospace should render different glyphs"
    );
}
