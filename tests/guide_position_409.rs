//! #409: `guide_position` — one legend at its own side (ggplot2 3.5's
//! `guide_legend(position =)`), the others at the theme's position.

use ggplot_rs::prelude::*;

fn plot() -> GGPlot {
    let s = |v: &[&str]| v.iter().map(|s| Value::Str(s.to_string())).collect();
    GGPlot::new(vec![
        ("x".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("y".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("g".to_string(), s(&["a", "b"])),
        ("h".to_string(), s(&["p", "q"])),
    ])
    .aes(Aes::new().x("x").y("y").color("g").shape("h"))
    .geom_point()
}

fn text_xy(svg: &str, label: &str) -> (f64, f64) {
    let end = svg.find(&format!(">{label}</text>")).expect(label);
    let tag = &svg[svg[..end].rfind("<text").unwrap()..end];
    let attr = |n: &str| -> f64 {
        let i = tag.find(&format!(" {n}=\"")).unwrap() + n.len() + 3;
        tag[i..].split('"').next().unwrap().parse().unwrap()
    };
    (attr("x"), attr("y"))
}

fn panel(svg: &str) -> [f64; 4] {
    let i = svg.find("data-plot=\"").unwrap() + 11;
    let v: Vec<f64> = svg[i..]
        .split('"')
        .next()
        .unwrap()
        .split(' ')
        .map(|n| n.parse().unwrap())
        .collect();
    [v[0], v[1], v[2], v[3]]
}

#[test]
fn one_legend_moves_to_the_bottom() {
    let svg = plot()
        .guide_position(Aesthetic::Shape, LegendPosition::Bottom)
        .render_svg_native_with_size(500, 350)
        .unwrap();
    let [px, py, pw, ph] = panel(&svg);
    // Colour stays right of the panel, shape sits below it.
    assert!(text_xy(&svg, "a").0 > px + pw);
    assert!(text_xy(&svg, "p").1 > py + ph);
    // Both bands are reserved: narrower and shorter than with one legend.
    let right_only = plot().render_svg_native_with_size(500, 350).unwrap();
    let [_, _, rw, rh] = panel(&right_only);
    assert!((pw - rw).abs() < 1.0 || pw < rw);
    assert!(ph < rh - 10.0);
}

#[test]
fn a_moved_top_legend_lifts_the_title() {
    let svg = plot()
        .labs(Labels {
            title: Some("Headline".into()),
            ..Default::default()
        })
        .guide_position(Aesthetic::Shape, LegendPosition::Top)
        .render_svg_native_with_size(500, 350)
        .unwrap();
    let [_, py, _, _] = panel(&svg);
    let (_, ty) = text_xy(&svg, "Headline");
    let (_, ly) = text_xy(&svg, "p");
    assert!(
        ty < ly && ly < py,
        "title {ty} above legend {ly} above panel {py}"
    );
}

#[test]
fn same_position_changes_nothing() {
    let a = plot().render_svg_native_with_size(500, 350).unwrap();
    let b = plot()
        .guide_position(Aesthetic::Shape, LegendPosition::Right)
        .render_svg_native_with_size(500, 350)
        .unwrap();
    assert_eq!(a, b);
}
