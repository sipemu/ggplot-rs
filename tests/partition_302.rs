//! `geom_sunburst` / `geom_icicle` (#302): hierarchical partitions.

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::GGPlot;

fn data() -> Vec<(String, Vec<Value>)> {
    let s = |v: &[&str]| v.iter().map(|x| Value::Str(x.to_string())).collect();
    vec![
        (
            "continent".into(),
            s(&["Europe", "Europe", "Europe", "Asia", "Asia", "Asia"]),
        ),
        (
            "country".into(),
            s(&["Germany", "Germany", "France", "Japan", "India", "India"]),
        ),
        (
            "city".into(),
            s(&["Berlin", "Munich", "Paris", "Tokyo", "Delhi", "Mumbai"]),
        ),
        (
            "pop".into(),
            [3.7, 1.5, 2.1, 14.0, 11.0, 12.5]
                .iter()
                .map(|&v| Value::Float(v))
                .collect(),
        ),
    ]
}

#[test]
fn sunburst_renders_rings_with_hover_and_hole() {
    let (svg, warnings) = GGPlot::new(data())
        .aes(Aes::new().weight("pop"))
        .geom_sunburst(["continent", "country", "city"])
        .render_svg_native_with_warnings(600, 600)
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    // 2 continents + 4 countries + 6 cities.
    assert_eq!(svg.matches("<title>").count(), 12);
    assert!(
        svg.contains("<title>Asia / India: 23.5 (52.5%)</title>"),
        "{svg}"
    );
    assert!(svg.contains(">Asia<"));
    // Branches are coloured by their top-level ancestor (discrete fill).
    assert!(svg.contains(r##"fill="#F8766D""##) && svg.contains(r##"fill="#00BA38""##));
    assert!(!svg.contains("NaN"));
}

#[test]
fn icicle_puts_the_root_level_on_top() {
    let built = GGPlot::new(data())
        .aes(Aes::new().weight("pop"))
        .geom_icicle(["continent", "country"])
        .build();
    let d = &built.layers[0].data;
    let depth: Vec<f64> = d
        .column("depth")
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    let ymax: Vec<f64> = d
        .column("ymax")
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    for (dp, y) in depth.iter().zip(&ymax) {
        assert_eq!(*y, -dp, "depth {dp} ends at -{dp}");
    }
    let svg = GGPlot::new(data())
        .aes(Aes::new().weight("pop"))
        .geom_icicle(["continent", "country"])
        .render_svg_native_with_size(600, 300)
        .unwrap();
    assert_eq!(svg.matches("<title>").count(), 6);
}
