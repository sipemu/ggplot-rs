//! sipemu/ggplot-rs#281: a `Top` discrete/binned-fill legend with a
//! `.subtitle()` and no `.title()` had the subtitle clamped down into the
//! legend's own row, overlapping it by a couple of px.
//!
//! Root cause: `renderer.rs`'s subtitle placement (step 8, and its
//! faceted twin near line 690) computed
//! `(plot_area.y - 2.0 - legend_lift).max(theme.title.size +
//! theme.subtitle.size)` — the generic "don't draw above the top margin"
//! floor was applied *before* subtracting the `Top` legend's lift
//! (`legend::top_legend_lift`, #233/#577). With no title, `plot_area.y -
//! 2.0 - legend_lift` collapses to roughly `plot_margin.top +
//! subtitle_height - 2.0`, which is smaller than the floor for the
//! default theme — so the floor always won and silently cancelled the
//! lift, putting the subtitle back where it would sit with *no* Top
//! legend at all: right on top of the legend's own band.
//!
//! The fix moves the `- legend_lift` term to after the `.max()`, which is
//! a no-op when there's no Top legend (`legend_lift == 0.0`) and
//! correctly shifts the floor itself up by the lift otherwise.

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;
use ggplot_rs::scale::color::RGBAColor;

fn frame(cols: Vec<(&str, Vec<Value>)>) -> Vec<(String, Vec<Value>)> {
    cols.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}
fn floats(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&f| Value::Float(f)).collect()
}
fn strs(v: &[&str]) -> Vec<Value> {
    v.iter().map(|s| Value::Str(s.to_string())).collect()
}

fn discrete_fill_data() -> Vec<(String, Vec<Value>)> {
    frame(vec![
        ("x", floats(&[1.0, 2.0])),
        ("y", floats(&[1.0, 2.0])),
        ("grp", strs(&["Group A", "Group B"])),
    ])
}

fn binned_fill_data() -> Vec<(String, Vec<Value>)> {
    frame(vec![
        ("x", floats(&[1.0, 2.0, 3.0])),
        ("y", floats(&[1.0, 1.0, 1.0])),
        ("v", floats(&[2.0, 22.0, 90.0])),
    ])
}

fn discrete_top_legend_plot() -> GGPlot {
    GGPlot::new(discrete_fill_data())
        .aes(Aes::new().x("x").y("y").fill("grp"))
        .geom_point()
        .legend_position(LegendPosition::Top)
}

fn binned_top_legend_plot() -> GGPlot {
    GGPlot::new(binned_fill_data())
        .aes(Aes::new().x("x").y("y").fill("v"))
        .geom_tile()
        .scale_fill_binned(
            vec![0.0, 10.0, 50.0, 100.0],
            RGBAColor {
                r: 240,
                g: 240,
                b: 200,
                a: 1.0,
            },
            RGBAColor {
                r: 30,
                g: 90,
                b: 30,
                a: 1.0,
            },
        )
        .legend_position(LegendPosition::Top)
}

