//! sipemu/ggplot-rs#341: a Bottom legend under `theme_void` (no x axis) was
//! placed 30px below the panel — past the band the layout reserved, at the
//! canvas edge — so its keys were clipped to zero height.

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;
use ggplot_rs::theme::LegendPosition;

fn plot() -> GGPlot {
    let d = vec![
        (
            "x".to_string(),
            (0..6).map(|i| Value::Float(i as f64)).collect::<Vec<_>>(),
        ),
        (
            "y".to_string(),
            (0..6).map(|i| Value::Float(i as f64)).collect(),
        ),
        (
            "g".to_string(),
            (0..6)
                .map(|i| Value::Str(["a", "b", "c"][i % 3].into()))
                .collect(),
        ),
    ];
    GGPlot::new(d)
        .aes(Aes::new().x("x").y("y").fill("g"))
        .geom_col()
        .theme_void()
        .legend_position(LegendPosition::Bottom)
}

/// `y` of every 12×12 legend key rect.
fn key_ys(svg: &str) -> Vec<f64> {
    svg.split("<rect")
        .skip(1)
        .filter(|r| r.contains("width=\"12") && r.contains("height=\"12"))
        .map(|r| {
            r.split(" y=\"")
                .nth(1)
                .unwrap()
                .split('"')
                .next()
                .unwrap()
                .parse()
                .unwrap()
        })
        .collect()
}

#[test]
fn native_keys_stay_on_the_canvas() {
    let ys = key_ys(&plot().render_svg_native_with_size(640, 480).unwrap());
    assert_eq!(ys.len(), 3, "{ys:?}");
    assert!(ys.iter().all(|&y| y + 12.0 <= 480.0), "{ys:?}");
}

#[cfg(feature = "plotters")]
#[test]
fn plotters_keys_stay_on_the_canvas() {
    let ys = key_ys(&plot().render_svg_with_size(640, 480).unwrap());
    assert_eq!(ys.len(), 3, "{ys:?}");
    assert!(ys.iter().all(|&y| y + 12.0 <= 480.0), "{ys:?}");
}
