//! Issue #9 — computed aesthetics: aes() accepts expressions, not just columns.

use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    vec![
        (
            "idx".to_string(),
            (0..5).map(|i| Value::Float(i as f64)).collect(),
        ),
        (
            "pop".to_string(),
            [
                1_000_000.0,
                2_500_000.0,
                3_000_000.0,
                500_000.0,
                8_000_000.0,
            ]
            .iter()
            .map(|v| Value::Float(*v))
            .collect(),
        ),
        (
            "gdp".to_string(),
            [10.0, 100.0, 1000.0, 50.0, 200.0]
                .iter()
                .map(|v| Value::Float(*v))
                .collect(),
        ),
    ]
}

#[test]
fn expression_maps_to_derived_column() {
    let built = GGPlot::new(data())
        .aes(Aes::new().x("idx").y("pop / 1e6"))
        .geom_point()
        .build();
    let y: Vec<f64> = built.layers[0]
        .data
        .column("y")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();
    assert_eq!(y, vec![1.0, 2.5, 3.0, 0.5, 8.0]);
}

#[test]
fn function_and_arithmetic_expressions_render() {
    // log(gdp) on x, a ratio on y — should build and render without error.
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("log10(gdp)").y("pop / gdp"))
        .geom_point()
        .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}

#[test]
fn bare_column_names_still_work() {
    // Backward compatibility: a plain column name is used directly.
    let built = GGPlot::new(data())
        .aes(Aes::new().x("idx").y("gdp"))
        .geom_point()
        .build();
    let y: Vec<f64> = built.layers[0]
        .data
        .column("y")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();
    assert_eq!(y, vec![10.0, 100.0, 1000.0, 50.0, 200.0]);
}

#[test]
fn expression_as_continuous_color() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("idx").y("gdp").color("log(pop)"))
        .geom_point()
        .scale_color_viridis_c()
        .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}