/// `(y, font-size)` for every `<text ...>content</text>` — same approach
/// `top_legend_title_overlap_577.rs::text_boxes` uses.
fn text_boxes(svg: &str, content: &str) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    let mut search_from = 0;
    while let Some(rel) = svg[search_from..].find("<text ") {
        let start = search_from + rel;
        let Some(end) = svg[start..].find("</text>").map(|e| start + e) else {
            break;
        };
        let tag = &svg[start..end];
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

/// A text box's `(top, bottom)` span from its SVG `(y, font_size)` —
/// `dominant-baseline="middle"` (`src/render/svg_backend.rs`), so the box
/// is `y ± font_size / 2`.
fn span(b: (f64, f64)) -> (f64, f64) {
    let (y, size) = b;
    (y - size * 0.5, y + size * 0.5)
}

/// The topmost edge of the Top legend's own key swatches — the first
/// `legend_key_width`-square `<rect>` in the SVG (the default theme's
/// 12x12, same convention `legend_top_margin_233.rs::legend_first_row_y`
/// uses), independent of label text metrics.
fn legend_key_top_y(svg: &str) -> f64 {
    let marker = svg
        .find("width=\"12.00\" height=\"12.00\"")
        .expect("a legend key swatch rect");
    let tag_start = svg[..marker].rfind("<rect").expect("its <rect> start");
    let tag = &svg[tag_start..marker];
    attr_f64(tag, "y=\"")
}

/// The pre-#281 subtitle floor for the default theme (`theme_gray`):
/// `title.size + subtitle.size`. Computed from `Theme::default()` rather
/// than hardcoded so it tracks the theme if its defaults ever change.
fn default_subtitle_floor() -> f64 {
    let theme = Theme::default();
    theme.title.size + theme.subtitle.size
}

const LEGEND_SPACING: f64 = 4.0; // theme_gray's default legend_spacing
                                 // Tiny float slack for the `.max`/text-metric round trip, not a loosened bar.
const SLACK: f64 = 0.5;

#[test]
fn top_discrete_fill_legend_with_subtitle_no_title_sits_above_the_legend() {
    let svg = discrete_top_legend_plot()
        .subtitle("week: 3")
        .render_svg_native_with_size(400, 300)
        .unwrap();
    let subtitle = text_boxes(&svg, "week: 3");
    assert_eq!(subtitle.len(), 1, "{svg}");
    let subtitle_bottom = span(subtitle[0]).1;
    let legend_top = legend_key_top_y(&svg);
    let gap = legend_top - subtitle_bottom;
    assert!(
        gap >= LEGEND_SPACING - SLACK,
        "subtitle {:?} must end at least legend_spacing ({LEGEND_SPACING}) above the \
         legend's first key (y={legend_top}), got gap={gap}: {svg}",
        span(subtitle[0])
    );
}

#[test]
fn top_binned_fill_legend_with_subtitle_no_title_sits_above_the_legend() {
    let svg = binned_top_legend_plot()
        .subtitle("week: 3")
        .render_svg_native_with_size(400, 300)
        .unwrap();
    let subtitle = text_boxes(&svg, "week: 3");
    assert_eq!(subtitle.len(), 1, "{svg}");
    let subtitle_bottom = span(subtitle[0]).1;
    let legend_top = legend_key_top_y(&svg);
    let gap = legend_top - subtitle_bottom;
    assert!(
        gap >= LEGEND_SPACING - SLACK,
        "subtitle {:?} must end at least legend_spacing above the legend's first key \
         (y={legend_top}), got gap={gap}: {svg}",
        span(subtitle[0])
    );
}

#[test]
fn top_legend_with_title_and_subtitle_still_sits_below_both() {
    // Regression: with a title present, `plot_area.y - 2.0 - legend_lift`
    // was already above the floor before #281 (title_height > 0 pads it
    // out), so this path never hit the bug — must keep passing exactly.
    let svg = discrete_top_legend_plot()
        .title("My Title")
        .subtitle("week: 3")
        .render_svg_native_with_size(400, 300)
        .unwrap();
    let title = text_boxes(&svg, "My Title");
    let subtitle = text_boxes(&svg, "week: 3");
    assert_eq!(title.len(), 1, "{svg}");
    assert_eq!(subtitle.len(), 1, "{svg}");
    assert!(
        title[0].0 < subtitle[0].0,
        "title must sit above subtitle: title={:?} subtitle={:?}",
        title[0],
        subtitle[0]
    );
    let subtitle_bottom = span(subtitle[0]).1;
    let legend_top = legend_key_top_y(&svg);
    let gap = legend_top - subtitle_bottom;
    assert!(
        gap >= LEGEND_SPACING - SLACK,
        "gap={gap} subtitle={:?} legend_top={legend_top}: {svg}",
        span(subtitle[0])
    );
}

#[test]
fn top_legend_with_title_only_unaffected() {
    let svg = discrete_top_legend_plot()
        .title("My Title")
        .render_svg_native_with_size(400, 300)
        .unwrap();
    let title = text_boxes(&svg, "My Title");
    assert_eq!(title.len(), 1, "{svg}");
    let legend_top = legend_key_top_y(&svg);
    let title_bottom = span(title[0]).1;
    assert!(
        legend_top - title_bottom >= LEGEND_SPACING - SLACK,
        "title={:?} legend_top={legend_top}: {svg}",
        span(title[0])
    );
}

#[test]
fn a_wrapped_two_row_top_legend_with_subtitle_no_title_sits_above_the_first_row() {
    // Many groups force the horizontal auto-wrap onto 2+ rows on a narrow
    // canvas — mirrors `legend_top_margin_233.rs`'s own wrap test.
    const W: u32 = 420;
    const H: u32 = 400;
    let x12: Vec<Value> = (0..12).map(|i| Value::Float(i as f64)).collect();
    let y12: Vec<Value> = (0..12).map(|i| Value::Float((i % 3) as f64)).collect();
    let g12: Vec<Value> = (0..12)
        .map(|i| Value::Str(format!("Group {:02}", i + 1)))
        .collect();
    let data = frame(vec![("x", x12), ("y", y12), ("grp", g12)]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").fill("grp"))
        .geom_point()
        .legend_position(LegendPosition::Top)
        .subtitle("week: 3")
        .render_svg_native_with_size(W, H)
        .unwrap();
    let subtitle = text_boxes(&svg, "week: 3");
    assert_eq!(subtitle.len(), 1, "{svg}");
    let subtitle_bottom = span(subtitle[0]).1;
    let legend_top = legend_key_top_y(&svg);
    let gap = legend_top - subtitle_bottom;
    assert!(
        gap >= LEGEND_SPACING - SLACK,
        "wrapped legend: subtitle {:?} must clear its first row (y={legend_top}), gap={gap}: \
         {svg}",
        span(subtitle[0])
    );
}

#[test]
fn no_top_legend_subtitle_position_is_byte_identical() {
    // No Top legend (default Right position) -> `legend_lift` is 0.0, so
    // the fix (moving `- legend_lift` to after the `.max()`) must be a
    // no-op here: `max(a, b) - 0.0 == max(a - 0.0, b - 0.0)` exactly. The
    // subtitle must land exactly on the pre-#281 floor, unchanged.
    let svg = GGPlot::new(discrete_fill_data())
        .aes(Aes::new().x("x").y("y").fill("grp"))
        .geom_point()
        .subtitle("week: 3")
        .render_svg_native_with_size(400, 300)
        .unwrap();
    let subtitle = text_boxes(&svg, "week: 3");
    assert_eq!(subtitle.len(), 1, "{svg}");
    assert_eq!(
        subtitle[0].0,
        default_subtitle_floor(),
        "no-legend subtitle y must stay exactly on the pre-#281 floor value"
    );
}

/// The exact minimal repro posted on the issue: `theme_void()`,
/// `legend_margin` reset to `Margin::default()`, a `scale_fill_binned`
/// legend, a `.subtitle()`, `LegendPosition::Top`, at 1200x320.
/// `theme_void`'s `subtitle` element is `ElementText::blank()`, whose
/// `size` stays at the struct default of 12.0 (bigger than
/// `theme_gray`'s 11.0) even though it's invisible, and the renderer
/// draws the subtitle text regardless of `theme.subtitle.visible`.
///
/// Confirmed (not just hand-derived) against the actual pre-#281
/// `renderer.rs`: this reproduces the issue's exact reported numbers,
/// subtitle drawn at y=25.20 (span 19.2 to 31.2) against a legend swatch
/// at y=28.0, a 3.2px overlap.
#[test]
fn theme_void_binned_legend_with_subtitle_matches_the_issues_exact_repro() {
    let data: Vec<(String, Vec<Value>)> = vec![
        ("x".into(), (0..5).map(|i| Value::Float(i as f64)).collect()),
        ("y".into(), (0..5).map(|_| Value::Float(0.0)).collect()),
        (
            "v".into(),
            [2.0, 15.0, 30.0, 55.0, 90.0]
                .iter()
                .map(|f| Value::Float(*f))
                .collect(),
        ),
    ];
    let mut theme = ggplot_rs::theme::presets::theme_void();
    theme.legend_margin = ggplot_rs::theme::Margin::default();
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").fill("v"))
        .geom_tile()
        .scale_fill_binned(
            vec![0.0, 10.0, 50.0, 100.0],
            RGBAColor {
                r: 240,
                g: 240,
                b: 200,
                a: 1.0,
            },
            RGBAColor {
                r: 30,
                g: 30,
                b: 120,
                a: 1.0,
            },
        )
        .theme(theme)
        .subtitle("week: 3")
        .legend_position(LegendPosition::Top)
        .render_svg_native_with_size(1200, 320)
        .unwrap();
    let subtitle = text_boxes(&svg, "week: 3");
    assert_eq!(subtitle.len(), 1, "{svg}");
    let subtitle_bottom = span(subtitle[0]).1;
    let legend_top = legend_key_top_y(&svg);
    let gap = legend_top - subtitle_bottom;
    assert!(
        gap >= LEGEND_SPACING - SLACK,
        "issue's exact repro: subtitle {:?} must end at least legend_spacing above the \
         legend's first key (y={legend_top}), got gap={gap}: {svg}",
        span(subtitle[0])
    );
}
