//! `geom_tilemap` (#317).

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::geom::tilemap::{us_state_grid, TileShape};
use ggplot_rs::GGPlot;

fn data() -> Vec<(String, Vec<Value>)> {
    let g = us_state_grid();
    vec![
        (
            "st".into(),
            g.iter().map(|s| Value::Str(s.0.into())).collect(),
        ),
        (
            "c".into(),
            g.iter().map(|s| Value::Float(s.1 as f64)).collect(),
        ),
        (
            "r".into(),
            g.iter().map(|s| Value::Float(s.2 as f64)).collect(),
        ),
        (
            "v".into(),
            (0..g.len()).map(|i| Value::Float(i as f64)).collect(),
        ),
    ]
}

#[test]
fn every_shape_draws_one_tile_per_state() {
    for shape in [
        TileShape::Square,
        TileShape::Hexagon,
        TileShape::Circle,
        TileShape::Diamond,
    ] {
        let (svg, warnings) = GGPlot::new(data())
            .aes(Aes::new().x("c").y("r").fill("v").label("st"))
            .geom_tilemap(shape)
            .render_svg_native_with_warnings(700, 450)
            .unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(svg.matches("<title>").count(), 51, "{shape:?}");
        assert!(svg.contains("<title>TX: 49</title>"));
        assert!(svg.contains(">CA<"), "labels fit");
        assert!(!svg.contains("NaN"));
    }
}
