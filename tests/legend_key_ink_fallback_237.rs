//! #237: a legend key with no aesthetic-mapped colour of its own
//! (`Shape`, `Linetype`, `Size`, `Alpha`, `Linewidth` — `Color`/`Fill`
//! always map a real colour from the data) drew in a *fixed* dark grey
//! regardless of theme, so it was almost invisible on a dark panel.
//! `draw_legend_swatch` now falls back to `theme.geom.ink`, then
//! `theme.legend_text.color`, the same way an unmapped panel mark
//! (`GeomPoint`/`GeomText`'s `apply_element_geom`) already prefers
//! `geom.ink` when the theme sets it.

use ggplot_rs::prelude::*;

fn floats(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&f| Value::Float(f)).collect()
}
fn frame(cols: Vec<(&str, Vec<Value>)>) -> Vec<(String, Vec<Value>)> {
    cols.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

fn size_data() -> Vec<(String, Vec<Value>)> {
    frame(vec![
        ("x", floats(&[1.0, 2.0, 3.0])),
        ("y", floats(&[1.0, 2.0, 3.0])),
        ("sz", floats(&[1.0, 2.0, 3.0])),
    ])
}

/// Every legend key circle's `fill` — a `<circle>` with no `class`
/// attribute (panel marks always carry `class="gg-mark gg-point"`).
fn legend_key_fills(svg: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (start, _) in svg.match_indices("<circle ") {
        let end = svg[start..].find(['/', '>']).unwrap() + start;
        let tag = &svg[start..end];
        if tag.contains("class=") {
            continue; // a panel mark
        }
        let f_start = tag.find("fill=\"").unwrap() + 6;
        let f_end = tag[f_start..].find('"').unwrap() + f_start;
        out.push(tag[f_start..f_end].to_string());
    }
    out
}

#[test]
fn an_explicit_geom_ink_wins_over_legend_text_color() {
    let svg = GGPlot::new(size_data())
        .aes(Aes::new().x("x").y("y").size("sz"))
        .geom_point()
        .theme(Theme::default().set_geom(ElementGeom::default().with_ink((255, 255, 255))))
        .render_svg_native_with_size(300, 200)
        .unwrap();
    for fill in legend_key_fills(&svg) {
        assert_eq!(fill, "#FFFFFF", "{svg}");
    }
}

#[test]
fn with_no_geom_ink_set_a_light_legend_text_color_is_used_not_a_fixed_dark_grey() {
    // The actual #237 scenario: a theme that sets `legend_text.color`
    // for a dark panel's readability, but — plausibly, since it's a
    // less commonly touched field — never also sets `geom.ink`.
    let svg = GGPlot::new(size_data())
        .aes(Aes::new().x("x").y("y").size("sz"))
        .geom_point()
        .theme(Theme::default().set_legend_text(ElementText {
            color: (240, 240, 240),
            ..Default::default()
        }))
        .render_svg_native_with_size(300, 200)
        .unwrap();
    for fill in legend_key_fills(&svg) {
        assert_eq!(fill, "#F0F0F0", "{svg}");
        assert_ne!(
            fill, "#323232",
            "should not fall back to the old fixed dark grey"
        );
    }
}

#[test]
fn the_default_theme_uses_legend_text_color_not_the_old_fixed_grey() {
    let svg = GGPlot::new(size_data())
        .aes(Aes::new().x("x").y("y").size("sz"))
        .geom_point()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    let fills = legend_key_fills(&svg);
    assert!(!fills.is_empty(), "{svg}");
    for fill in fills {
        // The base theme's legend_text.color is pure black.
        assert_eq!(fill, "#000000", "{svg}");
    }
}
