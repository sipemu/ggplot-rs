//! #409: `guide_override` — ggplot2's `guide_legend(override.aes = …)`.

use ggplot_rs::prelude::*;

fn plot() -> GGPlot {
    GGPlot::new(vec![
        (
            "x".to_string(),
            (0..6).map(|i| Value::Float(i as f64)).collect(),
        ),
        (
            "y".to_string(),
            (0..6).map(|i| Value::Float(i as f64)).collect(),
        ),
        (
            "g".to_string(),
            (0..6)
                .map(|i| Value::Str(["a", "b"][i % 2].into()))
                .collect(),
        ),
    ])
    .aes(Aes::new().x("x").y("y").color("g"))
    .geom_point()
}

fn radii(svg: &str) -> Vec<f64> {
    svg.split("<circle")
        .skip(1)
        .filter_map(|t| {
            let a = t.find(" r=\"")? + 4;
            t[a..].split('"').next()?.parse().ok()
        })
        .collect()
}

#[test]
fn override_sets_key_size_and_colour() {
    let base = plot().render_svg_native_with_size(400, 300).unwrap();
    let svg = plot()
        .guide_override(
            Aesthetic::Color,
            KeyOverride::new()
                .size(6.0)
                .alpha(0.5)
                .shape(PointShape::Square),
        )
        .render_svg_native_with_size(400, 300)
        .unwrap();
    // 6 data points stay circles; the 2 keys turn into squares.
    assert_eq!(radii(&base).len(), 8);
    assert_eq!(radii(&svg).len(), 6, "{svg}");
    assert!(svg.matches("opacity=\"0.5").count() >= 2, "{svg}");
    // `guides(..)` afterwards keeps it.
    let kept = plot()
        .guide_override(
            Aesthetic::Color,
            KeyOverride::new().shape(PointShape::Square),
        )
        .guides(GuideLegend::new().with_title("G"))
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert_eq!(radii(&kept).len(), 6);
}
