//! `geom_variable_pie` (#319).

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::GGPlot;

#[test]
fn wedges_angles_radii_and_labels() {
    let data = vec![
        (
            "k".to_string(),
            ["A", "B", "C"]
                .iter()
                .map(|s| Value::Str(s.to_string()))
                .collect(),
        ),
        (
            "share".to_string(),
            vec![Value::Float(1.0), Value::Float(1.0), Value::Float(2.0)],
        ),
        (
            "dens".to_string(),
            vec![Value::Float(10.0), Value::Float(20.0), Value::Float(15.0)],
        ),
    ];
    let (svg, warnings) = GGPlot::new(data)
        .aes(Aes::new().y("share").radius("dens").fill("k"))
        .geom_variable_pie()
        .render_svg_native_with_warnings(500, 500)
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(svg.contains("<title>C: 2 (radius 15)</title>"));
    assert_eq!(
        svg.matches("data-geom=\"variable_pie\"").count(),
        3 + 3,
        "wedges + labels"
    );
    assert!(!svg.contains("NaN"));
}
