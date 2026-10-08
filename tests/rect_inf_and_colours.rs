//! `-Inf`/`Inf` rect bounds mean "panel edge" (ggplot2) and don't train
//! scales; theme presets keep `primary_color`; explicit `geom_*_with` colours
//! win over it; sorted discrete colour scales.

use ggplot_rs::aes::{Aes, Aesthetic};
use ggplot_rs::annotate::Annotation;
use ggplot_rs::data::Value;
use ggplot_rs::geom::col::GeomCol;
use ggplot_rs::geom::point::GeomPoint;
use ggplot_rs::geom::rect::GeomRect;
use ggplot_rs::scale::color::{RGBAColor, ScaleColorDiscrete};
use ggplot_rs::GGPlot;

fn floats(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&f| Value::Float(f)).collect()
}
fn strs(v: &[&str]) -> Vec<Value> {
    v.iter().map(|s| Value::Str(s.to_string())).collect()
}
fn frame(cols: Vec<(&str, Vec<Value>)>) -> Vec<(String, Vec<Value>)> {
    cols.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

fn root_attr<'a>(svg: &'a str, name: &str) -> &'a str {
    let head = &svg[..svg.find('>').unwrap()];
    let key = format!(" {name}=\"");
    let i = head.find(&key).unwrap() + key.len();
    &head[i..i + head[i..].find('"').unwrap()]
}

fn nums(s: &str) -> Vec<f64> {
    s.split(' ').map(|t| t.parse().unwrap()).collect()
}

/// `(x, y, w, h)` of the first `<rect>` filled with `fill`.
fn rect_with_fill(svg: &str, fill: &str) -> (f64, f64, f64, f64) {
    let needle = format!("fill=\"{fill}\"");
    let at = svg.find(&needle).expect("rect with fill");
    let start = svg[..at].rfind("<rect ").unwrap();
    let tag = &svg[start..at];
    let get = |k: &str| -> f64 {
        let key = format!(" {k}=\"");
        let i = tag.find(&key).unwrap() + key.len();
        tag[i..i + tag[i..].find('"').unwrap()].parse().unwrap()
    };
    (get("x"), get("y"), get("width"), get("height"))
}

fn points_data() -> Vec<(String, Vec<Value>)> {
    frame(vec![
        ("x", floats(&[0.0, 10.0])),
        ("y", floats(&[0.0, 100.0])),
    ])
}

#[test]
fn geom_rect_infinite_bounds_span_the_panel() {
    let band = frame(vec![
        ("x0", floats(&[2.0])),
        ("x1", floats(&[4.0])),
        ("y0", floats(&[f64::NEG_INFINITY])),
        ("y1", floats(&[f64::INFINITY])),
    ]);
    let (svg, warnings) = GGPlot::new(points_data())
        .aes(Aes::new().x("x").y("y"))
        .geom_rect_with(GeomRect {
            fill: (1, 2, 3),
            color: (1, 2, 3),
            alpha: 0.2,
            line_width: 0.0,
        })
        .layer_data(band)
        .layer_aes(Aes::new().xmin("x0").xmax("x1").ymin("y0").ymax("y1"))
        .geom_point()
        .render_svg_native_with_warnings(400, 300)
        .unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    // Infinite bounds don't train the y scale.
    assert_eq!(root_attr(&svg, "data-ydomain"), "-5 105");
    let p = nums(root_attr(&svg, "data-plot"));
    let (_, y, _, h) = rect_with_fill(&svg, "#010203");
    assert!((y - p[1]).abs() < 0.02, "top edge {y} vs panel {}", p[1]);
    assert!((h - p[3]).abs() < 0.02, "height {h} vs panel {}", p[3]);
}

