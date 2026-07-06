//! Raster (canvas) backend — feature `canvas`.
#![cfg(feature = "canvas")]
use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;

fn scatter() -> Vec<(String, Vec<Value>)> {
    let n = 2000;
    let x = (0..n)
        .map(|i| Value::Float((i as f64 * 0.01).sin() * 3.0))
        .collect();
    let y = (0..n)
        .map(|i| Value::Float((i as f64 * 0.013).cos() * 3.0))
        .collect();
    vec![("x".to_string(), x), ("y".to_string(), y)]
}

#[test]
fn raster_rgba_is_drawn() {
    let (w, h, rgba) = GGPlot::new(scatter())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .title("raster")
        .render_rgba_with_size(240, 180)
        .unwrap();
    assert_eq!((w, h), (240, 180));
    assert_eq!(rgba.len(), 240 * 180 * 4);
    // Something non-white was drawn (points/axes/text).
    assert!(rgba
        .chunks(4)
        .any(|p| p[0] < 250 || p[1] < 250 || p[2] < 250));
    // Fully opaque.
    assert!(rgba.chunks(4).all(|p| p[3] == 255));
}

#[test]
fn raster_png_has_magic() {
    let png = GGPlot::new(scatter())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .render_png_raster_with_size(200, 150)
        .unwrap();
    assert_eq!(&png[..4], &[0x89, 0x50, 0x4E, 0x47]); // ‰PNG
}
