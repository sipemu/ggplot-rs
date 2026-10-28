//! `geom_network` (#310).

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::geom::network::GeomNetwork;
use ggplot_rs::stat::network::NetworkLayout;
use ggplot_rs::GGPlot;

fn edges() -> Vec<(String, Vec<Value>)> {
    let s = |v: &[&str]| v.iter().map(|x| Value::Str(x.to_string())).collect();
    vec![
        ("a".into(), s(&["A", "B", "C", "D"])),
        ("b".into(), s(&["B", "C", "A", "E"])),
    ]
}

#[test]
fn renders_edges_nodes_and_labels_deterministically() {
    let render = || {
        GGPlot::new(edges())
            .aes(Aes::new().from("a").to("b"))
            .geom_network()
            .render_svg_native_with_size(500, 500)
            .unwrap()
    };
    let svg = render();
    assert_eq!(svg, render(), "seeded layout is deterministic");
    assert!(svg.contains("<title>A — B: 1</title>"));
    assert!(svg.contains("<title>A (degree 2)</title>"));
    for n in ["A", "B", "C", "D", "E"] {
        assert!(svg.contains(&format!(">{n}<")));
    }
    assert!(!svg.contains("NaN"));
}

#[test]
fn directed_circle_layout_with_component_colours() {
    let svg = GGPlot::new(edges())
        .aes(Aes::new().from("a").to("b"))
        .geom_network_with(GeomNetwork {
            layout: NetworkLayout::Circle,
            directed: true,
            color_components: true,
            labels: false,
            ..Default::default()
        })
        .render_svg_native_with_size(400, 400)
        .unwrap();
    assert!(svg.contains("<title>D → E: 1</title>"));
    assert_eq!(
        svg.matches("data-part=\"arrow\"").count(),
        4,
        "one head per edge"
    );
    // Two components → two node fills.
    let fills: std::collections::BTreeSet<&str> = svg
        .match_indices("<circle")
        .map(|(i, _)| {
            let tag = &svg[i..i + svg[i..].find('>').unwrap()];
            let f = tag.find("fill=\"#").unwrap();
            &tag[f + 6..f + 13]
        })
        .collect();
    assert_eq!(fills.len(), 2, "{fills:?}");
    let grid = GGPlot::new(edges())
        .aes(Aes::new().from("a").to("b"))
        .geom_network_with(GeomNetwork {
            layout: NetworkLayout::Grid,
            ..Default::default()
        })
        .render_svg_native_with_size(400, 400)
        .unwrap();
    assert!(!grid.contains("NaN"));
}
