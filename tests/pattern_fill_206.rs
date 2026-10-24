//! #206: a scale's `na_pattern` drawn end to end — `geom_tile`/`geom_rect`/
//! `geom_polygon`/`geom_sf`'s no-data cells, and the legend's "no data" key.
#![cfg(feature = "sf")]

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;
use ggplot_rs::render::backend::FillPattern;
use ggplot_rs::scale::binned::ScaleFillBinned;

fn tile_data(values: &[Option<f64>]) -> Vec<(String, Vec<Value>)> {
    let x: Vec<Value> = (0..values.len()).map(|i| Value::Float(i as f64)).collect();
    let y: Vec<Value> = values.iter().map(|_| Value::Float(0.0)).collect();
    let v: Vec<Value> = values
        .iter()
        .map(|o| o.map(Value::Float).unwrap_or(Value::Na))
        .collect();
    vec![("x".into(), x), ("y".into(), y), ("v".into(), v)]
}

fn fill_scale() -> ScaleFillBinned {
    ScaleFillBinned::new(
        Aesthetic::Fill,
        vec![0.0, 10.0, 50.0, 100.0],
        vec![(0, 0, 255), (0, 255, 0), (255, 0, 0)],
    )
    .na_pattern(FillPattern::stripe((10, 20, 30), (200, 210, 220)))
}

#[test]
fn geom_tile_draws_the_na_pattern_for_a_no_data_cell() {
    let svg = GGPlot::new(tile_data(&[Some(5.0), Some(60.0), None]))
        .aes(Aes::new().x("x").y("y").fill("v"))
        .geom_tile()
        .scale_fill(fill_scale())
        .render_svg_native()
        .expect("render");
    assert!(svg.contains("<pattern "), "{svg}");
    assert!(svg.contains("fill=\"url(#"), "{svg}");
}

#[test]
fn geom_tile_without_a_na_pattern_set_draws_the_flat_na_color_as_before() {
    let svg = GGPlot::new(tile_data(&[Some(5.0), None]))
        .aes(Aes::new().x("x").y("y").fill("v"))
        .geom_tile()
        .scale_fill(ScaleFillBinned::new(
            Aesthetic::Fill,
            vec![0.0, 10.0, 50.0, 100.0],
            vec![(0, 0, 255), (0, 255, 0), (255, 0, 0)],
        ))
        .render_svg_native()
        .expect("render");
    assert!(!svg.contains("<pattern "), "{svg}");
    assert!(svg.contains("#7F7F7F"), "{svg}");
}

#[test]
fn geom_rect_draws_the_na_pattern_too() {
    let data: Vec<(String, Vec<Value>)> = vec![
        ("xmin".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
        ("xmax".into(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("ymin".into(), vec![Value::Float(0.0), Value::Float(0.0)]),
        ("ymax".into(), vec![Value::Float(1.0), Value::Float(1.0)]),
        ("v".into(), vec![Value::Float(5.0), Value::Na]),
    ];
    let svg = GGPlot::new(data)
        .aes(
            Aes::new()
                .xmin("xmin")
                .xmax("xmax")
                .ymin("ymin")
                .ymax("ymax")
                .fill("v"),
        )
        .geom_rect()
        .scale_fill(fill_scale())
        .render_svg_native()
        .expect("render");
    assert!(svg.contains("<pattern "), "{svg}");
}

#[test]
fn geom_polygon_draws_the_na_pattern_for_a_no_data_group() {
    // Two triangles, grouped by `fill` (no explicit "group" column) — one
    // group's fill is a real value, the other NA.
    let data: Vec<(String, Vec<Value>)> = vec![
        (
            "x".into(),
            vec![
                Value::Float(0.0),
                Value::Float(1.0),
                Value::Float(0.5),
                Value::Float(2.0),
                Value::Float(3.0),
                Value::Float(2.5),
            ],
        ),
        (
            "y".into(),
            vec![
                Value::Float(0.0),
                Value::Float(0.0),
                Value::Float(1.0),
                Value::Float(0.0),
                Value::Float(0.0),
                Value::Float(1.0),
            ],
        ),
        (
            "v".into(),
            vec![
                Value::Float(5.0),
                Value::Float(5.0),
                Value::Float(5.0),
                Value::Na,
                Value::Na,
                Value::Na,
            ],
        ),
    ];
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").fill("v"))
        .geom_polygon()
        .scale_fill(fill_scale())
        .render_svg_native()
        .expect("render");
    assert!(svg.contains("<pattern "), "{svg}");
}

#[test]
fn geom_sf_draws_the_na_pattern_for_a_no_data_feature() {
    let mut df = ggplot_rs::data::DataFrame::new();
    df.add_column(
        "geometry".into(),
        vec![
            Value::Str("POLYGON ((0 0, 1 0, 1 1, 0 1, 0 0))".into()),
            Value::Str("POLYGON ((2 0, 3 0, 3 1, 2 1, 2 0))".into()),
        ],
    );
    df.add_column("v".into(), vec![Value::Float(5.0), Value::Na]);
    let svg = GGPlot::new(df)
        .aes(Aes::new().fill("v"))
        .geom_sf()
        .scale_fill(fill_scale())
        .render_svg_native()
        .expect("render");
    assert!(svg.contains("<pattern "), "{svg}");
}

#[test]
fn the_legends_no_data_key_draws_the_pattern_swatch() {
    let svg = GGPlot::new(tile_data(&[Some(5.0), Some(60.0), None]))
        .aes(Aes::new().x("x").y("y").fill("v"))
        .geom_tile()
        .scale_fill(fill_scale())
        .render_svg_native()
        .expect("render");
    // The legend text still reads "no data (1)" exactly as #180 (no
    // regression to the count/label logic), and at least one of the
    // registered patterns backs the legend swatch, not just the cell.
    assert!(svg.contains("no data (1)"), "{svg}");
    assert_eq!(
        svg.matches("<pattern ").count(),
        1,
        "cell and swatch share the one registered pattern: {svg}"
    );
}
