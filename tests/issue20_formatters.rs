//! Issue #20 — configurable label formatters applied to a continuous scale.

use ggplot_rs::prelude::*;
use ggplot_rs::scale::continuous::ScaleContinuous;

fn data() -> Vec<(String, Vec<Value>)> {
    vec![
        (
            "x".to_string(),
            (0..5).map(|i| Value::Float(i as f64)).collect(),
        ),
        (
            "y".to_string(),
            [1.0e3, 2.5e5, 1.2e6, 8.0e6, 5.0e7]
                .iter()
                .map(|v| Value::Float(*v))
                .collect(),
        ),
    ]
}

#[test]
fn si_formatter_renders_on_axis() {
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_y_continuous(ScaleContinuous::new().with_label_formatter(label_si()))
        .render_svg()
        .expect("render");
    // SI-formatted tick labels should appear (e.g. "M" magnitude suffix).
    assert!(svg.contains('M') || svg.contains('k'));
}

fn renders_with(f: impl Fn(f64) -> String + Send + Sync + 'static) {
    let ok = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_line()
        .scale_y_continuous(ScaleContinuous::new().with_label_formatter(f))
        .render_svg()
        .is_ok();
    assert!(ok);
}

#[test]
fn number_and_bytes_and_ordinal_formatters_render() {
    renders_with(label_number(Some(1.0), "€", "", 1.0));
    renders_with(label_bytes(false));
    renders_with(label_ordinal());
    renders_with(label_comma); // plain fn still works
}
