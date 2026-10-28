//! `geom_arcdiagram` (#300): nodes on a line, links as arcs.

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::geom::arcdiagram::GeomArcdiagram;
use ggplot_rs::GGPlot;

fn edges() -> Vec<(String, Vec<Value>)> {
    let e = [
        ("A", "B", 3.0),
        ("A", "D", 1.0),
        ("C", "B", 2.0),
        ("D", "C", 4.0),
    ];
    let s = |k: usize| {
        e.iter()
            .map(|r| Value::Str(if k == 0 { r.0 } else { r.1 }.to_string()))
            .collect()
    };
    vec![
        ("from".into(), s(0)),
        ("to".into(), s(1)),
        ("w".into(), e.iter().map(|r| Value::Float(r.2)).collect()),
    ]
}

#[test]
fn renders_nodes_arcs_and_labels() {
    let (svg, warnings) = GGPlot::new(edges())
        .aes(Aes::new().from("from").to("to").weight("w"))
        .geom_arcdiagram()
        .render_svg_native_with_warnings(600, 360)
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(svg.contains("<title>D → C: 4</title>"));
    assert!(svg.contains("<title>A: 4</title>"), "A: out 3 + 1");
    for n in ["A", "B", "C", "D"] {
        assert!(svg.contains(&format!(">{n}<")), "label {n}");
    }
    assert_eq!(svg.matches("<title>").count(), 8);
    assert!(!svg.contains(">0.0<") && !svg.contains("NaN"));
}

#[test]
fn directed_below_extends_the_y_range_downward() {
    let up = GGPlot::new(edges())
        .aes(Aes::new().from("from").to("to"))
        .geom_arcdiagram()
        .build();
    let down = GGPlot::new(edges())
        .aes(Aes::new().from("from").to("to"))
        .geom_arcdiagram_with(GeomArcdiagram {
            directed_below: true,
            ..Default::default()
        })
        .build();
    let y = |b: &ggplot_rs::build::BuiltPlot| {
        b.scales
            .get(&ggplot_rs::aes::Aesthetic::Y)
            .unwrap()
            .domain()
            .unwrap()
    };
    // D(3) → C(2) and C(2) → B(1) run leftward → below the line.
    assert!(y(&down).0 < y(&up).0);
    assert!(y(&down).1 <= y(&up).1);
}
