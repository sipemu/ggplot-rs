//! sipemu/ggplot-rs#211: a continuous colourbar at `Top`/`Bottom` (or
//! `legend_direction(Horizontal)`) now draws wide × short — slices left
//! to right from the scale's min to max, ticks/labels below the bar —
//! instead of always drawing the tall × narrow bar meant for `Right`/
//! `Left`, which used to get clipped once moved under the panel. The
//! default (`Right`, vertical) must stay byte-identical.

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;
use ggplot_rs::theme::{presets::theme_dark, LegendDirection, LegendPosition};

fn floats(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&f| Value::Float(f)).collect()
}
fn frame(cols: Vec<(&str, Vec<Value>)>) -> Vec<(String, Vec<Value>)> {
    cols.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

fn fill_scatter() -> GGPlot {
    let n = 20;
    let data = frame(vec![
        ("x", floats(&(0..n).map(|i| i as f64).collect::<Vec<_>>())),
        (
            "y",
            floats(&(0..n).map(|i| (i % 5) as f64).collect::<Vec<_>>()),
        ),
        ("z", floats(&(0..n).map(|i| i as f64).collect::<Vec<_>>())),
    ]);
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").fill("z"))
        .geom_point()
}

fn render(plot: GGPlot, w: u32, h: u32) -> String {
    plot.render_svg_native_with_size(w, h)
        .expect("native svg render")
}

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

/// Every `<rect ... fill="#......" .../>` with a hex fill — the gradient
/// bar's 50 slices (plus the panel background/key swatches, filtered out
/// by the caller via a width/height bound where needed).
fn hex_fill_rects(svg: &str) -> Vec<(f64, f64, f64, f64)> {
    let mut out = Vec::new();
    for seg in svg.split("<rect ").skip(1) {
        let end = seg.find("/>").unwrap_or(seg.len());
        let tag = &seg[..end];
        if !tag.contains("fill=\"#") {
            continue;
        }
        let attr = |name: &str| -> f64 {
            let key = format!("{name}=\"");
            let i = tag.find(&key).unwrap() + key.len();
            tag[i..].split('"').next().unwrap().parse().unwrap()
        };
        out.push((attr("x"), attr("y"), attr("width"), attr("height")));
    }
    out
}

/// The gradient bar's own bounding box: min/max over its 50 slice rects
/// (identified as the ~50 narrow hex-fill rects sharing a width or height
/// close to `expected_slices`).
fn bar_bbox(svg: &str) -> (f64, f64, f64, f64) {
    let rects = hex_fill_rects(svg);
    assert!(rects.len() >= 50, "expected >=50 gradient slices: {svg}");
    // The 50 most recent hex-fill rects belong to the colourbar (points
    // drawn before it, in a scatter with few enough rows, are also
    // hex-filled — so take the LAST 50 found, matching draw order).
    let slices = &rects[rects.len() - 50..];
    let x0 = slices.iter().map(|r| r.0).fold(f64::MAX, f64::min);
    let y0 = slices.iter().map(|r| r.1).fold(f64::MAX, f64::min);
    let x1 = slices.iter().map(|r| r.0 + r.2).fold(f64::MIN, f64::max);
    let y1 = slices.iter().map(|r| r.1 + r.3).fold(f64::MIN, f64::max);
    (x0, y0, x1 - x0, y1 - y0)
}

#[test]
fn default_right_colourbar_stays_tall_and_narrow() {
    let svg = render(fill_scatter(), 400, 300);
    let (_, _, w, h) = bar_bbox(&svg);
    // legend_key_width=12 (narrow), legend_key_height*8.0=144 (tall).
    assert!((w - 12.0).abs() < 1.0, "width {w}: {svg}");
    assert!((h - 144.0).abs() < 2.0, "height {h}: {svg}");
}

#[test]
fn bottom_position_draws_the_bar_wide_and_short() {
    let svg = render(
        fill_scatter().legend_position(LegendPosition::Bottom),
        400,
        400,
    );
    let (_, _, w, h) = bar_bbox(&svg);
    // Swapped: length 144 (now horizontal), thickness 12.
    assert!((w - 144.0).abs() < 2.0, "width {w}: {svg}");
    assert!((h - 12.0).abs() < 1.0, "height {h}: {svg}");
}

#[test]
fn top_position_draws_the_bar_wide_and_short() {
    let svg = render(
        fill_scatter().legend_position(LegendPosition::Top),
        400,
        400,
    );
    let (_, _, w, h) = bar_bbox(&svg);
    assert!((w - 144.0).abs() < 2.0, "width {w}: {svg}");
    assert!((h - 12.0).abs() < 1.0, "height {h}: {svg}");
}

#[test]
fn legend_direction_horizontal_overrides_a_vertical_position() {
    // Right is vertical by default; an explicit `legend_direction`
    // overrides that (same override `legend_position`'s doc comment
    // already describes for the discrete case).
    let svg = render(
        fill_scatter()
            .legend_position(LegendPosition::Right)
            .legend_direction(LegendDirection::Horizontal),
        400,
        400,
    );
    let (_, _, w, h) = bar_bbox(&svg);
    assert!((w - 144.0).abs() < 2.0, "width {w}: {svg}");
    assert!((h - 12.0).abs() < 1.0, "height {h}: {svg}");
}

#[test]
fn bottom_colourbar_does_not_overlap_the_panel() {
    let svg = render(
        fill_scatter().legend_position(LegendPosition::Bottom),
        400,
        400,
    );
    let plot_area = data_plot_rect(&svg);
    let (_, bar_y, _, bar_h) = bar_bbox(&svg);
    let panel_bottom = plot_area[1] + plot_area[3];
    assert!(
        bar_y >= panel_bottom,
        "bar (y={bar_y}) overlaps the panel (bottom={panel_bottom}): {svg}"
    );
    // Also stays on-canvas (reserved band tall enough for title + bar +
    // ticks + one line of labels below, #211's actual fix).
    assert!(bar_y + bar_h <= 400.0, "bar spills off-canvas: {svg}");
}

#[test]
fn top_colourbar_does_not_overlap_the_panel() {
    let svg = render(
        fill_scatter().legend_position(LegendPosition::Top),
        400,
        400,
    );
    let plot_area = data_plot_rect(&svg);
    let (_, bar_y, _, bar_h) = bar_bbox(&svg);
    assert!(
        bar_y + bar_h <= plot_area[1],
        "bar (bottom={}) overlaps the panel (top={}): {svg}",
        bar_y + bar_h,
        plot_area[1]
    );
    assert!(bar_y >= 0.0, "bar starts off-canvas: {svg}");
}

#[test]
fn bottom_colourbar_works_in_a_dark_theme_too() {
    let svg = render(
        fill_scatter()
            .theme(theme_dark())
            .legend_position(LegendPosition::Bottom),
        400,
        400,
    );
    let plot_area = data_plot_rect(&svg);
    let (_, bar_y, w, h) = bar_bbox(&svg);
    assert!((w - 144.0).abs() < 2.0, "width {w}: {svg}");
    assert!((h - 12.0).abs() < 1.0, "height {h}: {svg}");
    assert!(
        bar_y >= plot_area[1] + plot_area[3],
        "bar overlaps the panel in theme_dark: {svg}"
    );
}

#[test]
fn bottom_colourbar_works_for_a_heatmap_shaped_continuous_fill() {
    let n = 9;
    let data = frame(vec![
        (
            "x",
            floats(&(0..n).map(|i| (i % 3) as f64).collect::<Vec<_>>()),
        ),
        (
            "y",
            floats(&(0..n).map(|i| (i / 3) as f64).collect::<Vec<_>>()),
        ),
        ("z", floats(&(0..n).map(|i| i as f64).collect::<Vec<_>>())),
    ]);
    let svg = render(
        GGPlot::new(data)
            .aes(Aes::new().x("x").y("y").fill("z"))
            .geom_tile()
            .legend_position(LegendPosition::Bottom),
        400,
        400,
    );
    let plot_area = data_plot_rect(&svg);
    let (_, bar_y, w, h) = bar_bbox(&svg);
    assert!((w - 144.0).abs() < 2.0, "width {w}: {svg}");
    assert!((h - 12.0).abs() < 1.0, "height {h}: {svg}");
    assert!(
        bar_y >= plot_area[1] + plot_area[3],
        "heatmap colourbar overlaps the panel: {svg}"
    );
}

#[test]
fn tick_labels_are_centered_below_the_bar_not_beside_it() {
    let svg = render(
        fill_scatter().legend_position(LegendPosition::Bottom),
        400,
        400,
    );
    assert!(
        svg.contains("text-anchor=\"middle\""),
        "no centered (middle-anchored) tick label found: {svg}"
    );
}
