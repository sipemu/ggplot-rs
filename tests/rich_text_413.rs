//! #413: rich text (ggtext's `element_markdown()`) behind `Theme::rich_text`.

use ggplot_rs::prelude::*;

fn plot(title: &str) -> GGPlot {
    GGPlot::new(vec![
        ("x".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("y".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
    ])
    .aes(Aes::new().x("x").y("y"))
    .geom_point()
    .labs(Labels {
        title: Some(title.into()),
        y: Some("CO~2~ (ppm)".into()),
        ..Default::default()
    })
}

const TITLE: &str = "**Bold** and *italic*, x^2^ <span style='color:#ff0000'>red</span>";

#[test]
fn off_by_default_text_is_literal() {
    let svg = plot(TITLE).render_svg_native_with_size(400, 300).unwrap();
    assert!(svg.contains("**Bold**"));
    assert!(!svg.contains("<tspan font-weight"));
}

#[test]
fn markdown_becomes_styled_tspans() {
    let svg = plot(TITLE)
        .theme_with(|t| t.rich_text = true)
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert!(
        svg.contains(r#"<tspan font-weight="bold">Bold</tspan>"#),
        "{svg}"
    );
    assert!(svg.contains(r#"<tspan font-style="italic">italic</tspan>"#));
    assert!(svg.contains(r#"baseline-shift="super""#));
    assert!(svg.contains(r##"<tspan fill="#FF0000">red</tspan>"##));
    // The y axis title's subscript too.
    assert!(svg.contains(r#"baseline-shift="sub""#));
    assert!(!svg.contains("**"));
}

#[test]
fn labels_without_markup_are_unchanged() {
    let a = plot("Plain title")
        .render_svg_native_with_size(400, 300)
        .unwrap();
    let b = plot("Plain title")
        .theme_with(|t| t.rich_text = true)
        .render_svg_native_with_size(400, 300)
        .unwrap();
    // Only the y title (which has markup) differs.
    assert!(a.contains(">Plain title</text>") && b.contains(">Plain title</text>"));
}
