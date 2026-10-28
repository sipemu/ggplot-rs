//! `geom_treemap` (#301): squarified tiles, subgroup nesting, labels.

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::GGPlot;

fn data() -> Vec<(String, Vec<Value>)> {
    let s = |v: &[&str]| v.iter().map(|x| Value::Str(x.to_string())).collect();
    vec![
        (
            "country".into(),
            s(&[
                "Germany", "France", "Italy", "China", "Japan", "India", "USA", "Canada",
            ]),
        ),
        (
            "region".into(),
            s(&[
                "Europe", "Europe", "Europe", "Asia", "Asia", "Asia", "Americas", "Americas",
            ]),
        ),
        (
            "gdp".into(),
            [4.5, 3.0, 2.2, 18.0, 4.2, 3.7, 27.0, 2.1]
                .iter()
                .map(|&v| Value::Float(v))
                .collect(),
        ),
    ]
}

#[test]
fn tiles_fill_the_panel_exactly_with_group_borders() {
    let built = GGPlot::new(data())
        .aes(Aes::new().weight("gdp").subgroup("region").label("country"))
        .geom_treemap()
        .build();
    let x = built.scales.get(&ggplot_rs::aes::Aesthetic::X).unwrap();
    let (lo, hi) = x.expanded_domain().unwrap();
    assert!(
        lo.abs() < 1e-9 && (hi - 1.6).abs() < 1e-9,
        "no expansion: {lo} {hi}"
    );
    assert!((built.theme.aspect_ratio.unwrap() - 1.0 / 1.6).abs() < 1e-12);
    let d = &built.layers[0].data;
    assert_eq!(d.nrows(), 8 + 3);
    // Filled by subgroup when fill is unmapped.
    assert_eq!(d.column("fill").unwrap()[0], Value::Str("Europe".into()));
}

#[test]
fn renders_tiles_labels_tooltips_and_no_axes() {
    let (svg, warnings) = GGPlot::new(data())
        .aes(Aes::new().weight("gdp").subgroup("region").label("country"))
        .geom_treemap()
        .render_svg_native_with_warnings(800, 560)
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(svg.contains("<title>USA: 27</title>"));
    assert!(svg.contains(">Germany<") && svg.contains(">Americas<"));
    assert!(
        !svg.contains(">0.5<") && !svg.contains(">1.0<"),
        "axes hidden"
    );
    assert!(!svg.contains("NaN"));
}

#[test]
fn flat_treemap_without_subgroups() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().weight("gdp").label("country").fill("gdp"))
        .geom_treemap()
        .render_svg_native_with_size(640, 400)
        .unwrap();
    assert_eq!(svg.matches("<title>").count(), 8);
}
