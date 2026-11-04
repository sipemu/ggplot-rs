//! #406: geom_function, stat_connect, stat_unique, stat_manual,
//! stat_sf_coordinates (stat_summary_hex: tests/summary_hex_406.rs).

use ggplot_rs::data::DataFrame;
use ggplot_rs::prelude::*;

fn xy(x: &[f64], y: &[f64]) -> Vec<(String, Vec<Value>)> {
    vec![
        ("x".into(), x.iter().map(|v| Value::Float(*v)).collect()),
        ("y".into(), y.iter().map(|v| Value::Float(*v)).collect()),
    ]
}

/// Vertex count of the longest `<polyline>` (axis / grid lines have 2).
fn polyline_points(svg: &str) -> usize {
    svg.split("<polyline")
        .skip(1)
        .filter_map(|t| {
            let a = t.find("points=\"")? + 8;
            Some(t[a..].split('"').next()?.split_whitespace().count())
        })
        .max()
        .unwrap_or(0)
}

#[test]
fn geom_function_without_data() {
    let svg = GGPlot::new(Vec::<(String, Vec<Value>)>::new())
        .geom_function_xlim(|x| x * x, -2.0, 2.0)
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert_eq!(polyline_points(&svg), 101, "n = 101 points, as ggplot2");
}

#[test]
fn geom_function_over_the_data_range() {
    let svg = GGPlot::new(xy(&[0.0, 1.0, 2.0], &[0.0, 1.0, 4.0]))
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .geom_function(|x| x * x)
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert_eq!(polyline_points(&svg), 101);
}

#[test]
fn stat_connect_draws_steps() {
    let plot = || GGPlot::new(xy(&[1.0, 2.0, 3.0], &[1.0, 3.0, 2.0])).aes(Aes::new().x("x").y("y"));
    let hv = plot()
        .stat_connect(Connection::Hv)
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert_eq!(polyline_points(&hv), 5);
    let linear = plot()
        .stat_connect(Connection::Linear)
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert_eq!(polyline_points(&linear), 3);
}

#[test]
fn stat_unique_and_stat_manual() {
    let data = xy(&[1.0, 1.0, 1.0, 2.0], &[1.0, 1.0, 1.0, 2.0]);
    let svg = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y"))
        .stat_unique()
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert_eq!(svg.matches("<circle").count(), 2);
    // stat_manual: one centroid point per group.
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .stat_manual(|df: &DataFrame| {
            let mean = |c: &str| {
                let v: Vec<f64> = df
                    .column(c)
                    .unwrap()
                    .iter()
                    .filter_map(|v| v.as_f64())
                    .collect();
                v.iter().sum::<f64>() / v.len() as f64
            };
            let mut out = DataFrame::new();
            out.add_column("x".into(), vec![Value::Float(mean("x"))]);
            out.add_column("y".into(), vec![Value::Float(mean("y"))]);
            out
        })
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert_eq!(svg.matches("<circle").count(), 1);
}

#[cfg(feature = "sf")]
#[test]
fn stat_sf_coordinates_puts_text_on_a_map() {
    let data = vec![
        (
            "geometry".to_string(),
            vec![
                Value::Str("POLYGON ((0 0, 4 0, 4 4, 0 4, 0 0))".into()),
                Value::Str("POLYGON ((6 0, 10 0, 10 4, 6 4, 6 0))".into()),
            ],
        ),
        (
            "name".to_string(),
            vec![Value::Str("West".into()), Value::Str("East".into())],
        ),
    ];
    let svg = GGPlot::new(data)
        .geom_sf()
        .geom_text()
        .layer_aes(Aes::new().label("name"))
        .stat(StatSfCoordinates::default())
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert!(svg.contains(">West<") && svg.contains(">East<"), "{svg}");
}
