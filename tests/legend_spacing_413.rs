//! #413: `legend_key_spacing_x` / `_y` (ggplot2's `legend.key.spacing.*`)
//! and `legend_box_spacing` (`legend.box.spacing`).

use ggplot_rs::prelude::*;

fn plot() -> GGPlot {
    GGPlot::new(vec![
        (
            "x".to_string(),
            (0..3).map(|i| Value::Float(i as f64)).collect(),
        ),
        (
            "y".to_string(),
            (0..3).map(|i| Value::Float(i as f64)).collect(),
        ),
        (
            "g".to_string(),
            ["a", "b", "c"].map(|s| Value::Str(s.into())).to_vec(),
        ),
    ])
    .aes(Aes::new().x("x").y("y").color("g"))
    .geom_point()
}

/// `(x, y)` of each legend label `a`, `b`, `c`.
fn label_pos(svg: &str) -> Vec<(f64, f64)> {
    ["a", "b", "c"]
        .iter()
        .map(|l| {
            let end = svg.find(&format!(">{l}</text>")).expect("label");
            let tag = &svg[svg[..end].rfind("<text").unwrap()..end];
            let attr = |n: &str| -> f64 {
                let i = tag.find(&format!(" {n}=\"")).unwrap() + n.len() + 3;
                tag[i..].split('"').next().unwrap().parse().unwrap()
            };
            (attr("x"), attr("y"))
        })
        .collect()
}

fn render(p: GGPlot) -> String {
    p.render_svg_native_with_size(500, 300).unwrap()
}

#[test]
fn key_spacing_y_spreads_vertical_keys() {
    let base = label_pos(&render(plot()));
    let spaced = label_pos(&render(
        plot().theme_with(|t| t.legend_key_spacing_y = Some(10.0)),
    ));
    let pitch = |p: &[(f64, f64)]| p[1].1 - p[0].1;
    assert!((pitch(&spaced) - pitch(&base) - 10.0).abs() < 0.02);
    assert!((spaced[2].1 - spaced[1].1 - pitch(&spaced)).abs() < 0.02);
}

#[test]
fn key_spacing_x_spreads_horizontal_keys() {
    let bottom = || plot().legend_position(LegendPosition::Bottom);
    let tight = label_pos(&render(
        bottom().theme_with(|t| t.legend_key_spacing_x = Some(0.0)),
    ));
    let wide = label_pos(&render(
        bottom().theme_with(|t| t.legend_key_spacing_x = Some(20.0)),
    ));
    let pitch = |p: &[(f64, f64)]| p[1].0 - p[0].0;
    assert!((pitch(&wide) - pitch(&tight) - 20.0).abs() < 0.02);
}

#[test]
fn box_spacing_moves_the_legend_off_the_panel() {
    // The default gap on the right is `legend_margin.left` (10px).
    let panel_right = |svg: &str| -> f64 {
        let i = svg.find("data-plot=\"").unwrap() + 11;
        let v: Vec<f64> = svg[i..]
            .split('"')
            .next()
            .unwrap()
            .split(' ')
            .map(|n| n.parse().unwrap())
            .collect();
        v[0] + v[2]
    };
    let svg = render(plot().theme_with(|t| t.legend_box_spacing = Some(40.0)));
    let gap = label_pos(&svg)[0].0 - panel_right(&svg);
    let svg0 = render(plot());
    let gap0 = label_pos(&svg0)[0].0 - panel_right(&svg0);
    assert!((gap - gap0 - 30.0).abs() < 0.02, "{gap} vs {gap0}");
}

#[cfg(feature = "serde")]
#[test]
fn theme_config_keys() {
    let cfg = ggplot_rs::theme::config::ThemeConfig {
        legend_key_spacing_x: Some(4.0),
        legend_key_spacing_y: Some(6.0),
        legend_box_spacing: Some(-1.0),
        ..Default::default()
    };
    let t = cfg.apply(theme_gray()).unwrap();
    assert_eq!(t.legend_key_spacing_x, Some(4.0));
    assert_eq!(t.legend_key_spacing_y, Some(6.0));
    assert_eq!(t.legend_box_spacing, Some(0.0));
}
