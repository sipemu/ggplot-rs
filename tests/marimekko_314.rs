//! `geom_variwide` / `geom_marimekko` (#314).

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::GGPlot;

fn s(v: &[&str]) -> Vec<Value> {
    v.iter().map(|x| Value::Str(x.to_string())).collect()
}
fn f(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&x| Value::Float(x)).collect()
}

#[test]
fn variwide_widths_and_category_axis() {
    let data = vec![
        ("country".to_string(), s(&["A", "B", "C"])),
        ("gdp_pc".to_string(), f(&[50.0, 20.0, 35.0])),
        ("pop".to_string(), f(&[1.0, 4.0, 3.0])),
    ];
    let built = GGPlot::new(data.clone())
        .aes(Aes::new().x("country").y("gdp_pc").weight("pop"))
        .geom_variwide()
        .build();
    let d = &built.layers[0].data;
    let x1: Vec<f64> = d
        .column("xmax")
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    assert_eq!(x1, vec![0.125, 0.625, 1.0]);
    let x = built.scales.get(&ggplot_rs::aes::Aesthetic::X).unwrap();
    let labels: Vec<String> = x.breaks().into_iter().map(|b| b.1).collect();
    assert_eq!(labels, ["A", "B", "C"]);

    let svg = GGPlot::new(data)
        .aes(Aes::new().x("country").y("gdp_pc").weight("pop"))
        .geom_variwide()
        .render_svg_native_with_size(500, 300)
        .unwrap();
    assert!(svg.contains("<title>B: 20 (width 4)</title>"));
    assert!(svg.contains(">C<"));
}

#[test]
fn marimekko_segments_stack_to_one() {
    let data = vec![
        ("market".to_string(), s(&["EU", "EU", "US", "US", "US"])),
        ("brand".to_string(), s(&["X", "Y", "X", "Y", "Z"])),
        ("sales".to_string(), f(&[30.0, 10.0, 20.0, 20.0, 20.0])),
    ];
    let (svg, warnings) = GGPlot::new(data)
        .aes(Aes::new().x("market").fill("brand").weight("sales"))
        .geom_marimekko()
        .render_svg_native_with_warnings(500, 300)
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(svg.contains("<title>EU / X: 30 (75%)</title>"));
    assert!(svg.contains(">75%<"), "segment label");
    assert!(svg.contains(">100%<"), "percent y axis");
    assert!(svg.contains(">Z<"), "fill legend");
}
