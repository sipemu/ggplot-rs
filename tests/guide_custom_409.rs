//! #409: `guide_custom()` — a legend entry drawn by your own code.

use ggplot_rs::prelude::*;
use ggplot_rs::render::backend::RectStyle;

fn data() -> Vec<(String, Vec<Value>)> {
    vec![
        ("x".into(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("y".into(), vec![Value::Float(1.0), Value::Float(2.0)]),
        (
            "g".into(),
            vec![Value::Str("a".into()), Value::Str("b".into())],
        ),
    ]
}

fn swatch() -> GuideCustom {
    GuideCustom::new(60.0, 20.0, |b, r| {
        b.draw_rect(
            (r.x, r.y),
            (r.x + r.width, r.y + r.height),
            &RectStyle {
                fill: Some((1, 2, 3)),
                ..Default::default()
            },
        )
    })
    .with_title("Custom")
}

fn plot_left(svg: &str) -> [f64; 4] {
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
fn custom_guide_draws_after_the_scale_legends() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
        .guide_custom(swatch())
        .render_svg_native_with_size(500, 300)
        .unwrap();
    assert!(svg.contains(">Custom<"));
    let rect = svg.find(r##"fill="#010203""##).expect("custom rect");
    assert!(rect > svg.find(">b<").unwrap(), "after the colour keys");
    // In the legend band, right of the panel.
    let tag = &svg[svg[..rect].rfind("<rect").unwrap()..rect];
    let i = tag.find(" x=\"").unwrap() + 4;
    let x: f64 = tag[i..].split('"').next().unwrap().parse().unwrap();
    let p = plot_left(&svg);
    assert!(x > p[0] + p[2]);
}

#[test]
fn custom_guide_alone_reserves_a_legend_band() {
    let base = || GGPlot::new(data()).aes(Aes::new().x("x").y("y")).geom_point();
    let plain = base().render_svg_native_with_size(500, 300).unwrap();
    let with = base()
        .guide_custom(GuideCustom::new(150.0, 20.0, |_, _| Ok(())))
        .render_svg_native_with_size(500, 300)
        .unwrap();
    // The band holds the whole 150px-wide box.
    assert!(plot_left(&with)[2] < plot_left(&plain)[2] - 150.0);
}

#[test]
fn horizontal_custom_guide_reserves_its_height() {
    let base = || {
        GGPlot::new(data())
            .aes(Aes::new().x("x").y("y").color("g"))
            .geom_point()
            .legend_position(LegendPosition::Bottom)
    };
    let plain = base().render_svg_native_with_size(500, 300).unwrap();
    let with = base()
        .guide_custom(GuideCustom::new(40.0, 60.0, |_, _| Ok(())))
        .render_svg_native_with_size(500, 300)
        .unwrap();
    assert!(plot_left(&with)[3] < plot_left(&plain)[3] - 30.0);
}
