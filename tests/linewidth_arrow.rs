//! B1-4 acceptance: the `linewidth` aesthetic, `ScaleLinewidthContinuous`
//! (range, transform, limits) and its legend.

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;

const VMAX: f64 = 10.0;

fn data() -> Vec<(String, Vec<Value>)> {
    let vs = [0.0, 2.5, 5.0, 7.5, 10.0];
    vec![
        ("x".into(), vs.iter().map(|_| Value::Float(0.0)).collect()),
        (
            "y".into(),
            (0..vs.len()).map(|i| Value::Float(i as f64)).collect(),
        ),
        (
            "xend".into(),
            vs.iter().map(|_| Value::Float(1.0)).collect(),
        ),
        (
            "yend".into(),
            (0..vs.len()).map(|i| Value::Float(i as f64)).collect(),
        ),
        ("v".into(), vs.iter().map(|&v| Value::Float(v)).collect()),
    ]
}

fn plot() -> GGPlot {
    GGPlot::new(data())
        .aes(
            Aes::new()
                .x("x")
                .y("y")
                .xend("xend")
                .yend("yend")
                .linewidth("v"),
        )
        .geom_segment()
        .scale_linewidth(
            ScaleLinewidthContinuous::new()
                .with_range(0.75, 8.0)
                .with_transform(ScaleTransform::Sqrt)
                .with_limits(0.0, VMAX),
        )
}

fn stroke_widths(svg: &str) -> Vec<f64> {
    svg.split("stroke-width=\"")
        .skip(1)
        .filter_map(|s| s.split('"').next())
        .filter_map(|s| s.parse::<f64>().ok())
        .collect()
}

#[test]
fn linewidth_maps_through_range_transform_and_limits() {
    let svg = plot().render_svg_native_with_size(300, 300).unwrap();
    assert!(!svg.contains("NaN"));

    let vs = [0.0, 2.5, 5.0, 7.5, 10.0];
    let expected: Vec<f64> = vs
        .iter()
        .map(|&v| 0.75 + 7.25 * (v / VMAX).sqrt())
        .collect();
    let widths = stroke_widths(&svg);
    for e in &expected {
        let rounded = format!("{e:.2}");
        assert!(
            widths
                .iter()
                .any(|w| (w - rounded.parse::<f64>().unwrap()).abs() < 1e-6),
            "expected stroke-width {rounded} (from v in {vs:?}) not found in {widths:?}"
        );
    }
}

#[test]
fn legend_has_n_keys_with_increasing_widths() {
    let breaks = vec![0.0, 2.5, 5.0, 7.5, 10.0];
    let with_legend = plot()
        .scale_linewidth(
            ScaleLinewidthContinuous::new()
                .with_range(0.75, 8.0)
                .with_transform(ScaleTransform::Sqrt)
                .with_limits(0.0, VMAX)
                .with_breaks(breaks.clone()),
        )
        .render_svg_native_with_size(400, 400)
        .unwrap();
    let without_legend = plot()
        .scale_linewidth(
            ScaleLinewidthContinuous::new()
                .with_range(0.75, 8.0)
                .with_transform(ScaleTransform::Sqrt)
                .with_limits(0.0, VMAX)
                .with_breaks(breaks.clone()),
        )
        .show_legend(false)
        .render_svg_native_with_size(400, 400)
        .unwrap();

    // The legend markup is whatever's appended once the shared prefix ends.
    let common = with_legend
        .bytes()
        .zip(without_legend.bytes())
        .take_while(|(a, b)| a == b)
        .count();
    let legend_markup = &with_legend[common..];
    assert!(
        !legend_markup.is_empty(),
        "expected extra markup for the legend"
    );

    let expected_widths: Vec<f64> = breaks
        .iter()
        .map(|&v| 0.75 + 7.25 * (v / VMAX).sqrt())
        .collect();
    let legend_widths: Vec<f64> = stroke_widths(legend_markup)
        .into_iter()
        .filter(|w| expected_widths.iter().any(|e| (w - e).abs() < 0.01))
        .collect();

    assert_eq!(
        legend_widths.len(),
        breaks.len(),
        "expected one legend key per break: {legend_widths:?}"
    );
    for i in 1..legend_widths.len() {
        assert!(
            legend_widths[i] > legend_widths[i - 1],
            "legend widths should increase: {legend_widths:?}"
        );
    }
}

#[test]
fn show_legend_false_hides_the_linewidth_legend() {
    let with_legend = plot().render_svg_native_with_size(400, 400).unwrap();
    let without_legend = plot()
        .show_legend(false)
        .render_svg_native_with_size(400, 400)
        .unwrap();
    assert!(with_legend.len() > without_legend.len());
}
