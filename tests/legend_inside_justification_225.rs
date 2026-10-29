//! #225: `LegendPosition::Inside(fx, fy)` used to always anchor the
//! legend's top-left corner at the panel's `(fx, fy)` point, growing
//! down-and-right from there regardless of how close to an edge `(fx,
//! fy)` was. A legend tall/wide enough — a continuous colourbar's fixed
//! 8-key-height bar, or a discrete legend with enough levels — routinely
//! ran off the canvas near `fy = 0` ("near the bottom"), since nothing
//! ever grew it *upward* to stay on-panel.
//!
//! Fixed to match ggplot2: `legend.justification.inside` defaults to
//! `legend.position.inside`, i.e. the legend's own fractional point `(fx,
//! fy)` — measured from its own bottom-left, the same `0..=1` convention
//! `Inside`'s `(fx, fy)` already uses — lands exactly on the panel's
//! `(fx, fy)` point. `fx=0`/`fy=0` anchors the legend's own left/bottom
//! edge; `fx=1`/`fy=1` anchors its right/top edge; `0.5` centers it. A
//! canvas-level clamp is the last-resort safety net for a legend taller
//! or wider than the panel itself.

use ggplot_rs::prelude::*;

fn floats(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&f| Value::Float(f)).collect()
}
fn strs(v: &[&str]) -> Vec<Value> {
    v.iter().map(|s| Value::Str(s.to_string())).collect()
}
fn frame(cols: Vec<(&str, Vec<Value>)>) -> Vec<(String, Vec<Value>)> {
    cols.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

/// `plot_area` as `[x, y, width, height]`, from the root `data-plot` attribute.
fn data_plot_rect(svg: &str) -> [f64; 4] {
    let start = svg.find("data-plot=\"").expect("data-plot attr") + "data-plot=\"".len();
    let rest = &svg[start..];
    let end = rest.find('"').expect("closing quote");
    let mut it = rest[..end]
        .split_whitespace()
        .map(|s| s.parse::<f64>().unwrap());
    [
        it.next().unwrap(),
        it.next().unwrap(),
        it.next().unwrap(),
        it.next().unwrap(),
    ]
}

/// The `(x, y)` of the first matching `<rect ... width="W" height="H" .../>`.
fn first_rect_xy(svg: &str, width: &str, height: &str) -> (f64, f64) {
    for (start, _) in svg.match_indices("<rect ") {
        let end = svg[start..].find("/>").unwrap() + start;
        let tag = &svg[start..end];
        if tag.contains(&format!("width=\"{width}\""))
            && tag.contains(&format!("height=\"{height}\""))
        {
            let x_start = tag.find("x=\"").unwrap() + 3;
            let x_end = tag[x_start..].find('"').unwrap() + x_start;
            let y_start = tag.find("y=\"").unwrap() + 3;
            let y_end = tag[y_start..].find('"').unwrap() + y_start;
            return (
                tag[x_start..x_end].parse().unwrap(),
                tag[y_start..y_end].parse().unwrap(),
            );
        }
    }
    panic!("no <rect width=\"{width}\" height=\"{height}\"> in {svg}");
}

/// Every matching `<rect ... width="W" .../>`'s `(x, y)`, for computing a
/// bounding box across all of them (e.g. every gradient-bar slice).
fn all_rects_xy(svg: &str, width: &str) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for (start, _) in svg.match_indices("<rect ") {
        let end = svg[start..].find("/>").unwrap() + start;
        let tag = &svg[start..end];
        if tag.contains(&format!("width=\"{width}\"")) {
            let x_start = tag.find("x=\"").unwrap() + 3;
            let x_end = tag[x_start..].find('"').unwrap() + x_start;
            let y_start = tag.find("y=\"").unwrap() + 3;
            let y_end = tag[y_start..].find('"').unwrap() + y_start;
            out.push((
                tag[x_start..x_end].parse().unwrap(),
                tag[y_start..y_end].parse().unwrap(),
            ));
        }
    }
    out
}

/// This theme's single-column legend width estimate
/// (`estimate_legend_box`'s formula — key width + spacing + 6 text-sizes).
fn single_col_width(theme: &Theme) -> f64 {
    theme.legend_key_width + theme.legend_spacing + theme.legend_text.size * 6.0
}

fn ten_level_discrete_data() -> Vec<(String, Vec<Value>)> {
    let n = 10;
    frame(vec![
        ("x", floats(&(0..n).map(|i| i as f64).collect::<Vec<_>>())),
        ("y", floats(&(0..n).map(|i| i as f64).collect::<Vec<_>>())),
        (
            "g",
            strs(
                &(0..n)
                    .map(|i| format!("Group {i:02}"))
                    .collect::<Vec<_>>()
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
            ),
        ),
    ])
}

