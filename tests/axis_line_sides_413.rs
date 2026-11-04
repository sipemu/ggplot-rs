//! #413: per-side axis lines and ticks (`axis.line.x.bottom`,
//! `axis.ticks.y.left`, …) inherit from the per-axis, then shared, ones.

use ggplot_rs::prelude::*;
use ggplot_rs::theme::elements::ElementLine;

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
    .aes(Aes::new().x("x").y("y"))
    .geom_point()
    .theme_classic()
}

fn red() -> ElementLine {
    ElementLine {
        color: (255, 0, 0),
        ..theme_classic().axis_line.clone()
    }
}

#[test]
fn side_inherits_then_overrides() {
    let t = theme_classic();
    assert!(std::ptr::eq(t.axis_line_at(AxisSide::Bottom), &t.axis_line));
    let t = Theme {
        axis_line_x: Some(red()),
        ..theme_classic()
    };
    assert_eq!(t.axis_line_at(AxisSide::Top).color, (255, 0, 0));
    let t = Theme {
        axis_line_x_bottom: Some(ElementLine::blank()),
        ..t
    };
    assert!(!t.axis_line_at(AxisSide::Bottom).visible);
    assert_eq!(t.axis_line_at(AxisSide::Top).color, (255, 0, 0));
    assert!(std::ptr::eq(t.axis_line_at(AxisSide::Left), &t.axis_line));
}

#[test]
fn left_axis_line_colour_is_drawn() {
    let svg = plot()
        .theme_with(|t| {
            t.axis_line_y_left = Some(red());
            t.axis_ticks_y_left = Some(ElementLine {
                color: (0, 0, 255),
                ..t.axis_ticks.clone()
            });
        })
        .render_svg_native_with_size(400, 300)
        .unwrap();
    assert_eq!(svg.matches(r##"stroke="#FF0000""##).count(), 1, "{svg}");
    assert!(svg.matches(r##"stroke="#0000FF""##).count() >= 3);
    let blank = plot()
        .theme_with(|t| t.axis_line_x_bottom = Some(ElementLine::blank()))
        .render_svg_native_with_size(400, 300)
        .unwrap();
    let base = plot().render_svg_native_with_size(400, 300).unwrap();
    assert_eq!(
        base.matches("<polyline").count() - blank.matches("<polyline").count(),
        1
    );
}

#[cfg(feature = "serde")]
#[test]
fn theme_config_side_keys() {
    use ggplot_rs::theme::config::{LineCfg, ThemeConfig};
    let cfg = ThemeConfig {
        axis_line_x: Some(LineCfg {
            color: Some([255, 0, 0]),
            ..Default::default()
        }),
        axis_ticks_y_right: Some(LineCfg {
            visible: Some(false),
            ..Default::default()
        }),
        ..Default::default()
    };
    let t = cfg.apply(theme_classic()).unwrap();
    assert_eq!(t.axis_line_at(AxisSide::Top).color, (255, 0, 0));
    assert!(!t.axis_ticks_at(AxisSide::Right).visible);
    assert!(t.axis_ticks_at(AxisSide::Left).visible);
}
