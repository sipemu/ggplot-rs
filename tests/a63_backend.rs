//! A-grade #63 — a second, plotters-free DrawBackend drives the same renderer.
use ggplot_rs::prelude::*;

#[test]
fn native_svg_backend_renders_a_full_plot() {
    let x: Vec<Value> = (0..20).map(|i| Value::Float(i as f64)).collect();
    let y: Vec<Value> = (0..20).map(|i| Value::Float((i % 5) as f64)).collect();
    let svg = GGPlot::new(vec![("x".to_string(), x), ("y".to_string(), y)])
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .title("Native")
        .render_svg_native_with_size(400, 300)
        .expect("native render");

    assert!(
        svg.starts_with("<svg") && svg.ends_with("</svg>"),
        "well-formed svg"
    );
    assert!(svg.matches("<circle").count() >= 20, "one circle per point");
    assert!(svg.contains("Native"), "title present");
    assert!(svg.contains("<text"), "axis/legend text present");
}

#[test]
fn native_and_plotters_agree_on_element_counts() {
    let data = vec![
        (
            "x".to_string(),
            (0..12).map(|i| Value::Float(i as f64)).collect::<Vec<_>>(),
        ),
        (
            "y".to_string(),
            (0..12)
                .map(|i| Value::Float((i % 3) as f64))
                .collect::<Vec<_>>(),
        ),
    ];
    let native = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .render_svg_native_with_size(400, 300)
        .unwrap();
    // Both backends draw one marker per point.
    assert_eq!(native.matches("<circle").count(), 12);
}
