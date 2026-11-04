//! #465: `svg_semantic_classes` tags the SVG's non-data elements with a
//! stable class vocabulary; off by default (output unchanged).

use ggplot_rs::prelude::*;

fn plot() -> GGPlot {
    let s = |v: &[&str]| v.iter().map(|s| Value::Str(s.to_string())).collect();
    GGPlot::new(vec![
        (
            "x".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "y".to_string(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        ("g".to_string(), s(&["a", "b", "a"])),
    ])
    .aes(Aes::new().x("x").y("y").color("g"))
    .geom_point()
    .guide_title(Aesthetic::Color, "Grp")
    .labs(Labels {
        title: Some("T".into()),
        subtitle: Some("S".into()),
        caption: Some("C".into()),
        tag: Some("A".into()),
        x: Some("X".into()),
        y: Some("Y".into()),
    })
}

/// The class of the element whose text content is `label`.
fn class_of(svg: &str, label: &str) -> String {
    let end = svg.find(&format!(">{label}</text>")).expect(label);
    let tag = &svg[svg[..end].rfind("<text").unwrap()..end];
    let i = tag.find(" class=\"").expect("class") + 8;
    tag[i..].split('"').next().unwrap().to_string()
}

#[test]
fn off_by_default() {
    let svg = plot().render_svg_native_with_size(500, 300).unwrap();
    assert!(!svg.contains("gg-axis"));
    assert!(!svg.contains("gg-legend"));
}

#[test]
fn every_kind_of_element_is_tagged() {
    let svg = plot()
        .svg_semantic_classes(true)
        .render_svg_native_with_size(500, 300)
        .unwrap();
    for class in [
        "gg-plot-background",
        "gg-panel-background",
        "gg-grid gg-grid-major gg-grid-x",
        "gg-grid gg-grid-minor gg-grid-y",
        "gg-axis gg-axis-x gg-axis-tick",
        "gg-axis gg-axis-y gg-axis-text",
        "gg-legend gg-legend-key",
    ] {
        assert!(svg.contains(&format!("class=\"{class}\"")), "{class}");
    }
    assert_eq!(class_of(&svg, "T"), "gg-title");
    assert_eq!(class_of(&svg, "S"), "gg-subtitle");
    assert_eq!(class_of(&svg, "C"), "gg-caption");
    assert_eq!(class_of(&svg, "A"), "gg-tag");
    assert_eq!(class_of(&svg, "X"), "gg-axis-title gg-axis-title-x");
    assert_eq!(class_of(&svg, "Y"), "gg-axis-title gg-axis-title-y");
    assert_eq!(class_of(&svg, "Grp"), "gg-legend gg-legend-title");
    assert_eq!(class_of(&svg, "a"), "gg-legend gg-legend-text");
    // Data marks keep their own classes.
    assert!(svg.contains("class=\"gg-mark gg-point\""));
}

#[test]
fn facet_strips_are_tagged() {
    let svg = plot()
        .facet_wrap("g", None)
        .svg_semantic_classes(true)
        .render_svg_native_with_size(500, 300)
        .unwrap();
    assert!(svg.contains("class=\"gg-strip gg-strip-background\""));
    assert!(svg.contains("class=\"gg-strip gg-strip-text\""));
}
