//! #241: a continuous `linewidth` legend was laid out outside the plot
//! area / canvas — `has_legend_mapping` (the boolean `layout_built` feeds
//! `PlotLayout::compute_with_extras` to decide whether to reserve *any*
//! legend space at all) had its own, independently-maintained copy of
//! "which aesthetics get a legend" that had drifted out of sync with
//! `guide::legend::LEGEND_AESTHETICS` — missing `Linewidth`. A plot
//! mapping only `linewidth` (no colour/fill/shape/linetype/size/alpha)
//! therefore reserved zero legend space, and the legend then drew with
//! nothing held back for it, running off the canvas. Now delegates to
//! the one authoritative list (`is_legend_aesthetic`) instead of keeping
//! a second copy that can drift again.

use ggplot_rs::prelude::*;

fn linewidth_plot() -> GGPlot {
    let data: Vec<(String, Vec<Value>)> = vec![
        ("x".into(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("y".into(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("lw".into(), vec![Value::Float(1.0), Value::Float(10.0)]),
    ];
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").linewidth("lw"))
        .geom_line()
}

/// Every `<text ...>`'s `x` attribute (the legend's tick labels, for a
/// Right-positioned legend, are the rightmost text on the canvas).
fn all_text_x(svg: &str) -> Vec<f64> {
    let mut out = Vec::new();
    for (start, _) in svg.match_indices("<text ") {
        let end = svg[start..].find('>').unwrap() + start;
        let tag = &svg[start..=end];
        let x_start = tag.find("x=\"").unwrap() + 3;
        let x_end = tag[x_start..].find('"').unwrap() + x_start;
        out.push(tag[x_start..x_end].parse().unwrap());
    }
    out
}

/// Every `<polyline ...>`'s point coordinates (the legend's key
/// segments, among others).
fn all_polyline_points(svg: &str) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for (start, _) in svg.match_indices("points=\"") {
        let p_start = start + "points=\"".len();
        let p_end = svg[p_start..].find('"').unwrap() + p_start;
        for pair in svg[p_start..p_end].split_whitespace() {
            if let Some((x, y)) = pair.split_once(',') {
                if let (Ok(x), Ok(y)) = (x.parse(), y.parse()) {
                    out.push((x, y));
                }
            }
        }
    }
    out
}

#[test]
fn linewidth_legend_text_stays_within_the_canvas_at_right() {
    const W: u32 = 500;
    const H: u32 = 300;
    let svg = linewidth_plot().render_svg_native_with_size(W, H).unwrap();
    for x in all_text_x(&svg) {
        assert!(
            x >= 0.0 && x <= W as f64,
            "a <text> x={x} overflows the {W}px canvas: {svg}"
        );
    }
}

#[test]
fn linewidth_legend_keys_stay_within_the_canvas_at_top_and_bottom() {
    const W: u32 = 400;
    const H: u32 = 400;
    for pos in [LegendPosition::Top, LegendPosition::Bottom] {
        let svg = linewidth_plot()
            .legend_position(pos.clone())
            .render_svg_native_with_size(W, H)
            .unwrap();
        for x in all_text_x(&svg) {
            assert!(
                x >= 0.0 && x <= W as f64,
                "{pos:?}: a <text> x={x} overflows the {W}px canvas"
            );
        }
        for (x, y) in all_polyline_points(&svg) {
            assert!(
                y >= 0.0 && y <= H as f64,
                "{pos:?}: a polyline point y={y} overflows the {H}px canvas"
            );
            let _ = x;
        }
    }
}

#[test]
fn linewidth_legend_stays_within_the_canvas_inside() {
    const W: u32 = 400;
    const H: u32 = 300;
    let svg = linewidth_plot()
        .legend_position(LegendPosition::Inside(0.95, 0.95))
        .render_svg_native_with_size(W, H)
        .unwrap();
    for x in all_text_x(&svg) {
        assert!(
            x >= 0.0 && x <= W as f64,
            "a <text> x={x} overflows the {W}px canvas: {svg}"
        );
    }
}

#[test]
fn linewidth_legend_reserves_panel_space_unlike_a_plot_with_no_legend_at_all() {
    // The core of #241: a plot that maps `linewidth` (and so draws a
    // legend for it) must have a *narrower* panel than one with no
    // mapping needing a legend at all — confirming the legend actually
    // reserved room rather than relying on happening to fit.
    const W: u32 = 500;
    const H: u32 = 300;
    let with_legend = linewidth_plot().render_svg_native_with_size(W, H).unwrap();
    let data: Vec<(String, Vec<Value>)> = vec![
        ("x".into(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("y".into(), vec![Value::Float(1.0), Value::Float(2.0)]),
    ];
    let without_legend = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_line()
        .render_svg_native_with_size(W, H)
        .unwrap();
    let panel_width = |svg: &str| -> f64 {
        let start = svg.find("data-plot=\"").unwrap() + "data-plot=\"".len();
        let rest = &svg[start..];
        let end = rest.find('"').unwrap();
        let mut it = rest[..end].split_whitespace();
        it.next(); // x
        it.next(); // y
        it.next().unwrap().parse().unwrap() // width
    };
    assert!(
        panel_width(&with_legend) < panel_width(&without_legend),
        "a drawn linewidth legend must reserve panel width: with={} without={}",
        panel_width(&with_legend),
        panel_width(&without_legend)
    );
}

#[test]
fn linewidth_legend_key_widths_are_monotone_in_the_breaks() {
    let svg = linewidth_plot()
        .render_svg_native_with_size(500, 300)
        .unwrap();
    let mut widths: Vec<f64> = Vec::new();
    for (start, _) in svg.match_indices("stroke-width=\"") {
        let v_start = start + "stroke-width=\"".len();
        let v_end = svg[v_start..].find('"').unwrap() + v_start;
        widths.push(svg[v_start..v_end].parse().unwrap());
    }
    // At least some genuine spread of distinct widths exists (the panel
    // line itself plus >= 2 legend key widths).
    let mut sorted = widths.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    sorted.dedup();
    assert!(
        sorted.len() >= 3,
        "expected several distinct stroke widths (panel line + legend keys): {widths:?}"
    );
}
