//! `geom_flow` flow maps (#324).
#![cfg(feature = "sf")]

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::geom::flow::GeomFlow;
use ggplot_rs::spatial::SfProjection;
use ggplot_rs::GGPlot;

fn flows() -> Vec<(String, Vec<Value>)> {
    let f = |v: &[f64]| v.iter().map(|&x| Value::Float(x)).collect();
    vec![
        ("lon0".into(), f(&[13.4, 2.35, -0.13])),
        ("lat0".into(), f(&[52.5, 48.86, 51.5])),
        ("lon1".into(), f(&[2.35, -0.13, 13.4])),
        ("lat1".into(), f(&[48.86, 51.5, 52.5])),
        ("n".into(), f(&[300.0, 120.0, 40.0])),
        (
            "route".into(),
            ["Berlin → Paris", "Paris → London", "London → Berlin"]
                .iter()
                .map(|s| Value::Str(s.to_string()))
                .collect(),
        ),
    ]
}

#[test]
fn projected_curved_flows_with_arrows() {
    let aes = || {
        Aes::new()
            .x("lon0")
            .y("lat0")
            .xend("lon1")
            .yend("lat1")
            .weight("n")
            .label("route")
    };
    let built = GGPlot::new(flows())
        .aes(aes())
        .geom_flow(GeomFlow::default().project(SfProjection::Mercator))
        .build();
    let x = built.layers[0].data.column("x").unwrap()[0]
        .as_f64()
        .unwrap();
    assert!(
        (x - 13.4f64.to_radians() * 6_371_008.8).abs() < 1.0
            || (x - 13.4f64.to_radians()).abs() < 1e-9,
        "projected: {x}"
    );

    let (svg, warnings) = GGPlot::new(flows())
        .aes(aes())
        .geom_flow(GeomFlow::default().project(SfProjection::Mercator))
        .coord_sf()
        .render_svg_native_with_warnings(500, 400)
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(svg.contains("<title>Berlin → Paris: 300</title>"));
    assert_eq!(svg.matches("data-part=\"arrow\"").count(), 3);
    assert!(!svg.contains("NaN"));
}
