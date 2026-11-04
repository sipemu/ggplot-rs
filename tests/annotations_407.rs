//! #407: annotation_logticks, annotation_raster, annotation_custom (inset).

use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    let x: Vec<f64> = (0..30).map(|i| 10f64.powf(i as f64 / 10.0)).collect();
    vec![
        ("x".into(), x.iter().map(|v| Value::Float(*v)).collect()),
        (
            "y".into(),
            x.iter().map(|v| Value::Float(v.sqrt())).collect(),
        ),
    ]
}

fn lines(svg: &str) -> usize {
    svg.matches("<polyline").count() + svg.matches("<line").count()
}

#[test]
fn logticks_on_a_log10_axis() {
    let base = || {
        GGPlot::new(data())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .scale_x_log10()
    };
    let plain = base().render_svg_native_with_size(500, 400).unwrap();
    let ticks = base()
        .annotation_logticks("b")
        .render_svg_native_with_size(500, 400)
        .unwrap();
    // The expanded panel spans ≈ −0.15 … 3.05 in log10: 0.8, 0.9, 1–9,
    // 10–90, 100–900 and 1000 → 30 ticks, as ggplot2 draws over the panel.
    let added = lines(&ticks) - lines(&plain);
    assert_eq!(added, 30, "{added} log ticks");
    let both = base()
        .annotate(Annotation::logticks("bl").outside())
        .render_svg_native_with_size(500, 400)
        .unwrap();
    assert!(lines(&both) > lines(&ticks), "left side adds ticks too");
}

#[test]
fn raster_is_placed_as_an_image() {
    // A 1×1 PNG.
    let png: Vec<u8> = vec![
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 248, 207, 192, 0, 0,
        3, 1, 1, 0, 201, 254, 146, 239, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .annotation_raster(png, f64::NEG_INFINITY, f64::INFINITY, 5.0, 10.0)
        .render_svg_native_with_size(500, 400)
        .unwrap();
    assert!(svg.contains("<image"), "raster drawn");
}

fn inset() -> GGPlot {
    GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_line()
        .title("INSET")
        .theme_bw()
}

fn with_inset() -> GGPlot {
    GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .annotation_custom(inset(), 400.0, f64::INFINITY, f64::NEG_INFINITY, 12.0)
}

#[test]
fn custom_inset_draws_a_nested_plot() {
    let svg = with_inset().render_svg_native_with_size(600, 400).unwrap();
    assert!(svg.contains(">INSET<"), "inset title drawn");
    // The inset's own line and axes sit inside the host panel.
    assert!(svg.matches("<polyline").count() > 2);
}

#[cfg(feature = "plotters")]
#[test]
fn custom_inset_in_png() {
    let main = with_inset;
    let png = main().render_png_with_size(600, 400).unwrap();
    let plain = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .render_png_with_size(600, 400)
        .unwrap();
    assert_ne!(png, plain, "inset drawn in PNG too");
    if let Ok(dir) = std::env::var("GGPLOT_VISUAL_DIR") {
        std::fs::write(format!("{dir}/inset.png"), &png).unwrap();
        let log = GGPlot::new(data())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .scale_x_log10()
            .scale_y_log10()
            .annotation_logticks("bl")
            .theme_bw()
            .render_png_with_size(600, 400)
            .unwrap();
        std::fs::write(format!("{dir}/logticks.png"), log).unwrap();
    }
}