fn continuous_data() -> Vec<(String, Vec<Value>)> {
    frame(vec![
        ("x", floats(&[1.0, 2.0, 3.0])),
        ("y", floats(&[1.0, 2.0, 3.0])),
        ("z", floats(&[0.0, 50.0, 100.0])),
    ])
}

fn discrete_inside_plot(fx: f64, fy: f64) -> GGPlot {
    GGPlot::new(ten_level_discrete_data())
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
        .legend_position(LegendPosition::Inside(fx, fy))
}

fn continuous_inside_plot(fx: f64, fy: f64) -> GGPlot {
    GGPlot::new(continuous_data())
        .aes(Aes::new().x("x").y("y").color("z"))
        .geom_point()
        .scale_color_gradient(
            ggplot_rs::scale::color::RGBAColor::new(0, 0, 255),
            ggplot_rs::scale::color::RGBAColor::new(255, 0, 0),
        )
        .legend_position(LegendPosition::Inside(fx, fy))
}

/// A canvas generous enough that neither legend ever needs the
/// canvas-level safety-net clamp — isolates the justification math
/// itself from that separate concern.
const W: u32 = 1000;
const H: u32 = 1000;

#[test]
fn discrete_legend_justification_point_matches_the_panel_at_several_positions() {
    for (fx, fy) in [
        (0.05, 0.05),
        (0.95, 0.95),
        (0.5, 0.5),
        (0.0, 0.0),
        (1.0, 1.0),
    ] {
        let svg = discrete_inside_plot(fx, fy)
            .render_svg_native_with_size(W, H)
            .unwrap();
        let theme = Theme::default();
        let legend_w = single_col_width(&theme);
        let legend_h = 10.0 * theme.legend_key_height;
        let panel = data_plot_rect(&svg);
        // #377: justified within the panel inset by `legend_margin`.
        let m = &theme.legend_margin;
        let expected_x = panel[0] + m.left + fx * (panel[2] - m.left - m.right - legend_w);
        let expected_y = panel[1] + m.top + (1.0 - fy) * (panel[3] - m.top - m.bottom - legend_h);
        let (first_x, first_y) = first_rect_xy(&svg, "12.00", "12.00");
        assert!(
            (first_x - expected_x).abs() < 0.5,
            "fx={fx} fy={fy}: x={first_x} expected {expected_x}"
        );
        assert!(
            (first_y - expected_y).abs() < 0.5,
            "fx={fx} fy={fy}: y={first_y} expected {expected_y}"
        );
    }
}

#[test]
fn continuous_legend_justification_point_matches_the_panel_at_several_positions() {
    for (fx, fy) in [
        (0.05, 0.05),
        (0.95, 0.95),
        (0.5, 0.5),
        (0.0, 0.0),
        (1.0, 1.0),
    ] {
        let svg = continuous_inside_plot(fx, fy)
            .render_svg_native_with_size(W, H)
            .unwrap();
        let theme = Theme::default();
        let legend_w = single_col_width(&theme);
        // The bar plus its bottom label's lower half (#377).
        let legend_h = theme.legend_key_height * 8.0 + theme.legend_text.size / 2.0;
        let panel = data_plot_rect(&svg);
        // #377: justified within the panel inset by `legend_margin`.
        let m = &theme.legend_margin;
        let expected_x = panel[0] + m.left + fx * (panel[2] - m.left - m.right - legend_w);
        let expected_y = panel[1] + m.top + (1.0 - fy) * (panel[3] - m.top - m.bottom - legend_h);
        // The first bar slice (i=0, the top of the bar) — a 12px-wide
        // rect whose height is the thin per-slice height, not 12.00.
        let (first_x, first_y) = all_rects_xy(&svg, "12.00")
            .into_iter()
            .find(|(_, y)| *y >= panel[1] - 50.0) // skip any unrelated 12px-wide rect, if ever
            .expect("a bar slice rect");
        assert!(
            (first_x - expected_x).abs() < 0.5,
            "fx={fx} fy={fy}: x={first_x} expected {expected_x}"
        );
        assert!(
            (first_y - expected_y).abs() < 0.5,
            "fx={fx} fy={fy}: y={first_y} expected {expected_y}"
        );
    }
}

