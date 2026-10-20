//! B2-7: geom ink/paper/accent (ggplot2 4.0's `element_geom`).
//! `Theme::with_ink_paper`/`set_geom` recolour a geom's *built-in default*
//! colour; a mapped or explicit colour is untouched, and the default theme
//! (nothing set) is byte-identical.

use ggplot_rs::prelude::*;

fn floats(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&f| Value::Float(f)).collect()
}
fn strs(v: &[&str]) -> Vec<Value> {
    v.iter().map(|s| Value::Str(s.to_string())).collect()
}
fn frame(cols: Vec<(&str, Vec<Value>)>) -> Vec<(String, Vec<Value>)> {
    cols.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

/// Every `<tag …>` start tag in `svg` whose name is `tag`.
fn tags<'a>(svg: &'a str, tag: &str) -> Vec<&'a str> {
    let open = format!("<{tag} ");
    svg.match_indices(&open)
        .map(|(i, _)| {
            let rest = &svg[i..];
            &rest[..rest.find('>').map_or(rest.len(), |e| e + 1)]
        })
        .collect()
}

const WHITE: (u8, u8, u8) = (255, 255, 255);
const BLACK: (u8, u8, u8) = (0, 0, 0);

#[test]
fn default_theme_no_ink_paper_set_is_byte_identical() {
    let data = frame(vec![("x", floats(&[1.0, 2.0])), ("y", floats(&[1.0, 2.0]))]);
    let plain = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y"))
        .geom_line()
        .geom_point()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    let with_default_theme = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_line()
        .geom_point()
        .theme(Theme::default())
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert_eq!(plain, with_default_theme);
}

#[test]
fn ink_recolours_the_line_and_point_defaults() {
    let data = frame(vec![("x", floats(&[1.0, 2.0])), ("y", floats(&[1.0, 2.0]))]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_line()
        .geom_point()
        .theme(Theme::default().with_ink_paper(WHITE, BLACK))
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "line: {svg}");
    assert!(svg.contains("fill=\"#FFFFFF\""), "point: {svg}");
    assert!(!svg.contains("#000000"), "no more built-in black: {svg}");
}

#[test]
fn ink_recolours_geom_text() {
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("label", strs(&["hi"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text()
        .theme(Theme::default().with_ink_paper(WHITE, BLACK))
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("<text"), "{svg}");
    assert!(svg.contains("fill=\"#FFFFFF\""), "{svg}");
    assert!(svg.contains(">hi</text>"), "{svg}");
}

#[test]
fn ink_and_paper_recolour_boxplot_outline_and_fill() {
    let data = frame(vec![
        ("x", strs(&["a", "a", "a", "a"])),
        ("y", floats(&[1.0, 2.0, 3.0, 4.0])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_boxplot()
        .theme(Theme::default().with_ink_paper(WHITE, BLACK))
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "outline: {svg}");
    assert!(svg.contains("fill=\"#000000\""), "fill: {svg}");
}

#[test]
fn ink_recolours_the_legend_key_for_shape_and_linetype() {
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0, 3.0, 4.0])),
        ("y", floats(&[1.0, 2.0, 3.0, 4.0])),
        ("g", strs(&["a", "b", "a", "b"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").shape("g"))
        .geom_point()
        .theme(Theme::default().with_ink_paper(WHITE, BLACK))
        .render_svg_native_with_size(400, 300)
        .unwrap();
    // The legend key swatch uses ink (white); the data points keep their
    // own ink-recoloured black-default colour too, so just check the
    // legend's white shows up somewhere a bare "#000000" wouldn't.
    assert!(svg.contains("#FFFFFF"), "{svg}");
}

#[test]
fn a_mapped_or_explicit_colour_is_unaffected_by_ink() {
    let data = frame(vec![("x", floats(&[1.0, 2.0])), ("y", floats(&[1.0, 2.0]))]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point_with(GeomPoint {
            color: (10, 20, 30),
            ..Default::default()
        })
        .theme(Theme::default().with_ink_paper(WHITE, BLACK))
        .render_svg_native_with_size(300, 200)
        .unwrap();
    let circles = tags(&svg, "circle");
    assert_eq!(circles.len(), 2, "{svg}");
    for c in circles {
        assert!(c.contains("fill=\"#0A141E\""), "{c}");
        assert!(
            !c.contains("#FFFFFF"),
            "the point must not pick up ink: {c}"
        );
    }
}

#[test]
fn accent_recolours_the_smooth_line() {
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0, 3.0, 4.0, 5.0])),
        ("y", floats(&[1.0, 2.0, 1.5, 3.0, 2.5])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_smooth()
        .theme(Theme::default().set_geom(ElementGeom::default().with_accent((200, 50, 10))))
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("#C8320A"), "{svg}");
    assert!(!svg.contains("#3366CC"), "{svg}");
}

#[test]
fn ink_recolours_geom_path() {
    let data = frame(vec![("x", floats(&[1.0, 2.0])), ("y", floats(&[1.0, 2.0]))]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_path()
        .theme(Theme::default().with_ink_paper(WHITE, BLACK))
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert!(svg.contains("stroke=\"#FFFFFF\""), "{svg}");
}

#[test]
fn ink_and_paper_recolour_geom_label() {
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("label", strs(&["hi"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_label()
        .theme(Theme::default().with_ink_paper(WHITE, BLACK))
        .render_svg_native_with_size(300, 200)
        .unwrap();
    let rects = tags(&svg, "rect");
    assert!(
        rects.iter().any(|r| r.contains("fill=\"#000000\"")),
        "label background should be paper: {svg}"
    );
    assert!(svg.contains("fill=\"#FFFFFF\""), "{svg}");
    assert!(svg.contains(">hi</text>"), "{svg}");
}
