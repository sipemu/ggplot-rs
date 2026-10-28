//! `geom_sankey` (#298): edge-list → column layout, ribbons, labels, no axes.

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::geom::sankey::GeomSankey;
use ggplot_rs::GGPlot;

fn flows() -> Vec<(String, Vec<Value>)> {
    let e = [
        ("Coal", "Power", 40.0),
        ("Gas", "Power", 30.0),
        ("Gas", "Heat", 20.0),
        ("Power", "Homes", 50.0),
        ("Power", "Industry", 20.0),
    ];
    let s = |k: usize| {
        e.iter()
            .map(|r| Value::Str(if k == 0 { r.0 } else { r.1 }.to_string()))
            .collect()
    };
    vec![
        ("src".into(), s(0)),
        ("dst".into(), s(1)),
        ("w".into(), e.iter().map(|r| Value::Float(r.2)).collect()),
    ]
}

fn aes() -> Aes {
    Aes::new().from("src").to("dst").weight("w")
}

#[test]
fn renders_nodes_links_labels_and_hides_axes() {
    let (svg, warnings) = GGPlot::new(flows())
        .aes(aes())
        .geom_sankey()
        .render_svg_native_with_warnings(700, 420)
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(svg.contains("<title>Gas → Heat: 20</title>"));
    assert!(
        svg.contains("<title>Power: 70</title>"),
        "max(in 70, out 70)"
    );
    for name in ["Coal", "Gas", "Power", "Heat", "Homes", "Industry"] {
        assert!(svg.contains(&format!(">{name}<")), "label {name}");
    }
    // Position guides are hidden even under the default (grey) theme, and
    // the per-node fill mapping draws no legend.
    assert!(!svg.contains(">0.0<") && !svg.contains(">1.0<") && !svg.contains(">2.0<"));
    assert!(!svg.contains(">node<") && !svg.contains(">fill<"));
    assert!(!svg.contains("NaN"));
    // 6 nodes + 5 links carry hover titles.
    assert_eq!(svg.matches("<title>").count(), 11);
}

#[test]
fn hidden_guides_survive_a_later_theme_preset_and_weight_is_optional() {
    let svg = GGPlot::new(flows())
        .aes(Aes::new().from("src").to("dst"))
        .geom_sankey_with(GeomSankey {
            labels: false,
            ..Default::default()
        })
        .theme_bw()
        .render_svg_native_with_size(600, 400)
        .unwrap();
    assert!(
        svg.contains("<title>Gas → Heat: 1</title>"),
        "unweighted = 1"
    );
    assert!(!svg.contains(">Coal<"), "labels off");
    assert!(!svg.contains(">0.0<"));
}

#[test]
fn build_exposes_layout_columns() {
    let built = GGPlot::new(flows()).aes(aes()).geom_sankey().build();
    let d = &built.layers[0].data;
    let parts = d.column("part").unwrap();
    assert_eq!(parts.iter().filter(|p| p.key_str() == "node").count(), 6);
    assert_eq!(parts.iter().filter(|p| p.key_str() == "link").count(), 5);
    assert!(d.has_column("fill"));
}
