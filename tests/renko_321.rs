//! Renko and point-and-figure (#321).

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::GGPlot;

fn prices() -> Vec<(String, Vec<Value>)> {
    let p = [10.0, 12.0, 15.0, 14.0, 12.0, 11.0, 14.0, 16.0];
    vec![("close".into(), p.iter().map(|&v| Value::Float(v)).collect())]
}

#[test]
fn renko_bricks_and_point_figure_marks() {
    let (svg, warnings) = GGPlot::new(prices())
        .aes(Aes::new().y("close"))
        .geom_renko(1.0)
        .render_svg_native_with_warnings(500, 300)
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    // 10→16 net with a 15→11 reversal: 5 up, 3 down, 4 up bricks.
    assert_eq!(svg.matches("data-geom=\"renko\"").count(), 5 + 3 + 4);
    assert!(svg.contains(">Brick<"));

    let svg = GGPlot::new(prices())
        .aes(Aes::new().y("close"))
        .geom_point_figure(1.0, 3)
        .render_svg_native_with_size(500, 300)
        .unwrap();
    // X columns draw two strokes per box, O boxes one circle.
    assert_eq!(svg.matches("<title>O ").count(), 4);
    assert!(svg.contains("<title>X 15–16</title>"));
}