#[test]
fn a_tall_continuous_legend_near_the_bottom_stays_on_the_canvas() {
    // The exact repro #225 described: a colourbar inside near (0.05,
    // 0.05) — "near the bottom" — used to render entirely below the
    // canvas. A deliberately short canvas keeps the panel shorter than
    // the legend's fixed 144px bar, forcing the safety-net clamp too.
    const SMALL_H: u32 = 220;
    let svg = continuous_inside_plot(0.05, 0.05)
        .render_svg_native_with_size(400, SMALL_H)
        .unwrap();
    let slices = all_rects_xy(&svg, "12.00");
    assert!(!slices.is_empty(), "{svg}");
    let min_y = slices.iter().map(|(_, y)| *y).fold(f64::INFINITY, f64::min);
    let max_y = slices
        .iter()
        .map(|(_, y)| *y)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(
        min_y >= 0.0 && max_y <= SMALL_H as f64,
        "bar should stay on the {SMALL_H}px canvas: min_y={min_y} max_y={max_y}"
    );
}

#[test]
fn a_tall_discrete_legend_near_the_bottom_stays_on_the_canvas() {
    // Same repro, but for a plain 10-level discrete legend — #225's bug
    // wasn't actually continuous-specific, just more visible there.
    const SMALL_H: u32 = 220;
    let svg = discrete_inside_plot(0.05, 0.05)
        .render_svg_native_with_size(400, SMALL_H)
        .unwrap();
    let keys = all_rects_xy(&svg, "12.00");
    let keys: Vec<(f64, f64)> = keys
        .into_iter()
        .filter(|(_, y)| *y >= 0.0) // the 12x12 swatches only
        .collect();
    assert!(!keys.is_empty(), "{svg}");
    let min_y = keys.iter().map(|(_, y)| *y).fold(f64::INFINITY, f64::min);
    let max_y = keys
        .iter()
        .map(|(_, y)| *y)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(
        min_y >= 0.0 && max_y <= SMALL_H as f64,
        "legend keys should stay on the {SMALL_H}px canvas: min_y={min_y} max_y={max_y}"
    );
}

#[test]
fn discrete_legend_at_0_0_anchors_its_own_bottom_left_on_the_panel() {
    // fx=0, fy=0: the legend's own bottom-left corner lands on the
    // panel's bottom-left corner — it must grow *upward* from there, not
    // downward off the canvas (#225's bug).
    let svg = discrete_inside_plot(0.0, 0.0)
        .render_svg_native_with_size(W, H)
        .unwrap();
    let panel = data_plot_rect(&svg);
    let theme = Theme::default();
    let legend_h = 10.0 * theme.legend_key_height;
    let (_, first_y) = first_rect_xy(&svg, "12.00", "12.00");
    // The legend's bottom edge (first_y + legend_h) must sit at the
    // panel's bottom edge, inset by `legend_margin.bottom` (#377).
    assert!(
        (first_y + legend_h - (panel[1] + panel[3] - theme.legend_margin.bottom)).abs() < 0.5,
        "legend bottom edge should sit at the panel's bottom edge: \
         first_y={first_y} legend_h={legend_h} panel_bottom={}",
        panel[1] + panel[3]
    );
}

#[test]
fn discrete_legend_at_1_1_anchors_its_own_top_right_on_the_panel() {
    // fx=1, fy=1: the legend's own top-right corner lands on the panel's
    // top-right corner.
    let svg = discrete_inside_plot(1.0, 1.0)
        .render_svg_native_with_size(W, H)
        .unwrap();
    let panel = data_plot_rect(&svg);
    let theme = Theme::default();
    let legend_w = single_col_width(&theme);
    let (first_x, first_y) = first_rect_xy(&svg, "12.00", "12.00");
    let m = &theme.legend_margin;
    assert!(
        (first_x + legend_w - (panel[0] + panel[2] - m.right)).abs() < 0.5,
        "legend right edge should sit at the panel's right edge: \
         first_x={first_x} legend_w={legend_w} panel_right={}",
        panel[0] + panel[2]
    );
    assert!(
        (first_y - (panel[1] + m.top)).abs() < 0.5,
        "legend top edge should sit at the panel's top edge: \
         first_y={first_y} panel_y={}",
        panel[1]
    );
}

#[test]
fn a_legend_taller_than_the_whole_canvas_still_left_top_aligns_without_panicking() {
    // The degenerate end of the safety net: a legend (10 levels, 180px
    // tall) taller than even the full canvas, not just the panel. Falls
    // back to the old unconditional top-left alignment (left/top-aligned
    // at the canvas edge) rather than a `pos.clamp` that could panic on
    // an inverted (hi < lo) range.
    let svg = discrete_inside_plot(0.5, 0.5)
        .render_svg_native_with_size(400, 60)
        .unwrap();
    let (x, y) = first_rect_xy(&svg, "12.00", "12.00");
    assert!(x >= 0.0 && y >= 0.0, "{svg}");
}
