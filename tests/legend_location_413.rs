//! #413: `legend.location = "plot"` — a bottom legend aligned with the plot
//! instead of the panel.

use ggplot_rs::prelude::*;

fn key_x(plot_level: bool) -> f64 {
    let svg = GGPlot::new(vec![
        (
            "x".to_string(),
            (0..4).map(|i| Value::Float(i as f64 * 1000.0)).collect(),
        ),
        (
            "y".to_string(),
            (0..4).map(|i| Value::Float(i as f64 * 100000.0)).collect(),
        ),
        (
            "g".to_string(),
            (0..4)
                .map(|i| Value::Str(["a", "b"][i % 2].into()))
                .collect(),
        ),
    ])
    .aes(Aes::new().x("x").y("y").color("g"))
    .geom_point()
    .legend_position(LegendPosition::Bottom)
    .theme_with(|t| t.legend_location_plot = plot_level)
    .render_svg_native_with_size(400, 300)
    .unwrap();
    let t = svg.split("<text").find(|t| t.contains(">a<")).unwrap();
    let a = t.find(" x=\"").unwrap() + 4;
    t[a..].split('"').next().unwrap().parse().unwrap()
}

#[test]
fn plot_location_moves_the_legend_left() {
    assert!(
        key_x(true) < key_x(false) - 10.0,
        "{} {}",
        key_x(true),
        key_x(false)
    );
}
