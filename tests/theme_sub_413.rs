//! #413: `theme_sub_*` shortcuts, `rel()`, `theme_replace` / `theme_with`,
//! `legend_ticks_length`.

use ggplot_rs::prelude::*;

fn plot() -> GGPlot {
    GGPlot::new(vec![
        (
            "x".to_string(),
            (0..5).map(|i| Value::Float(i as f64)).collect(),
        ),
        (
            "y".to_string(),
            (0..5).map(|i| Value::Float(i as f64)).collect(),
        ),
    ])
    .aes(Aes::new().x("x").y("y").color("y"))
    .geom_point()
}

fn svg(p: GGPlot) -> String {
    p.render_svg_native_with_size(400, 300).unwrap()
}

#[test]
fn theme_sub_axis_x_touches_only_x() {
    let red = ElementText {
        color: (200, 0, 0),
        ..theme_gray().axis_text_x.clone()
    };
    let s = svg(plot().theme_update(theme_sub_axis_x(Some(red), None, None, None)));
    // Five red x labels, the y labels untouched.
    assert_eq!(s.matches("fill=\"#C80000\"").count(), 5, "{s}");
    let line = ElementLine {
        visible: true,
        color: (0, 0, 200),
        ..theme_gray().axis_line.clone()
    };
    let s = svg(plot().theme_update(theme_sub_axis_y(None, None, Some(line), None)));
    assert!(s.contains("stroke=\"#0000C8\""));
}

#[test]
fn rel_and_theme_with_merge() {
    let t = theme_gray();
    assert!((t.title.rel(1.5).size - t.title.size * 1.5).abs() < 1e-9);
    assert!((t.axis_line.rel(2.0).width - t.axis_line.width * 2.0).abs() < 1e-9);
    let s = svg(plot().theme_with(|t| t.axis_text_x.size = 20.0));
    assert!(s.contains("font-size=\"20.00\""));
    // theme_replace replaces the whole element.
    let s = svg(plot().theme_replace(ThemeUpdate {
        panel_grid_major: Some(ElementLine {
            visible: false,
            ..theme_gray().panel_grid_major.clone()
        }),
        ..Default::default()
    }));
    assert_ne!(s, svg(plot()));
}

#[test]
fn legend_ticks_length_changes_colourbar_ticks() {
    let base = svg(plot());
    let long = svg(plot().theme_with(|t| t.legend_ticks_length = 8.0));
    assert_ne!(base, long);
}