#[test]
fn annotate_rect_infinite_bounds_span_the_panel() {
    let svg = GGPlot::new(points_data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .annotate(Annotation::Rect {
            xmin: f64::NEG_INFINITY,
            xmax: f64::INFINITY,
            ymin: 40.0,
            ymax: 60.0,
            fill: (4, 5, 6),
            alpha: 0.3,
        })
        .render_svg_native_with_size(400, 300)
        .unwrap();
    let p = nums(root_attr(&svg, "data-plot"));
    let (x, _, w, _) = rect_with_fill(&svg, "#040506");
    assert!((x - p[0]).abs() < 0.02);
    assert!((w - p[2]).abs() < 0.02);
}

#[test]
fn infinite_rect_on_discrete_axis_is_not_a_level() {
    let bars = frame(vec![("x", strs(&["a", "b"])), ("y", floats(&[1.0, 2.0]))]);
    let band = frame(vec![
        ("x0", floats(&[f64::NEG_INFINITY])),
        ("x1", floats(&[f64::INFINITY])),
        ("y0", floats(&[0.5])),
        ("y1", floats(&[1.5])),
    ]);
    let svg = GGPlot::new(bars)
        .aes(Aes::new().x("x").y("y"))
        .geom_col()
        .geom_rect_with(GeomRect {
            fill: (7, 8, 9),
            ..Default::default()
        })
        .layer_data(band)
        .layer_aes(Aes::new().xmin("x0").xmax("x1").ymin("y0").ymax("y1"))
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert_eq!(
        root_attr(&svg, "data-xlevels"),
        "[&quot;a&quot;,&quot;b&quot;]"
    );
    let p = nums(root_attr(&svg, "data-plot"));
    let (x, _, w, _) = rect_with_fill(&svg, "#070809");
    assert!((x - p[0]).abs() < 0.02 && (w - p[2]).abs() < 0.02);
}

#[test]
fn infinite_points_are_still_dropped() {
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0])),
        ("y", floats(&[f64::INFINITY, 2.0])),
    ]);
    let built = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .try_build()
        .unwrap();
    assert_eq!(built.layers[0].data.nrows(), 1);
}

#[test]
fn theme_preset_keeps_primary_color() {
    for plot in [
        GGPlot::new(points_data())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .primary_color((0x12, 0x34, 0x56))
            .theme_minimal(),
        GGPlot::new(points_data())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .primary_color((0x12, 0x34, 0x56))
            .theme_void(),
        GGPlot::new(points_data())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .primary_color((0x12, 0x34, 0x56))
            .theme(ggplot_rs::theme::Theme::default()),
    ] {
        let svg = plot.render_svg_native_with_size(300, 200).unwrap();
        assert!(
            svg.contains("fill=\"#123456\""),
            "primary survived the preset"
        );
    }
}

#[test]
fn explicit_geom_colours_beat_primary_color() {
    let svg = GGPlot::new(points_data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point_with(GeomPoint {
            color: (0xaa, 0x00, 0x00),
            ..Default::default()
        })
        .geom_line()
        .primary_color((0x12, 0x34, 0x56))
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("<circle") && svg.contains("fill=\"#AA0000\""));
    // The default-configured line still takes the brand colour.
    assert!(svg.contains("stroke=\"#123456\""));

    let bars = frame(vec![("x", strs(&["a"])), ("y", floats(&[1.0]))]);
    let svg = GGPlot::new(bars)
        .aes(Aes::new().x("x").y("y"))
        .geom_col_with(GeomCol {
            fill: (0xff, 0xff, 0xff),
            ..Default::default()
        })
        .primary_color((0x12, 0x34, 0x56))
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("fill=\"#FFFFFF\" fill-opacity=\"1.000\" stroke"));
    assert!(!svg.contains("#123456"));
}

#[test]
fn sorted_discrete_scale_is_order_independent() {
    let palette = vec![
        RGBAColor::new(0x11, 0, 0),
        RGBAColor::new(0x22, 0, 0),
        RGBAColor::new(0x33, 0, 0),
    ];
    let render = |levels: &[&str]| {
        let data = frame(vec![("x", strs(levels)), ("y", floats(&[1.0, 2.0, 3.0]))]);
        GGPlot::new(data)
            .aes(Aes::new().x("x").y("y").fill("x"))
            .geom_col()
            .scale_fill(
                ScaleColorDiscrete::new(Aesthetic::Fill)
                    .with_palette(palette.clone())
                    .sorted(),
            )
            .render_svg_native_with_size(300, 200)
            .unwrap()
    };
    for svg in [render(&["c", "a", "b"]), render(&["b", "c", "a"])] {
        // "a" → first palette colour, "c" → third, whatever the data order.
        assert!(svg.contains(r##"fill="#110000" fill-opacity="1.000" stroke="#323232" stroke-width="0.50" data-x="a""##), "{svg}");
        assert!(svg.contains(r##"fill="#330000" fill-opacity="1.000" stroke="#323232" stroke-width="0.50" data-x="c""##));
    }

    // Convenience form with the default palette.
    let data = frame(vec![("x", strs(&["z", "m"])), ("y", floats(&[1.0, 2.0]))]);
    let built = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").fill("x"))
        .geom_col()
        .scale_fill_discrete_sorted()
        .try_build()
        .unwrap();
    let labels: Vec<String> = built
        .scales
        .get(&Aesthetic::Fill)
        .unwrap()
        .breaks()
        .into_iter()
        .map(|(_, l)| l)
        .collect();
    assert_eq!(labels, vec!["m", "z"]);
}
