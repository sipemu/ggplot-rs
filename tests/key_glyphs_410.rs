//! #410: per-layer legend key glyphs, combined keys, show.legend per aesthetic.

use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    let f = |v: &[f64]| v.iter().map(|x| Value::Float(*x)).collect::<Vec<_>>();
    let s = |v: &[&str]| {
        v.iter()
            .map(|x| Value::Str(x.to_string()))
            .collect::<Vec<_>>()
    };
    vec![
        ("x".into(), f(&[1.0, 2.0, 3.0, 1.0, 2.0, 3.0])),
        ("y".into(), f(&[1.0, 3.0, 2.0, 2.0, 4.0, 3.0])),
        ("g".into(), s(&["a", "a", "a", "b", "b", "b"])),
    ]
}

/// Marks of `tag` the legend adds: the plot with its legend minus the same
/// plot without one.
fn legend_marks(plot: impl Fn() -> GGPlot, tag: &str) -> usize {
    let count = |svg: String| svg.matches(&format!("<{tag}")).count();
    let with = count(render(plot()));
    let without = count(render(plot().legend_position(LegendPosition::None)));
    with - without
}

fn render(p: GGPlot) -> String {
    p.render_svg_native_with_size(600, 400).unwrap()
}

#[test]
fn point_and_line_layers_share_one_key_each() {
    let plot = || {
        GGPlot::new(data())
            .aes(Aes::new().x("x").y("y").color("g"))
            .geom_line()
            .geom_point()
    };
    // Two keys, each a line *and* a point.
    assert_eq!(legend_marks(plot, "circle"), 2, "point glyphs");
    assert_eq!(legend_marks(plot, "polyline"), 2, "line glyphs");
}

#[test]
fn key_glyph_override_and_show_legend_for() {
    let plot = || {
        GGPlot::new(data())
            .aes(Aes::new().x("x").y("y").color("g"))
            .geom_line()
            .key_glyph(KeyGlyph::Timeseries)
            .geom_point()
            .show_legend_for(Aesthetic::Color, false)
    };
    assert_eq!(
        legend_marks(plot, "circle"),
        0,
        "points hidden from the colour legend"
    );
    // The zig-zag timeseries glyph: one 4-point polyline per key.
    let svg = render(plot());
    let zigzags = svg
        .split("<polyline")
        .filter(|t| {
            t.split("points=\"")
                .nth(1)
                .is_some_and(|p| p.split('"').next().unwrap().split_whitespace().count() == 4)
        })
        .count();
    assert!(zigzags >= 2, "timeseries keys");
}

#[test]
fn boxplot_key_has_whiskers_and_box() {
    let plot = || {
        GGPlot::new(data())
            .aes(Aes::new().x("g").y("y").fill("g"))
            .geom_boxplot()
    };
    // Per key: 2 whiskers + the median line.
    assert_eq!(legend_marks(plot, "polyline"), 6, "boxplot key lines");
}

#[cfg(feature = "plotters")]
#[test]
fn png_keys_are_not_clipped_away() {
    let plot = || {
        GGPlot::new(data())
            .aes(Aes::new().x("x").y("y").color("g"))
            .geom_point()
    };
    let with = plot().render_png_with_size(600, 400).unwrap();
    let without = plot()
        .legend_position(LegendPosition::None)
        .render_png_with_size(600, 400)
        .unwrap();
    assert_ne!(with, without);
}
