//! Host-facing SVG metadata: per-mark `data-x` / `data-series` / `data-value`,
//! root `data-domain` family, and positioned fragments for composition.

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::position::stack::PositionStack;
use ggplot_rs::GGPlot;

fn floats(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&f| Value::Float(f)).collect()
}
fn strs(v: &[&str]) -> Vec<Value> {
    v.iter().map(|s| Value::Str(s.to_string())).collect()
}
fn frame(cols: Vec<(&str, Vec<Value>)>) -> Vec<(String, Vec<Value>)> {
    cols.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

/// The value of attribute `name` on the root `<svg …>` tag.
fn root_attr<'a>(svg: &'a str, name: &str) -> Option<&'a str> {
    let head = &svg[..svg.find('>').unwrap()];
    let key = format!(" {name}=\"");
    let i = head.find(&key)? + key.len();
    Some(&head[i..i + head[i..].find('"').unwrap()])
}

#[test]
fn points_carry_series_and_raw_value() {
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0, 3.0])),
        ("y", floats(&[10.5, 20.123456789, 30.0])),
        ("g", strs(&["web", "app", "web"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert!(svg.contains(r#"data-x="2" data-series="app" data-value="20.123456789""#));
    assert!(svg.contains(r#"data-series="web" data-value="10.5""#));
}

#[test]
fn stacked_bars_report_segment_value_not_stack_top() {
    let data = frame(vec![
        ("x", strs(&["mon", "mon"])),
        ("y", floats(&[22.0, 8.0])),
        ("g", strs(&["web", "app"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").fill("g").label("g"))
        .geom_col()
        .position(PositionStack)
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert!(svg.contains(r#"data-x="mon" data-series="web" data-value="22""#));
    assert!(svg.contains(r#"data-series="app" data-value="8""#));
    // The tooltip reports the segment value, not the cumulative top (30).
    assert!(svg.contains("<title>app: 8</title>"), "{svg}");
}

#[test]
fn series_and_value_attributes_are_escaped() {
    let data = frame(vec![
        ("x", strs(&["a\"><script>"])),
        ("y", floats(&[1.0])),
        ("g", strs(&["x' onmouseover='alert(1)"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").fill("g"))
        .geom_col()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(!svg.contains("<script>"));
    assert!(!svg.contains("' onmouseover"));
    assert!(svg.contains("data-series=\"x&#39; onmouseover=&#39;alert(1)\""));
}

#[test]
fn root_carries_expanded_domain() {
    let data = frame(vec![
        ("x", floats(&[0.0, 10.0])),
        ("y", floats(&[100.0, 200.0])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .render_svg_native_with_size(400, 300)
        .unwrap();
    // Default expansion is 5% each side.
    assert_eq!(root_attr(&svg, "data-domain"), Some("-0.5 10.5 95 205"));
    assert_eq!(root_attr(&svg, "data-xdomain"), Some("-0.5 10.5"));
    assert_eq!(root_attr(&svg, "data-ydomain"), Some("95 205"));
    assert!(root_attr(&svg, "data-plot").is_some());
}

#[test]
fn discrete_axis_emits_levels_instead_of_domain() {
    let data = frame(vec![
        ("x", strs(&["b\"q", "a"])),
        ("y", floats(&[1.0, 3.0])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_col()
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert_eq!(root_attr(&svg, "data-domain"), None);
    assert_eq!(root_attr(&svg, "data-xdomain"), None);
    assert_eq!(
        root_attr(&svg, "data-xlevels"),
        Some(r#"[&quot;b\&quot;q&quot;,&quot;a&quot;]"#)
    );
    assert!(root_attr(&svg, "data-ydomain").is_some());
}

#[test]
fn fragment_is_positioned_without_xmlns() {
    let data = frame(vec![("x", floats(&[1.0, 2.0])), ("y", floats(&[1.0, 2.0]))]);
    let frag = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .render_svg_native_at(12.5, 40.0, 320, 200)
        .unwrap();
    assert!(frag
        .starts_with(r#"<svg x="12.50" y="40.00" width="320" height="200" viewBox="0 0 320 200""#));
    assert!(!frag.contains("xmlns"));
    assert!(frag.ends_with("</svg>"));

    // Composes into a parent document as-is.
    let page = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"640\" height=\"240\">{frag}</svg>"
    );
    assert_eq!(page.matches("xmlns=").count(), 1);
}

#[test]
fn faceted_marks_keep_hover_metadata() {
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0, 1.0, 2.0])),
        ("y", floats(&[1.0, 2.0, 3.0, 4.0])),
        ("f", strs(&["A", "A", "B", "B"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .facet_wrap("f", None)
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert!(svg.contains(r#"data-value="4""#));
    assert!(svg.contains("<title>"));
}
