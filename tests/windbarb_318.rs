//! `geom_windbarb` (#318).

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::geom::windbarb::GeomWindbarb;
use ggplot_rs::GGPlot;

fn obs() -> Vec<(String, Vec<Value>)> {
    let f = |v: &[f64]| v.iter().map(|&x| Value::Float(x)).collect();
    vec![
        ("t".into(), f(&[0.0, 1.0, 2.0, 3.0])),
        ("kt".into(), f(&[1.0, 15.0, 65.0, 30.0])),
        ("dir".into(), f(&[0.0, 90.0, 225.0, 300.0])),
    ]
}

#[test]
fn glyphs_for_calm_barbs_and_pennants() {
    let (svg, warnings) = GGPlot::new(obs())
        .aes(Aes::new().x("t"))
        .geom_windbarb(GeomWindbarb::new("kt", "dir"))
        .render_svg_native_with_warnings(500, 200)
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert!(svg.contains("<title>2: 65 from 225°</title>"));
    // Calm: one circle; 65 kt: one pennant polygon.
    let marks = |tag: &str| {
        svg.match_indices(tag)
            .filter(|(i, _)| {
                svg[*i..]
                    .split('>')
                    .next()
                    .unwrap()
                    .contains("data-geom=\"windbarb\"")
            })
            .count()
    };
    assert_eq!(marks("<circle"), 1);
    assert_eq!(marks("<polygon"), 1);
    // shafts: 3, barbs: 15 kt → 1 full + 1 half, 65 → 1 full + 1 half, 30 → 3 full.
    assert_eq!(marks("<polyline"), 3 + 2 + 2 + 3);
    assert!(!svg.contains("NaN"));
}

#[test]
fn meters_per_second_are_converted() {
    let svg = GGPlot::new(obs())
        .aes(Aes::new().x("t"))
        .geom_windbarb(GeomWindbarb::new("kt", "dir").meters_per_second())
        .render_svg_native_with_size(500, 200)
        .unwrap();
    // 65 m/s ≈ 126 kt → 2 pennants plus barbs; 30 m/s ≈ 58 kt → 1 pennant.
    let pennants = svg
        .match_indices("<polygon")
        .filter(|(i, _)| {
            svg[*i..]
                .split('>')
                .next()
                .unwrap()
                .contains("data-geom=\"windbarb\"")
        })
        .count();
    assert_eq!(pennants, 2 + 1);
}
