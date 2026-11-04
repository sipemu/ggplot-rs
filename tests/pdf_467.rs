//! #467: vector PDF export (feature `pdf`).
#![cfg(feature = "pdf")]

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
            vec![Value::Float(2.0), Value::Float(1.0), Value::Float(3.0)],
        ),
        ("g".to_string(), s(&["a", "b", "a"])),
    ])
    .aes(Aes::new().x("x").y("y").color("g").shape("g"))
    .geom_point()
    .geom_line()
    .labs(Labels {
        title: Some("Hello PDF".into()),
        ..Default::default()
    })
}

#[test]
fn renders_a_one_page_pdf_with_embedded_fonts() {
    let pdf = plot().render_pdf_with_size(400, 300).unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.trim_end().ends_with("%%EOF"));
    assert!(text.contains("/Count 1"), "one page");
    assert!(text.contains("/FontFile2"), "fonts are embedded");
    assert!(text.contains("MediaBox"));
}

#[test]
fn facets_clipping_and_rotated_text_render() {
    let pdf = plot()
        .facet_wrap("g", None)
        .coord_cartesian_zoom(Some((1.5, 2.5)), None)
        .axis_text_x_angle(45.0)
        .render_pdf_with_size(500, 300)
        .unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
}

#[cfg(feature = "plotters")]
#[test]
fn save_dispatches_on_the_pdf_extension() {
    let dir = std::env::temp_dir().join(format!("ggplot-rs-pdf-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("plot.pdf");
    plot()
        .save_with_size(path.to_str().unwrap(), 300, 200)
        .unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
    std::fs::remove_dir_all(&dir).ok();
}
