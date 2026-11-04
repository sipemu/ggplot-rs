//! #409: a secondary x axis along the top (`sec.axis` on x) and
//! `dup_axis()` (`SecAxis::dup`).

use ggplot_rs::prelude::*;
use ggplot_rs::scale::sec_axis::SecAxis;

fn plot() -> GGPlot {
    GGPlot::new(vec![
        (
            "x".to_string(),
            (0..6).map(|i| Value::Float(i as f64 * 10.0)).collect(),
        ),
        (
            "y".to_string(),
            (0..6).map(|i| Value::Float(i as f64)).collect(),
        ),
    ])
    .aes(Aes::new().x("x").y("y"))
    .geom_point()
    .title("T")
}

fn svg(p: GGPlot) -> String {
    p.render_svg_native_with_size(500, 350).unwrap()
}

fn text_y(s: &str, label: &str) -> Vec<f64> {
    s.split("<text")
        .skip(1)
        .filter(|t| {
            t.split('>')
                .nth(1)
                .is_some_and(|c| c.starts_with(&format!("{label}<")))
        })
        .map(|t| {
            let a = t.find(" y=\"").unwrap() + 4;
            t[a..].split('"').next().unwrap().parse().unwrap()
        })
        .collect()
}

#[test]
fn sec_x_axis_draws_transformed_labels_on_top() {
    let s = svg(plot().scale_x_continuous(
        ScaleContinuous::new().with_sec_axis(SecAxis::new(|c| c * 1.8 + 32.0).with_name("F")),
    ));
    // 20 °C → 68 °F on top; the primary 20 stays at the bottom.
    let top = text_y(&s, "68");
    let bottom = text_y(&s, "20");
    assert!(!top.is_empty() && !bottom.is_empty(), "{s}");
    assert!(top[0] < bottom.iter().cloned().fold(f64::MAX, f64::min));
    // Title, then the axis title, then the labels — no overlap.
    let (t, f) = (text_y(&s, "T")[0], text_y(&s, "F")[0]);
    assert!(t < f && f < top[0], "{t} {f} {top:?}");
}

#[test]
fn dup_axis_repeats_the_labels_and_shrinks_the_panel() {
    let base = svg(plot());
    let dup = svg(plot().scale_x_continuous(ScaleContinuous::new().with_sec_axis(SecAxis::dup())));
    assert_eq!(text_y(&dup, "30").len(), 2 * text_y(&base, "30").len());
    let panel_h = |s: &str| -> f64 {
        let r = s.split("<rect").nth(2).unwrap();
        let a = r.find(" height=\"").unwrap() + 9;
        r[a..].split('"').next().unwrap().parse().unwrap()
    };
    assert!(panel_h(&dup) < panel_h(&base));
}
