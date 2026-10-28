//! `geom_packed_bubble` (#309).

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::GGPlot;

fn data() -> Vec<(String, Vec<Value>)> {
    let s = |v: &[&str]| v.iter().map(|x| Value::Str(x.to_string())).collect();
    vec![
        ("name".into(), s(&["a", "b", "c", "d", "e"])),
        ("grp".into(), s(&["X", "X", "Y", "Y", "Y"])),
        (
            "w".into(),
            [50.0, 20.0, 30.0, 10.0, 5.0]
                .iter()
                .map(|&v| Value::Float(v))
                .collect(),
        ),
    ]
}

#[test]
fn clustered_bubbles_render_with_outlines_labels_and_square_panel() {
    let built = GGPlot::new(data())
        .aes(Aes::new().weight("w").subgroup("grp").label("name"))
        .geom_packed_bubble()
        .build();
    assert_eq!(built.theme.aspect_ratio, Some(1.0));
    assert_eq!(built.layers[0].data.nrows(), 5 + 2);

    let (svg, warnings) = GGPlot::new(data())
        .aes(Aes::new().weight("w").subgroup("grp").label("name"))
        .geom_packed_bubble()
        .render_svg_native_with_warnings(500, 500)
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(svg.contains("<title>a: 50</title>"));
    assert_eq!(svg.matches("<title>").count(), 5);
    assert!(svg.contains(">a<"), "label fits the biggest bubble");
    assert!(svg.contains(">Y<"), "fill legend by subgroup");
    assert!(!svg.contains(">NA<"), "cluster rows add no NA legend key");
}

#[test]
fn flat_pack_without_subgroups() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().weight("w"))
        .geom_packed_bubble()
        .render_svg_native_with_size(400, 400)
        .unwrap();
    assert_eq!(svg.matches("data-geom=\"packed_bubble\"").count(), 5);
}
