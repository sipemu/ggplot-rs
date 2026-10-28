//! `geom_chord` (#299): dependency wheel from an edge list.

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::geom::chord::GeomChord;
use ggplot_rs::GGPlot;

fn migration() -> Vec<(String, Vec<Value>)> {
    let e = [
        ("North", "South", 12.0),
        ("South", "North", 7.0),
        ("North", "East", 5.0),
        ("East", "West", 9.0),
        ("West", "North", 4.0),
        ("South", "West", 6.0),
    ];
    let s = |k: usize| {
        e.iter()
            .map(|r| Value::Str(if k == 0 { r.0 } else { r.1 }.to_string()))
            .collect()
    };
    vec![
        ("from".into(), s(0)),
        ("to".into(), s(1)),
        ("n".into(), e.iter().map(|r| Value::Float(r.2)).collect()),
    ]
}

#[test]
fn renders_arcs_ribbons_and_labels_without_axes() {
    let (svg, warnings) = GGPlot::new(migration())
        .aes(Aes::new().from("from").to("to").weight("n"))
        .geom_chord()
        .render_svg_native_with_warnings(600, 520)
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(svg.contains("<title>North → South: 12</title>"));
    // North: out 12 + 5, in 7 + 4.
    assert!(svg.contains("<title>North: 28</title>"));
    for n in ["North", "South", "East", "West"] {
        assert!(svg.contains(&format!(">{n}<")), "label {n}");
    }
    assert_eq!(svg.matches("<title>").count(), 4 + 6);
    assert!(
        !svg.contains(">1.0<") && !svg.contains(">0.0<"),
        "axes hidden"
    );
    assert!(!svg.contains("NaN"));
}

#[test]
fn circle_stays_round_under_any_theme() {
    let built = GGPlot::new(migration())
        .aes(Aes::new().from("from").to("to"))
        .geom_chord_with(GeomChord {
            labels: false,
            ..Default::default()
        })
        .theme_minimal()
        .build();
    let ratio = built.theme.aspect_ratio.unwrap();
    assert!((ratio - 1.25 / 1.45).abs() < 1e-12);
    let d = &built.layers[0].data;
    assert_eq!(d.nrows(), 4 + 6);
}
