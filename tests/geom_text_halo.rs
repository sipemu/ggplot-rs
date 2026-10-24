//! #229: `geom_text` text halo (`bg_colour`/`bg_r`, ggrepel's naming). SVG:
//! `stroke`/`stroke-width`/`stroke-linejoin="round"`/`paint-order="stroke"`
//! on the `<text>`, so the stroke paints under the fill. Default is off
//! (`bg_colour: None`), so existing output stays byte-identical.

use ggplot_rs::prelude::*;

fn floats(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&f| Value::Float(f)).collect()
}
fn strs(v: &[&str]) -> Vec<Value> {
    v.iter().map(|s| Value::Str(s.to_string())).collect()
}
fn frame(cols: Vec<(&str, Vec<Value>)>) -> Vec<(String, Vec<Value>)> {
    cols.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

#[test]
fn default_has_no_halo_and_is_byte_identical() {
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("label", strs(&["hi"])),
    ]);
    let plain = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    let explicit_off = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text_with(GeomText {
            bg_colour: None,
            bg_r: 0.0,
            ..Default::default()
        })
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert_eq!(plain, explicit_off);
    assert!(!plain.contains("paint-order"));
    assert!(!label_text_element(&plain).contains("stroke"), "{plain}");
}

/// The `<text ... data-geom="text">...</text>` element `geom_text` itself
/// drew (axis tick labels share `<text>` too, but not that `data-geom`).
fn label_text_element(svg: &str) -> &str {
    let marker = svg.find("data-geom=\"text\"").expect("a geom_text mark");
    let start = svg[..marker].rfind("<text").expect("its <text> start");
    let end = svg[start..].find("</text>").unwrap() + start + "</text>".len();
    &svg[start..end]
}

#[test]
fn bg_colour_and_bg_r_draw_a_stroke_halo_under_the_fill() {
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("label", strs(&["hi"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text_with(GeomText::default().bg_colour((255, 255, 255)).bg_r(2.0))
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");
    assert!(svg.contains("stroke-width=\"4.00\""), "{svg}");
    assert!(svg.contains("stroke-linejoin=\"round\""), "{svg}");
    assert!(svg.contains("paint-order=\"stroke\""), "{svg}");
}

#[test]
fn bg_colour_without_bg_r_draws_no_halo() {
    // ggrepel's own bg.r has a default, but ggplot-rs's bg_r defaults to
    // 0.0 — setting only bg_colour must not draw a zero-width (invisible
    // but still present) stroke attribute, since that's dead markup.
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("label", strs(&["hi"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text_with(GeomText::default().bg_colour((255, 255, 255)))
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(!label_text_element(&svg).contains("stroke"), "{svg}");
    assert!(!svg.contains("paint-order"), "{svg}");
}

#[test]
fn geom_label_and_geom_text_repel_never_gain_a_halo_attribute() {
    // Neither GeomLabel nor GeomTextRepel were given a bg_colour/bg_r
    // setting in #229 (ggrepel's own geom_label_repel has no bg.colour
    // either) — this just confirms the shared TextStyle field they all
    // pass through stays `None`/`0.0` for them.
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0])),
        ("y", floats(&[1.0, 2.0])),
        ("label", strs(&["a", "b"])),
    ]);
    let label_svg = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_label()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(!label_svg.contains("paint-order"), "{label_svg}");

    let repel_svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text_repel()
        .render_svg_native_with_size(300, 300)
        .unwrap();
    assert!(!repel_svg.contains("paint-order"), "{repel_svg}");
}

/// Raster backends have no native stroke-text; #229 approximates the halo
/// with an 8-offset outline pass. Smoke-test both so a panic in that path
/// (not just the SVG one above) would fail CI.
#[cfg(feature = "canvas")]
#[test]
fn canvas_pixel_backend_halo_renders_without_panicking() {
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("label", strs(&["hi"])),
    ]);
    let (w, h, rgba) = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text_with(GeomText::default().bg_colour((255, 255, 255)).bg_r(2.0))
        .render_rgba_with_size(300, 200)
        .unwrap();
    assert_eq!(rgba.len(), (w as usize) * (h as usize) * 4);
}

#[cfg(all(feature = "plotters", not(target_arch = "wasm32")))]
#[test]
fn plotters_bitmap_backend_halo_renders_without_panicking() {
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("label", strs(&["hi"])),
    ]);
    let png = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text_with(GeomText::default().bg_colour((0, 0, 0)).bg_r(2.0))
        .render_png_with_size(300, 200)
        .unwrap();
    assert!(!png.is_empty());
}
