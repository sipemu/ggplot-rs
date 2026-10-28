//! `geom_bullet` (#307).

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::geom::bullet::GeomBullet;
use ggplot_rs::GGPlot;

fn data() -> Vec<(String, Vec<Value>)> {
    let f = |v: &[f64]| v.iter().map(|&x| Value::Float(x)).collect();
    vec![
        (
            "kpi".into(),
            ["Revenue", "Profit"]
                .iter()
                .map(|s| Value::Str(s.to_string()))
                .collect(),
        ),
        ("actual".into(), f(&[270.0, 22.0])),
        ("goal".into(), f(&[250.0, 26.0])),
        ("r1".into(), f(&[150.0, 20.0])),
        ("r2".into(), f(&[225.0, 25.0])),
        ("r3".into(), f(&[300.0, 30.0])),
    ]
}

#[test]
fn per_row_ranges_targets_and_scale_extent() {
    let geom = || {
        GeomBullet::default()
            .with_target("goal")
            .with_range_columns(["r1", "r2", "r3"])
    };
    let built = GGPlot::new(data())
        .aes(Aes::new().x("kpi").y("actual"))
        .geom_bullet(geom())
        .build();
    let (lo, hi) = built
        .scales
        .get(&ggplot_rs::aes::Aesthetic::Y)
        .unwrap()
        .domain()
        .unwrap();
    assert_eq!((lo, hi), (0.0, 300.0), "bands and targets train the scale");

    let svg = GGPlot::new(data())
        .aes(Aes::new().x("kpi").y("actual"))
        .geom_bullet(geom())
        .render_svg_native_with_size(600, 260)
        .unwrap();
    assert!(svg.contains("<title>Revenue: 270 (target 250)</title>"));
    // 2 KPIs × (3 bands + 1 bar) rects, 2 target ticks.
    assert_eq!(svg.matches("data-geom=\"bullet\"").count(), 2 * 4 + 2);
    assert!(svg.contains("#C81E1E"), "target colour");
}

#[test]
fn shared_ranges_and_vertical() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("kpi").y("actual"))
        .geom_bullet_vertical(GeomBullet::default().with_ranges([100.0, 200.0]))
        .render_svg_native_with_size(400, 400)
        .unwrap();
    assert!(svg.contains("<title>Profit: 22</title>"));
    assert_eq!(svg.matches("data-geom=\"bullet\"").count(), 2 * 3);
}
