//! `geom_venn` (#312).

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::GGPlot;

fn data() -> Vec<(String, Vec<Value>)> {
    vec![
        (
            "s".into(),
            ["A", "B", "A&B"]
                .iter()
                .map(|x| Value::Str(x.to_string()))
                .collect(),
        ),
        (
            "n".into(),
            [100.0, 60.0, 25.0]
                .iter()
                .map(|&v| Value::Float(v))
                .collect(),
        ),
    ]
}

#[test]
fn renders_two_circles_and_region_labels() {
    let (svg, warnings) = GGPlot::new(data())
        .aes(Aes::new().sets("s").weight("n"))
        .geom_venn()
        .render_svg_native_with_warnings(500, 500)
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(svg.contains("<title>A: 100</title>") && svg.contains("<title>B: 60</title>"));
    for t in [">A<", ">B<", ">25<"] {
        assert!(svg.contains(t), "label {t}");
    }
    // Two different set colours.
    assert!(svg.contains("#F8766D") && svg.contains("#00BA38"));
    assert!(!svg.contains("NaN"));
}

#[test]
fn sizes_only_through_intersections_and_disjoint_sets() {
    let only_pair = vec![
        (
            "s".to_string(),
            vec![Value::Str("X&Y".into()), Value::Str("Z".into())],
        ),
        ("n".to_string(), vec![Value::Float(5.0), Value::Float(3.0)]),
    ];
    let built = GGPlot::new(only_pair)
        .aes(Aes::new().sets("s").weight("n"))
        .geom_venn()
        .build();
    let parts = built.layers[0].data.column("part").unwrap();
    assert_eq!(parts.iter().filter(|p| p.key_str() == "circle").count(), 3);
}
