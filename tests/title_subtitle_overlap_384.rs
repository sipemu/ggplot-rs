//! sipemu/ggplot-rs#384: with both a title and a subtitle, the subtitle
//! overlapped the title and sat half inside the panel — even with the
//! default theme.
//!
//! Root cause: `renderer.rs` drew the subtitle's *centre* (text is
//! `dominant-baseline="middle"`) 2px above the panel top, and the title's
//! centre `0.9 × title.size` above it, ignoring the bands `PlotLayout`
//! reserves (`title.size × 2` for the title, `subtitle.size × 1.5` for the
//! subtitle). The fix (`Renderer::title_subtitle_y`) stacks both centres
//! bottom-up inside those bands, in the single-panel and the faceted path.

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;
use ggplot_rs::scale::continuous::ScaleContinuous;

fn data() -> Vec<(String, Vec<Value>)> {
    vec![
        (
            "x".to_string(),
            [10.0, 20.0, 30.0, 10.0, 20.0, 30.0]
                .iter()
                .map(|&f| Value::Float(f))
                .collect(),
        ),
        (
            "y".to_string(),
            [1.0, 3.0, 2.0, 2.0, 1.0, 3.0]
                .iter()
                .map(|&f| Value::Float(f))
                .collect(),
        ),
        (
            "g".to_string(),
            ["a", "a", "a", "b", "b", "b"]
                .iter()
                .map(|s| Value::Str(s.to_string()))
                .collect(),
        ),
    ]
}

fn plot() -> GGPlot {
    GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .title("Tooth growth")
        .subtitle("by supplement")
}

/// `(y, font-size)` of the `<text>` whose content is exactly `content`.
fn text_box(svg: &str, content: &str) -> (f64, f64) {
    let needle = format!(">{content}</text>");
    let end = svg
        .find(&needle)
        .unwrap_or_else(|| panic!("no <text> {content:?}: {svg}"));
    let start = svg[..end].rfind("<text ").unwrap();
    let tag = &svg[start..end];
    (attr(tag, "y=\""), attr(tag, "font-size=\""))
}

fn attr(tag: &str, key: &str) -> f64 {
    let s = tag.find(key).unwrap() + key.len();
    let e = tag[s..].find('"').unwrap() + s;
    tag[s..e].parse().unwrap()
}

/// `dominant-baseline="middle"` → the box is `y ± size / 2`.
fn span((y, size): (f64, f64)) -> (f64, f64) {
    (y - size / 2.0, y + size / 2.0)
}

/// Top edge of the (first) panel: theme_gray's `#EBEBEB` panel background.
fn panel_top(svg: &str) -> f64 {
    let m = svg.find(r##"fill="#EBEBEB""##).expect("a panel background");
    let start = svg[..m].rfind("<rect").unwrap();
    attr(&svg[start..m], "y=\"")
}

const SLACK: f64 = 0.01;

fn assert_stacked(svg: &str, floor: f64) {
    let title = span(text_box(svg, "Tooth growth"));
    let subtitle = span(text_box(svg, "by supplement"));
    assert!(title.0 >= 0.0, "title {title:?} leaves the canvas: {svg}");
    assert!(
        title.1 <= subtitle.0 + SLACK,
        "title {title:?} overlaps subtitle {subtitle:?}: {svg}"
    );
    assert!(
        subtitle.1 <= floor + SLACK,
        "subtitle {subtitle:?} runs below y={floor}: {svg}"
    );
}

#[test]
fn default_theme_title_and_subtitle_do_not_overlap() {
    let svg = plot()
        .theme_gray()
        .render_svg_native_with_size(480, 300)
        .unwrap();
    assert_stacked(&svg, panel_top(&svg));
}

#[test]
fn every_preset_and_title_size_stacks_cleanly() {
    for size in [8.0, 13.2, 16.0, 22.0, 28.0, 40.0] {
        let mut theme = ggplot_rs::theme::presets::theme_gray();
        theme.title.size = size;
        let svg = plot()
            .theme(theme)
            .render_svg_native_with_size(480, 300)
            .unwrap();
        assert_stacked(&svg, panel_top(&svg));
    }
    for sub in [6.0, 11.0, 20.0] {
        let mut theme = ggplot_rs::theme::presets::theme_gray();
        theme.subtitle.size = sub;
        let svg = plot()
            .theme(theme)
            .render_svg_native_with_size(480, 300)
            .unwrap();
        assert_stacked(&svg, panel_top(&svg));
    }
}

#[test]
fn faceted_title_and_subtitle_clear_the_strips() {
    let svg = plot()
        .facet_wrap("g", None)
        .theme_gray()
        .render_svg_native_with_size(480, 300)
        .unwrap();
    // The panel rects start below the strips, so bound by the strip label.
    let strip = span(text_box(&svg, "a"));
    assert_stacked(&svg, strip.0);
}

#[test]
fn top_x_axis_lifts_the_subtitle_above_its_labels() {
    let svg = plot()
        .scale_x_continuous(ScaleContinuous::new().with_position_opposite())
        .theme_gray()
        .render_svg_native_with_size(480, 300)
        .unwrap();
    let label = span(text_box(&svg, "20"));
    assert!(
        label.1 <= panel_top(&svg),
        "x labels should be on top: {svg}"
    );
    assert_stacked(&svg, label.0);
}

#[test]
fn title_only_keeps_its_place() {
    // No subtitle: the title stays 0.9 × size above the panel, as before.
    let svg = GGPlot::new(data())
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .title("Tooth growth")
        .theme_gray()
        .render_svg_native_with_size(480, 300)
        .unwrap();
    let (y, size) = text_box(&svg, "Tooth growth");
    assert!((panel_top(&svg) - size * 0.9 - y).abs() < SLACK, "{svg}");
}
