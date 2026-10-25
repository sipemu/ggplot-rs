//! sipemu/ggplot-rs#255: `GGPlot::svg_title`/`svg_desc` — an accessible
//! name/description for the rendered root `<svg>`, written as `<title>`/
//! `<desc>` first children plus `role="img"` and
//! `aria-labelledby`/`aria-describedby`. Off by default, so output stays
//! byte-identical when neither is called.

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

fn base_plot() -> GGPlot {
    GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
}

/// The value of attribute `name` on the root `<svg …>` tag.
fn root_attr<'a>(svg: &'a str, name: &str) -> Option<&'a str> {
    let head = &svg[..svg.find('>').unwrap()];
    let key = format!(" {name}=\"");
    let i = head.find(&key)? + key.len();
    Some(&head[i..i + head[i..].find('"').unwrap()])
}

#[test]
fn without_title_or_desc_output_is_unaffected() {
    // Points carry their own per-mark `<title>` hover tooltip by default
    // (bare `<title>text</title>`, no `id`) — unrelated to this feature.
    // Only the root-level `<title id="...">` and `<desc>` must be absent.
    let svg = base_plot().render_svg_native_with_size(300, 200).unwrap();
    assert!(!svg.contains("<title id="));
    assert!(!svg.contains("<desc"));
    assert!(!svg.contains("role=\"img\""));
    assert!(!svg.contains("aria-labelledby"));
    assert!(!svg.contains("aria-describedby"));
}

#[test]
fn svg_title_adds_a_title_child_role_and_aria_labelledby() {
    let svg = base_plot()
        .svg_title("Sales by region")
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert_eq!(root_attr(&svg, "role"), Some("img"));
    let id = root_attr(&svg, "aria-labelledby").expect("aria-labelledby present");
    assert!(
        svg.contains(&format!("<title id=\"{id}\">Sales by region</title>")),
        "{svg}"
    );
    assert_eq!(root_attr(&svg, "aria-describedby"), None);
    assert!(!svg.contains("<desc"));
}

#[test]
fn svg_desc_adds_a_desc_child_role_and_aria_describedby() {
    let svg = base_plot()
        .svg_desc("4 regions; highest Bavaria (5.2); 1 without data")
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert_eq!(root_attr(&svg, "role"), Some("img"));
    let id = root_attr(&svg, "aria-describedby").expect("aria-describedby present");
    assert!(
        svg.contains(&format!(
            "<desc id=\"{id}\">4 regions; highest Bavaria (5.2); 1 without data</desc>"
        )),
        "{svg}"
    );
    assert_eq!(root_attr(&svg, "aria-labelledby"), None);
    assert!(!svg.contains("<title id="));
}

#[test]
fn both_set_title_comes_before_desc_as_the_svgs_first_children() {
    let svg = base_plot()
        .svg_title("Sales by region")
        .svg_desc("4 regions; highest Bavaria (5.2); 1 without data")
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert_eq!(root_attr(&svg, "role"), Some("img"));
    let title_i = svg.find("<title").expect("title present");
    let desc_i = svg.find("<desc").expect("desc present");
    assert!(title_i < desc_i, "{svg}");

    // Both are the very first content after the root `<svg ...>` opening
    // tag's `>` — nothing else (defs, marks) precedes them.
    let root_close = svg.find('>').unwrap();
    assert_eq!(&svg[root_close + 1..root_close + 1 + 7], "<title ", "{svg}");
}

#[test]
fn title_and_desc_text_is_escaped() {
    let svg = base_plot()
        .svg_title("<script>\"&'")
        .svg_desc("<script>\"&'")
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(!svg.contains("<script>"));
    // Two occurrences: one in <title>, one in <desc>.
    assert_eq!(svg.matches("&lt;script&gt;").count(), 2);
}

#[test]
fn a_user_role_attr_collides_with_the_automatic_one() {
    let (svg, warnings) = base_plot()
        .svg_attr("role", "forged")
        .svg_title("Sales by region")
        .render_svg_native_with_warnings(300, 200)
        .unwrap();
    assert_ne!(root_attr(&svg, "role"), Some("forged"));
    assert_eq!(root_attr(&svg, "role"), Some("img"));
    assert!(warnings.iter().any(|w| w.contains("role")));
}

#[test]
fn a_user_aria_labelledby_attr_collides_with_the_automatic_one() {
    let (svg, warnings) = base_plot()
        .svg_attr("aria-labelledby", "forged")
        .svg_title("Sales by region")
        .render_svg_native_with_warnings(300, 200)
        .unwrap();
    assert_ne!(root_attr(&svg, "aria-labelledby"), Some("forged"));
    assert!(warnings.iter().any(|w| w.contains("aria-labelledby")));
}
