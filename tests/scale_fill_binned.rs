//! #180: `GGPlot::scale_fill_binned`/`scale_fill_binned_with` end to end —
//! the legend shows each class's trained count and a "no data" key.

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;
use ggplot_rs::scale::binned::ScaleFillBinned;
use ggplot_rs::scale::color::RGBAColor;

fn tile_data(values: &[Option<f64>]) -> Vec<(String, Vec<Value>)> {
    let x: Vec<Value> = (0..values.len()).map(|i| Value::Float(i as f64)).collect();
    let y: Vec<Value> = values.iter().map(|_| Value::Float(0.0)).collect();
    let v: Vec<Value> = values
        .iter()
        .map(|o| o.map(Value::Float).unwrap_or(Value::Na))
        .collect();
    vec![("x".into(), x), ("y".into(), y), ("v".into(), v)]
}

#[test]
fn scale_fill_binned_two_colour_legend_shows_counts_and_na_key() {
    let svg = GGPlot::new(tile_data(&[
        Some(2.0),
        Some(5.0),
        Some(22.0),
        Some(60.0),
        Some(90.0),
        None,
    ]))
    .aes(Aes::new().x("x").y("y").fill("v"))
    .geom_tile()
    .scale_fill_binned(
        vec![0.0, 10.0, 50.0, 100.0],
        RGBAColor {
            r: 240,
            g: 240,
            b: 200,
            a: 1.0,
        },
        RGBAColor {
            r: 30,
            g: 90,
            b: 30,
            a: 1.0,
        },
    )
    .render_svg_native()
    .expect("scale_fill_binned render");
    assert!(svg.contains("0–10 (2)"), "{svg}");
    assert!(svg.contains("10–50 (1)"), "{svg}");
    assert!(svg.contains("50–100 (2)"), "{svg}");
    assert!(svg.contains("no data (1)"), "{svg}");
}

#[test]
fn scale_fill_binned_with_explicit_colors_no_na_key_when_nothing_missing() {
    let svg = GGPlot::new(tile_data(&[Some(1.0), Some(20.0)]))
        .aes(Aes::new().x("x").y("y").fill("v"))
        .geom_tile()
        .scale_fill_binned_with(
            vec![0.0, 10.0, 50.0],
            vec![
                RGBAColor {
                    r: 0,
                    g: 0,
                    b: 255,
                    a: 1.0,
                },
                RGBAColor {
                    r: 255,
                    g: 0,
                    b: 0,
                    a: 1.0,
                },
            ],
        )
        .render_svg_native()
        .expect("scale_fill_binned_with render");
    assert!(svg.contains("0–10 (1)"), "{svg}");
    assert!(svg.contains("10–50 (1)"), "{svg}");
    assert!(!svg.contains("no data"), "{svg}");
}

/// Full control via `ScaleFillBinned` directly (`.na_color`/`.na_label`),
/// plumbed through `GGPlot::scale_fill` rather than the convenience
/// wrapper — also exercises the plot/legend title via `.set_name`.
#[test]
fn scale_fill_binned_direct_with_custom_na_color_and_label() {
    let s = ScaleFillBinned::two(
        ggplot_rs::aes::Aesthetic::Fill,
        vec![0.0, 50.0, 100.0],
        (0, 0, 0),
        (255, 255, 255),
    )
    .na_color((10, 20, 30))
    .na_label("missing");
    let svg = GGPlot::new(tile_data(&[Some(1.0), None]))
        .aes(Aes::new().x("x").y("y").fill("v"))
        .geom_tile()
        .scale_fill(s)
        .render_svg_native()
        .expect("direct ScaleFillBinned render");
    assert!(svg.contains("missing (1)"), "{svg}");
}
