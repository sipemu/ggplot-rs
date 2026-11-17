//! A density of outcomes and a line around zero, each side in its own colour
//! (DataZooDE/anofox-visualization#1256). `cargo run --example sign_split_demo`
use ggplot_rs::data::{DataFrame, Value};
use ggplot_rs::geom::density::GeomDensity;
use ggplot_rs::geom::line::GeomLine;
use ggplot_rs::prelude::*;

fn main() {
    let (red, green) = ((192, 58, 98), (31, 122, 82));
    let mut df = DataFrame::new();
    df.add_column(
        "x".into(),
        (0..400)
            .map(|i| Value::Float((i as f64 * 0.731).sin() * 3.0 + (i as f64 * 0.13).cos() + 0.8))
            .collect(),
    );
    let svg = GGPlot::new(df)
        .aes(Aes::new().x("x"))
        .geom_density_with(GeomDensity {
            alpha: 0.6,
            split: Some(SignSplit::new(red, green).axis(SplitAxis::X)),
            ..Default::default()
        })
        .render_svg_native_with_size(500, 300)
        .unwrap();
    std::fs::write("sign_split_density.svg", svg).unwrap();
    let mut df = DataFrame::new();
    df.add_column(
        "x".into(),
        (0..30).map(|i| Value::Float(i as f64)).collect(),
    );
    df.add_column(
        "y".into(),
        (0..30)
            .map(|i| Value::Float((i as f64 * 0.4).sin() * 3.0 + 0.5))
            .collect(),
    );
    let svg = GGPlot::new(df)
        .aes(Aes::new().x("x").y("y"))
        .geom_line_with(GeomLine {
            width: 2.0,
            interpolate: Interpolate::MonotoneCubic,
            split: Some(SignSplit::new(red, green)),
            ..Default::default()
        })
        .render_svg_native_with_size(500, 300)
        .unwrap();
    std::fs::write("sign_split_line.svg", svg).unwrap();
}
