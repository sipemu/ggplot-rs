//! End-to-end: geom_point + stat_cor() renders a correlation label (ggpubr).
#![cfg(feature = "ggpubr")]

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;

fn scatter_data() -> Vec<(String, Vec<Value>)> {
    let x: Vec<Value> = (0..25).map(|i| Value::Float(i as f64)).collect();
    // Strong positive linear relationship with a little jitter.
    let y: Vec<Value> = (0..25)
        .map(|i| Value::Float(2.0 * i as f64 + 3.0 + ((i * 7 % 5) as f64 - 2.0)))
        .collect();
    vec![("x".to_string(), x), ("y".to_string(), y)]
}

#[test]
fn stat_cor_renders_pearson_label() {
    let svg = GGPlot::new(scatter_data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .stat_cor()
        .render_svg()
        .expect("render");
    // Near-perfect linear data → R rounds to 1.00; label carries a p-value.
    assert!(
        svg.contains("R = 1.00") || svg.contains("R = 0.9"),
        "expected a high Pearson R label"
    );
    // Very strong correlation → p hits the "< 2.2e-16" floor.
    assert!(
        svg.contains("p =") || svg.contains("p &lt;") || svg.contains("p <"),
        "expected a p-value in the label"
    );
}

#[test]
fn stat_cor_with_spearman_and_fixed_position() {
    let stat = StatCor::new(CorMethod::Spearman).label_pos(1.0, 50.0);
    let svg = GGPlot::new(scatter_data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .stat_cor_with(stat)
        .render_svg()
        .expect("render");
    assert!(svg.contains("R ="), "spearman label should render");
}
