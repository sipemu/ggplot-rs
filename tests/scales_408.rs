//! #408: scale_*_hue, scale_*_viridis_d, scale_*_distiller, expand_limits.

use ggplot_rs::prelude::*;

fn data(groups: &[&str]) -> Vec<(String, Vec<Value>)> {
    let n = groups.len();
    vec![
        (
            "x".into(),
            (0..n).map(|i| Value::Float(i as f64 + 1.0)).collect(),
        ),
        (
            "y".into(),
            (0..n).map(|i| Value::Float(2.0 * i as f64 + 1.0)).collect(),
        ),
        (
            "g".into(),
            groups.iter().map(|g| Value::Str(g.to_string())).collect(),
        ),
    ]
}

fn svg(p: GGPlot) -> String {
    p.render_svg_native_with_size(400, 300).unwrap()
}

#[test]
fn hue_scale_is_ggplot2s_palette_for_any_n() {
    // R: scales::hue_pal()(2) and (5).
    let s = svg(GGPlot::new(data(&["a", "b"]))
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
        .scale_color_hue(HuePalette::default()));
    assert!(
        s.contains("#F8766D") && s.contains("#00BFC4"),
        "2-level hues"
    );
    let s = svg(GGPlot::new(data(&["a", "b", "c", "d", "e"]))
        .aes(Aes::new().x("x").y("y").fill("g"))
        .geom_col()
        .scale_fill_hue(HuePalette::default()));
    for c in ["#F8766D", "#A3A500", "#00BF7D", "#00B0F6", "#E76BF3"] {
        assert!(s.contains(c), "missing {c}");
    }
}

#[test]
fn viridis_d_and_distiller() {
    let s = svg(GGPlot::new(data(&["a", "b", "c", "d"]))
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
        .scale_color_viridis_d('D', 0.0, 1.0, 1));
    // R: viridis_pal()(4) ends — first and last match exactly.
    assert!(
        s.contains("#440154") && s.contains("#FDE725"),
        "viridis ends"
    );
    let s = svg(GGPlot::new(data(&["a", "b", "c"]))
        .aes(Aes::new().x("x").y("y").color("y"))
        .geom_point()
        .scale_color_distiller(PaletteName::Blues, -1));
    // direction = -1: the lowest value gets Blues' darkest. (ggplot2 uses the
    // 7-class Brewer table, darkest #084594; ggplot-rs ships the 9-class one,
    // darkest #08306B.)
    assert!(
        s.contains("fill=\"#08306B\""),
        "darkest blue at the low end"
    );
}

#[test]
fn expand_limits_includes_zero() {
    let plain = svg(GGPlot::new(data(&["a", "b"]))
        .aes(Aes::new().x("x").y("y"))
        .geom_point());
    let zero = svg(GGPlot::new(data(&["a", "b"]))
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .expand_limits(&[], &[0.0]));
    assert!(
        !plain.contains(">0<") && zero.contains(">0<"),
        "y axis reaches 0"
    );
}
