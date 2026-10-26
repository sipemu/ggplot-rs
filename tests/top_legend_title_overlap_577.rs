//! anofox-visualization#577 (same B3-2 top-band layout as #233/#545): a
//! plot's own `.title()`/`.subtitle()` block and a `Top` legend's own
//! title drew on top of each other at narrow widths whenever the `Top`
//! legend was a continuous, horizontally-drawn colourbar (a continuous
//! `fill`/`color` scale) *with its own title* — `top_legend_lift` (the
//! amount `renderer.rs` lifts the title/subtitle above the legend band)
//! only ever used `legend_band_height`'s flat discrete-row height, never
//! `continuous_horizontal_bar_height`'s title-aware height that `plot.rs`
//! already reserves in `plot_area.y` via
//! `continuous_horizontal_legend_extra_height` (#211) — so the lift
//! undershot by exactly the legend title's own height, and the plot's
//! title/subtitle landed inside the legend's title row instead of above
//! it.

use ggplot_rs::guide::config::GuideLegend;
use ggplot_rs::prelude::*;

/// A continuous `fill` legend title is empty by default (`ScaleColorGradient2::new`,
/// `src/scale/gradient.rs`) unless given one explicitly via `guides(...)` —
/// the real-world trigger (anofox-visualization's flow map) always sets one.
fn titled_fill_plot() -> GGPlot {
    let data: Vec<(String, Vec<Value>)> = vec![
        (
            "x".into(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "y".into(),
            vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)],
        ),
        (
            "val".into(),
            vec![Value::Float(1.0), Value::Float(50.0), Value::Float(100.0)],
        ),
    ];
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").fill("val"))
        .geom_tile()
        .guides(GuideLegend::new().with_title("Flow volume"))
        .title("Flow by region")
        .subtitle("Weekly totals")
}

/// `(y, font_size)` for every `<text ...>...</text>` whose content is
/// exactly `content` — keyed off content rather than the `x` anchor
/// `all_text` (sipemu/ggplot-rs#545's test) uses, since the three labels
/// here share no common anchor.
fn text_boxes(svg: &str, content: &str) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    let mut search_from = 0;
    while let Some(rel) = svg[search_from..].find("<text ") {
        let start = search_from + rel;
        let Some(end) = svg[start..].find("</text>").map(|e| start + e) else {
            break;
        };
        let tag = &svg[start..end];
        // Same split `all_text` (sipemu/ggplot-rs#545's test) uses: the
        // text content is everything after the tag's own closing `>`.
        let text_start = tag.rfind('>').unwrap() + 1;
        if &tag[text_start..] == content {
            out.push((attr_f64(tag, "y=\""), attr_f64(tag, "font-size=\"")));
        }
        search_from = end + 1;
    }
    out
}

fn attr_f64(tag: &str, key: &str) -> f64 {
    let start = tag.find(key).unwrap() + key.len();
    let end = tag[start..].find('"').unwrap() + start;
    tag[start..end].parse().unwrap()
}

/// A text box's vertical span from its SVG `(y, font_size)` — `y` is the
/// glyph's vertical *center* (`dominant-baseline="middle"`,
/// `src/render/svg_backend.rs`), so the box is `y ± font_size / 2`.
/// Deliberately *not* used to check the title-vs-subtitle gap: those two
/// lines are drawn intentionally close by design (`renderer.rs`'s
/// `title_y`/`subtitle_y` formulas put them `title.size * 0.9 - 2` px
/// apart, tighter than one full line height) — real glyphs don't fill
/// their whole font-size box, so that by-design closeness isn't the bug
/// #577 reported. Only the subtitle-vs-legend-title gap (the pair that
/// actually collided) is checked this way.
fn span(b: (f64, f64)) -> (f64, f64) {
    let (y, size) = b;
    (y - size * 0.5, y + size * 0.5)
}

/// A visible gap, not just "don't technically overlap" — the broken
/// `top_legend_lift` (no `continuous_horizontal_legend_extra_height`
/// awareness) left only ~1px between the subtitle and the legend title in
/// this exact repro, which `!overlaps(...)` alone wouldn't have caught.
/// The fixed lift opens a ~30px gap instead, so 10px comfortably
/// distinguishes "fixed" from "undershot by exactly the missing term".
const MIN_CLEARANCE: f64 = 10.0;

/// At every width in the sweep, the plot's title/subtitle block must sit
/// well above a Top continuous-fill legend's own title (which in turn
/// sits right above its colour bar) — not overlapping, and not merely
/// grazing it.
#[test]
fn title_subtitle_and_top_colorbar_legend_title_do_not_overlap() {
    const H: u32 = 340;
    for w in [250u32, 330, 390, 480, 600] {
        let svg = titled_fill_plot()
            .legend_position(LegendPosition::Top)
            .render_svg_native_with_size(w, H)
            .unwrap();

        let title = text_boxes(&svg, "Flow by region");
        let subtitle = text_boxes(&svg, "Weekly totals");
        let legend_title = text_boxes(&svg, "Flow volume");
        assert_eq!(
            title.len(),
            1,
            "w={w}: expected exactly one plot title: {svg}"
        );
        assert_eq!(
            subtitle.len(),
            1,
            "w={w}: expected exactly one subtitle: {svg}"
        );
        assert_eq!(
            legend_title.len(),
            1,
            "w={w}: expected exactly one legend title (the explicit guide title): {svg}"
        );

        let subtitle_span = span(subtitle[0]);
        let legend_span = span(legend_title[0]);

        // Title above subtitle (by center, not span — the two are drawn
        // intentionally close, see `span`'s doc comment) above the
        // legend's own title, with a minimum clearance at that last
        // boundary — a swap, an overlap, or a too-close graze are all bugs.
        assert!(
            title[0].0 < subtitle[0].0,
            "w={w}: title center {:?} is not above subtitle center {:?}",
            title[0],
            subtitle[0]
        );
        let clearance = legend_span.0 - subtitle_span.1;
        assert!(
            clearance >= MIN_CLEARANCE,
            "w={w}: only {clearance:.1}px between subtitle {subtitle_span:?} and the Top \
             legend's own title {legend_span:?} (want >= {MIN_CLEARANCE}px)"
        );
    }
}
