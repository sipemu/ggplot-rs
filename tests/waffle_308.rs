//! `geom_waffle` / `geom_parliament` (#308).

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::GGPlot;

fn data() -> Vec<(String, Vec<Value>)> {
    vec![
        (
            "party".into(),
            ["Left", "Centre", "Right"]
                .iter()
                .map(|s| Value::Str(s.to_string()))
                .collect(),
        ),
        (
            "seats".into(),
            [41.0, 27.0, 32.0]
                .iter()
                .map(|&v| Value::Float(v))
                .collect(),
        ),
    ]
}

#[test]
fn waffle_draws_one_square_per_unit_with_a_legend() {
    let (svg, warnings) = GGPlot::new(data())
        .aes(Aes::new().fill("party").weight("seats"))
        .geom_waffle(10)
        .render_svg_native_with_warnings(600, 400)
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(svg.matches("data-geom=\"waffle\"").count(), 100);
    assert!(svg.contains(">Centre<"), "fill legend");
    assert!(!svg.contains("NaN"));
}

#[test]
fn parliament_draws_one_seat_per_unit() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().fill("party").weight("seats"))
        .geom_parliament(0)
        .render_svg_native_with_size(600, 360)
        .unwrap();
    assert_eq!(svg.matches("data-geom=\"waffle\"").count(), 100);
    // Squares only for the waffle: seats are circles.
    assert!(svg.matches("<circle").count() >= 100);
}

#[test]
fn parliament_bottom_legend_is_drawn() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().fill("party").weight("seats"))
        .geom_parliament(0)
        .theme_void()
        .legend_position(ggplot_rs::theme::LegendPosition::Bottom)
        .render_svg_native_with_size(600, 360)
        .unwrap();
    assert!(
        svg.contains(">Centre<"),
        "{}",
        &svg[svg.len().saturating_sub(3000)..]
    );
}
