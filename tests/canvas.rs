//! Raster (canvas) backend — feature `canvas`.
#![cfg(feature = "canvas")]
use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;
use ggplot_rs::render::backend::{DrawBackend, RectStyle};
use ggplot_rs::render::pixel_backend::PixelBackend;
use ggplot_rs::render::Rect;

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

#[test]
fn raster_area_panel_within_canvas() {
    let (rgba, area) = GGPlot::new(scatter())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .render_rgba_area_with_size(300, 200)
        .unwrap();
    assert_eq!(rgba.len(), 300 * 200 * 4);
    let [px, py, pw, ph] = area;
    assert!(pw > 0.0 && ph > 0.0, "panel has size");
    assert!(
        px >= 0.0 && py >= 0.0 && px + pw <= 300.0 && py + ph <= 200.0,
        "panel within canvas: {area:?}"
    );
}

/// #176: `PixelBackend::draw_rect`'s `corner_radius > 0.0` path (no native
/// rounded-rect primitive here — it approximates one as a polygon via
/// `render::backend::rounded_rect_points`, same as `plotters_backend`).
/// Drawn directly against the backend (not through a full `GGPlot`), so the
/// pixel coordinates are exact: a 100x100 opaque-white canvas, a
/// 20..=80 x 20..=80 box with a 15px radius, filled and stroked.
#[test]
fn draw_rect_with_corner_radius_rounds_the_corners_not_the_edge_midpoints() {
    let plot_area = Rect {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    };
    let mut backend = PixelBackend::new(100, 100, plot_area);
    backend
        .draw_rect(
            (20.0, 20.0),
            (80.0, 80.0),
            &RectStyle {
                fill: Some((0, 0, 255)),
                stroke: Some((255, 0, 0)),
                stroke_width: 2.0,
                alpha: 1.0,
                clip: false,
                corner_radius: 15.0,
            },
        )
        .unwrap();
    let rgba = backend.into_rgba();
    let px = |x: usize, y: usize| -> (u8, u8, u8) {
        let i = (y * 100 + x) * 4;
        (rgba[i], rgba[i + 1], rgba[i + 2])
    };
    // A corner of the bounding box, well outside the 15px radius sweep:
    // still background (white) — the box is rounded, not a plain square.
    let corner = px(21, 21);
    assert_eq!(
        corner,
        (255, 255, 255),
        "corner should be untouched background, got {corner:?}"
    );
    // The centre of a straight edge (no corner curvature there): filled.
    let edge_mid = px(50, 21);
    assert_ne!(
        edge_mid,
        (255, 255, 255),
        "edge midpoint should be drawn (fill or stroke)"
    );
    // The box centre, away from any edge: the fill colour (blue).
    let (r, g, b) = px(50, 50);
    assert!(
        b > r && b > g,
        "centre should be the blue fill, got ({r},{g},{b})"
    );
}

/// The same path with `corner_radius` clamped to the plain-rect fallback
/// (`<= 0.0`): draws a plain square — its corner IS filled, unlike the
/// rounded case above — confirming the two code paths actually differ.
#[test]
fn draw_rect_with_zero_corner_radius_fills_the_corner_like_a_plain_rect() {
    let plot_area = Rect {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 100.0,
    };
    let mut backend = PixelBackend::new(100, 100, plot_area);
    backend
        .draw_rect(
            (20.0, 20.0),
            (80.0, 80.0),
            &RectStyle {
                fill: Some((0, 0, 255)),
                stroke: None,
                stroke_width: 0.0,
                alpha: 1.0,
                clip: false,
                corner_radius: 0.0,
            },
        )
        .unwrap();
    let rgba = backend.into_rgba();
    let i = (21 * 100 + 21) * 4;
    let corner = (rgba[i], rgba[i + 1], rgba[i + 2]);
    assert_ne!(
        corner,
        (255, 255, 255),
        "plain rect's corner should be filled"
    );
}
