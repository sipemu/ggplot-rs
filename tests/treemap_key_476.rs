//! `geom_treemap` (#476): a tile's `data-x` is its `key` where mapped (a
//! code to drill on), its `label` (the shown name) otherwise.

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::GGPlot;

fn data() -> Vec<(String, Vec<Value>)> {
    let s = |v: &[&str]| v.iter().map(|x| Value::Str(x.to_string())).collect();
    vec![
        ("code".into(), s(&["DE-BY", "DE-BE", "FR-IDF"])),
        ("name".into(), s(&["Bavaria", "Berlin", "Île-de-France"])),
        ("country".into(), s(&["DE", "DE", "FR"])),
        (
            "gdp".into(),
            [7.0, 2.0, 7.5].iter().map(|&v| Value::Float(v)).collect(),
        ),
    ]
}

#[test]
fn key_sets_data_x_label_stays_the_text() {
    let (svg, warnings) = GGPlot::new(data())
        .aes(
            Aes::new()
                .weight("gdp")
                .subgroup("country")
                .label("name")
                .key("code"),
        )
        .geom_treemap()
        .render_svg_native_with_warnings(800, 560)
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    for code in ["DE-BY", "DE-BE", "FR-IDF"] {
        assert!(svg.contains(&format!("data-x=\"{code}\"")), "{code}: {svg}");
    }
    assert!(!svg.contains("data-x=\"Bavaria\""), "{svg}");
    assert!(svg.contains(">Bavaria<"), "the label is still drawn");
    assert!(
        svg.contains("<title>Bavaria: 7</title>"),
        "tooltip names the label"
    );
}

#[test]
fn without_a_key_data_x_is_the_label() {
    let (svg, _) = GGPlot::new(data())
        .aes(Aes::new().weight("gdp").subgroup("country").label("name"))
        .geom_treemap()
        .render_svg_native_with_warnings(800, 560)
        .unwrap();
    assert!(svg.contains("data-x=\"Bavaria\""), "{svg}");
}
