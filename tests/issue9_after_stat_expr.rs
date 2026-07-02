//! Issue #9 — computed after_stat aesthetics (aggregate expressions).
use ggplot_rs::prelude::*;

#[test]
fn histogram_proportion_after_stat() {
    // Bar heights should sum to 1 when y = count / sum(count).
    let x: Vec<Value> = (0..40).map(|i| Value::Float((i % 8) as f64)).collect();
    let built = GGPlot::new(vec![("x".to_string(), x)])
        .aes(Aes::new().x("x").after_stat_y("count / sum(count)"))
        .geom_histogram_with(GeomHistogram::default())
        .build();
    let y: Vec<f64> = built.layers[0]
        .data
        .column("y")
        .unwrap()
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();
    assert!(!y.is_empty());
    let total: f64 = y.iter().sum();
    assert!(
        (total - 1.0).abs() < 1e-6,
        "proportions should sum to 1, got {total}"
    );
}

#[test]
fn bar_count_normalized_renders() {
    let g: Vec<Value> = (0..30)
        .map(|i| Value::Str(["a", "b", "c"][i % 3].into()))
        .collect();
    let svg = GGPlot::new(vec![("g".to_string(), g)])
        .aes(Aes::new().x("g").after_stat_y("count / max(count)"))
        .geom_bar()
        .render_svg();
    assert!(svg.is_ok(), "{svg:?}");
}
