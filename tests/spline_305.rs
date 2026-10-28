//! Spline interpolation for lines, areas and ribbons (#305).

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::geom::area::GeomArea;
use ggplot_rs::geom::line::GeomLine;
use ggplot_rs::geom::ribbon::GeomRibbon;
use ggplot_rs::prelude::Interpolate;
use ggplot_rs::GGPlot;

fn data() -> Vec<(String, Vec<Value>)> {
    let x = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0];
    let y = [1.0, 3.0, 2.0, 5.0, 4.0, 6.0];
    let f = |v: &[f64]| v.iter().map(|&a| Value::Float(a)).collect();
    vec![
        ("x".into(), f(&x)),
        ("y".into(), f(&y)),
        ("lo".into(), f(&y.map(|v| v - 1.0))),
        ("hi".into(), f(&y.map(|v| v + 1.0))),
    ]
}

/// Vertex count of the first `tag` element drawn by a data layer.
fn points_in_first(svg: &str, tag: &str) -> usize {
    let el = svg
        .match_indices(tag)
        .map(|(i, _)| &svg[i..i + svg[i..].find('>').unwrap()])
        .find(|el| el.contains("data-geom="))
        .unwrap();
    let p = el.find("points=\"").unwrap() + 8;
    el[p..p + el[p..].find('"').unwrap()].split(' ').count()
}

#[test]
fn line_area_and_ribbon_densify_when_smoothed() {
    let render = |plot: GGPlot| plot.render_svg_native_with_size(500, 300).unwrap();
    let aes = || Aes::new().x("x").y("y");
    let straight = render(GGPlot::new(data()).aes(aes()).geom_line());
    let smooth = render(GGPlot::new(data()).aes(aes()).geom_line_with(GeomLine {
        interpolate: Interpolate::MonotoneCubic,
        ..Default::default()
    }));
    assert_eq!(points_in_first(&straight, "<polyline"), 6);
    assert!(points_in_first(&smooth, "<polyline") > 60);

    let area = render(GGPlot::new(data()).aes(aes()).geom_area_with(GeomArea {
        interpolate: Interpolate::CatmullRom,
        ..Default::default()
    }));
    assert!(points_in_first(&area, "<polygon") > 120);

    let ribbon = render(
        GGPlot::new(data())
            .aes(Aes::new().x("x").ymin("lo").ymax("hi"))
            .geom_ribbon_with(GeomRibbon {
                interpolate: Interpolate::MonotoneCubic,
                ..Default::default()
            }),
    );
    assert!(points_in_first(&ribbon, "<polygon") > 120);
    for svg in [&smooth, &area, &ribbon] {
        assert!(!svg.contains("NaN"));
    }
}
