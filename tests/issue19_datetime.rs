//! Issue #19 — date/time axis: calendar breaks + strftime labels.

use ggplot_rs::prelude::*;

/// One point per month across 2021 (epoch seconds at the 1st).
fn timeseries() -> Vec<(String, Vec<Value>)> {
    // 2021-01-01 UTC = 1609459200; add ~30.4-day steps.
    let base = 1_609_459_200.0;
    let x: Vec<Value> = (0..12)
        .map(|m| Value::Float(base + m as f64 * 2_629_800.0))
        .collect();
    let y: Vec<Value> = (0..12).map(|m| Value::Float((m as f64).sin())).collect();
    vec![("t".to_string(), x), ("y".to_string(), y)]
}

#[test]
fn monthly_breaks_with_labels_render() {
    let svg = GGPlot::new(timeseries())
        .aes(Aes::new().x("t").y("y"))
        .geom_line()
        .scale_x_datetime(
            ScaleDateTime::new()
                .with_date_breaks("2 months")
                .with_date_labels("%b %Y"),
        )
        .render_svg()
        .expect("render");
    // Month-name labels should appear on the axis.
    assert!(
        svg.contains("Mar") || svg.contains("May") || svg.contains("2021"),
        "expected date labels in svg"
    );
}

#[test]
fn yearly_and_iso_labels_render() {
    let svg = GGPlot::new(timeseries())
        .aes(Aes::new().x("t").y("y"))
        .geom_point()
        .scale_x_datetime(
            ScaleDateTime::new()
                .with_date_breaks("1 year")
                .with_date_labels("%Y-%m-%d"),
        )
        .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}
