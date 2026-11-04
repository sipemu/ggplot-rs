//! #413: `legend_box_just` (ggplot2's `legend.box.just`) lines several
//! legends up along the stack's cross axis.

use ggplot_rs::prelude::*;

fn plot() -> GGPlot {
    let s = |v: &[&str]| v.iter().map(|s| Value::Str(s.to_string())).collect();
    GGPlot::new(vec![
        ("x".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("y".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        (
            "g".to_string(),
            s(&["a rather long level", "another long one"]),
        ),
        ("h".to_string(), s(&["p", "q"])),
    ])
    .aes(Aes::new().x("x").y("y").color("g").shape("h"))
    .geom_point()
}

/// The x of the text element whose content is `label`.
fn text_x(svg: &str, label: &str) -> f64 {
    let end = svg.find(&format!(">{label}</text>")).expect(label);
    let tag = &svg[svg[..end].rfind("<text").unwrap()..end];
    let i = tag.find(" x=\"").unwrap() + 4;
    tag[i..].split('"').next().unwrap().parse().unwrap()
}

fn render(p: GGPlot) -> String {
    p.render_svg_native_with_size(600, 300).unwrap()
}

#[test]
fn narrower_legend_moves_by_the_width_difference() {
    let left = render(plot());
    let centre = render(plot().theme_with(|t| t.legend_box_just = 0.5));
    let right = render(plot().theme_with(|t| t.legend_box_just = 1.0));
    // The wide colour legend stays put; the narrow shape one moves.
    assert_eq!(
        text_x(&left, "a rather long level"),
        text_x(&right, "a rather long level")
    );
    let (l, c, r) = (
        text_x(&left, "p"),
        text_x(&centre, "p"),
        text_x(&right, "p"),
    );
    assert!(r > l + 20.0, "{l} {r}");
    assert!(((c - l) - (r - l) / 2.0).abs() < 0.02, "{l} {c} {r}");
}

#[test]
fn horizontal_legends_justify_vertically() {
    // A colourbar (tall when horizontal) beside a one-row shape legend.
    let bottom = || {
        GGPlot::new(vec![
            ("x".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
            ("y".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
            ("v".to_string(), vec![Value::Float(0.0), Value::Float(10.0)]),
            (
                "h".to_string(),
                vec![Value::Str("p".into()), Value::Str("q".into())],
            ),
        ])
        .aes(Aes::new().x("x").y("y").color("v").shape("h"))
        .geom_point()
        .legend_position(LegendPosition::Bottom)
    };
    let text_y = |svg: &str, label: &str| -> f64 {
        let end = svg.find(&format!(">{label}</text>")).expect(label);
        let tag = &svg[svg[..end].rfind("<text").unwrap()..end];
        let i = tag.find(" y=\"").unwrap() + 4;
        tag[i..].split('"').next().unwrap().parse().unwrap()
    };
    let top = render(bottom());
    let low = render(bottom().theme_with(|t| t.legend_box_just = 1.0));
    assert!(text_y(&low, "p") > text_y(&top, "p") + 5.0);
    assert_eq!(text_x(&low, "p"), text_x(&top, "p"));
}

#[cfg(feature = "serde")]
#[test]
fn theme_config_names_and_fractions() {
    use ggplot_rs::theme::config::{JustCfg, ThemeConfig};
    let apply = |j: JustCfg| {
        ThemeConfig {
            legend_box_just: Some(j),
            ..Default::default()
        }
        .apply(theme_gray())
    };
    assert_eq!(
        apply(JustCfg::Name("center".into()))
            .unwrap()
            .legend_box_just,
        0.5
    );
    assert_eq!(apply(JustCfg::Fraction(2.0)).unwrap().legend_box_just, 1.0);
    assert!(apply(JustCfg::Name("middle".into())).is_err());
}
