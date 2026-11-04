//! #409 / #408: `guide_bins()` — a binned scale's default legend, labelled
//! at the bin edges, with `show.limits` adding the outer two.

use ggplot_rs::prelude::*;

fn data() -> Vec<(String, Vec<Value>)> {
    let x: Vec<Value> = (1..=100).map(|i| Value::Float(i as f64)).collect();
    let v: Vec<Value> = (0..100)
        .map(|i| Value::Float(3.0 + 94.0 * i as f64 / 99.0))
        .collect();
    vec![("x".into(), x.clone()), ("y".into(), x), ("v".into(), v)]
}

fn base() -> GGPlot {
    GGPlot::new(data())
        .aes(Aes::new().x("x").y("y").size("v"))
        .geom_point()
        .scale_size_binned(5)
}

fn render(p: GGPlot) -> String {
    p.render_svg_native_with_size(400, 400).unwrap()
}

/// Legend label texts (drawn in black, unlike the grey axis labels).
fn legend_labels(svg: &str) -> Vec<String> {
    svg.split("<text")
        .skip(1)
        .filter_map(|t| {
            let (tag, rest) = t.split_once('>')?;
            tag.contains(r##"fill="#000000""##)
                .then(|| rest.split_once("</text>").map(|(l, _)| l.to_string()))?
        })
        .collect()
}

#[test]
fn bins_label_the_inner_edges_by_default() {
    // R: scale_size_binned() over 3..97 → bins 0–25–50–75–100.
    let labels = legend_labels(&render(base()));
    assert_eq!(labels, ["25", "50", "75"]);
}

#[test]
fn show_limits_adds_the_outer_edges() {
    let svg = render(base().guide_bins(Aesthetic::Size, GuideBins::new().with_show_limits(true)));
    assert_eq!(legend_labels(&svg), ["0", "25", "50", "75", "100"]);
    // Reversed keys reverse the edge labels too.
    let svg = render(
        base()
            .guide_bins(Aesthetic::Size, GuideBins::new().with_show_limits(true))
            .guide_reverse(Aesthetic::Size, true),
    );
    assert_eq!(legend_labels(&svg), ["100", "75", "50", "25", "0"]);
}

#[test]
fn axis_draws_a_line_and_a_tick_per_edge() {
    let lines = |svg: &str| svg.matches("<polyline").count();
    let with_axis = render(base());
    let without = render(base().guide_bins(Aesthetic::Size, GuideBins::new().with_axis(false)));
    // One axis line plus one tick at each of the 5 edges.
    assert_eq!(lines(&with_axis) - lines(&without), 6);
}

#[test]
fn legend_keys_restore_one_range_label_per_bin() {
    let labels = legend_labels(&render(base().guide_legend_keys(Aesthetic::Size)));
    assert_eq!(labels, ["0–25", "25–50", "50–75", "75–100"]);
}

#[test]
fn horizontal_bins_label_below_the_keys() {
    let svg = render(base().legend_position(LegendPosition::Bottom));
    assert_eq!(legend_labels(&svg), ["25", "50", "75"]);
    assert!(svg.contains(r#"text-anchor="middle""#));
}
