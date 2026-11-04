//! #413: per-side axis text (`axis.text.x.top`, `axis.text.y.right`),
//! `theme_test()`, `margin_auto()` / `margin_part()`.

use ggplot_rs::prelude::*;
use ggplot_rs::scale::sec_axis::SecAxis;

fn data() -> Vec<(String, Vec<Value>)> {
    vec![
        ("x".into(), (0..5).map(|i| Value::Float(i as f64)).collect()),
        (
            "y".into(),
            (0..5).map(|i| Value::Float(i as f64 * 10.0)).collect(),
        ),
    ]
}

fn red() -> ElementText {
    ElementText {
        color: (200, 0, 0),
        size: 14.0,
        ..theme_gray().axis_text_x.clone()
    }
}

fn svg(p: GGPlot) -> String {
    p.render_svg_native_with_size(400, 300).unwrap()
}

#[test]
fn top_x_axis_uses_axis_text_x_top() {
    let base = || {
        GGPlot::new(data())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .scale_x_continuous(ScaleContinuous::new().with_position_opposite())
    };
    let plain = svg(base());
    assert!(!plain.contains("#C80000"));
    let mut t = theme_gray();
    t.axis_text_x_top = Some(red());
    let s = svg(base().theme(t.clone()));
    assert!(s.contains("fill=\"#C80000\""), "top labels recoloured");
    // A bottom axis ignores it.
    let bottom = svg(GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .theme(t));
    assert!(!bottom.contains("#C80000"));
}

#[test]
fn secondary_y_axis_uses_axis_text_y_right() {
    let mut t = theme_gray();
    t.axis_text_y_right = Some(red());
    let s = svg(GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_y_continuous(ScaleContinuous::new().with_sec_axis(SecAxis::new(|v| v * 2.0)))
        .theme(t));
    // Only the right-hand labels are red: as many red labels as left ones.
    let red_n = s.matches("fill=\"#C80000\"").count();
    assert!(red_n >= 3, "{red_n}");
    let left_n = s.matches(">20<").count();
    assert!(left_n >= 1);
}

#[test]
fn theme_test_is_bw_without_grid() {
    let t = theme_test();
    let bw = theme_bw();
    assert!(!t.panel_grid_major.visible && !t.panel_grid_minor.visible);
    assert!(t.panel_border.visible);
    assert_eq!(t.panel_background.fill, bw.panel_background.fill);
    let s = svg(GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .theme_test());
    assert!(s.contains("<circle"));
}

#[test]
fn margin_auto_and_part() {
    let m = |t, r, b, l| Margin {
        top: t,
        right: r,
        bottom: b,
        left: l,
    };
    let eq = |a: Margin, b: Margin| {
        assert_eq!(
            (a.top, a.right, a.bottom, a.left),
            (b.top, b.right, b.bottom, b.left)
        )
    };
    eq(Margin::auto(&[5.0]), m(5.0, 5.0, 5.0, 5.0));
    eq(Margin::auto(&[1.0, 2.0]), m(1.0, 2.0, 1.0, 2.0));
    eq(Margin::auto(&[1.0, 2.0, 3.0]), m(1.0, 2.0, 3.0, 2.0));
    eq(Margin::auto(&[1.0, 2.0, 3.0, 4.0]), m(1.0, 2.0, 3.0, 4.0));
    eq(Margin::auto(&[]), m(0.0, 0.0, 0.0, 0.0));
    eq(
        Margin::auto(&[1.0]).part(None, Some(9.0), None, None),
        m(1.0, 9.0, 1.0, 1.0),
    );
}
