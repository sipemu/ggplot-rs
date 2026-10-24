//! #229 part 2: a `geom_label` label containing a literal `"\n"`.
//!
//! Before this fix, `GeomLabel::draw` only ever split a label into
//! multiple lines when `.with_wrap(..)` was set, and even then
//! `wrap_text`'s `split_whitespace()` treated an embedded `"\n"` as just
//! another word separator rather than a forced break. With no
//! `.with_wrap(..)` (the common case), a `"\n"`-containing label drew
//! exactly one box (so "one box per line" wasn't actually the bug), but
//! the raw `"\n"` byte sat unbroken inside the single `<text>` content —
//! SVG has no line-break effect for that, so it rendered as one squished
//! line, not two. `geom_label` now splits on `"\n"` into forced lines
//! *before* any width-based wrapping, drawing one box sized to fit all of
//! them (ggplot2's behaviour), with one `<text>` per line inside it.

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

/// `<rect>`/`<text>` elements carrying this row's own `data-geom="label"`
/// (the plot/panel background rects, and axis-label `<text>`s, don't).
fn label_mark_count(svg: &str) -> usize {
    svg.matches("data-geom=\"label\"").count()
}

/// This label's own `<rect ... width="…" …>`'s `width` value — found by
/// walking back from its `data-geom="label"` marker to the nearest
/// preceding `<rect`.
fn label_rect_width(svg: &str) -> f64 {
    let marker = svg.find("data-geom=\"label\"").expect("a label mark");
    let rect_start = svg[..marker].rfind("<rect").expect("its <rect> start");
    let frag = &svg[rect_start..marker];
    let w_start = frag.find("width=\"").unwrap() + "width=\"".len();
    let w_end = frag[w_start..].find('"').unwrap() + w_start;
    frag[w_start..w_end].parse().unwrap()
}

#[test]
fn a_forced_linebreak_with_no_wrap_set_draws_one_box_and_two_lines() {
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("label", strs(&["5.4K\norders"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_label()
        .render_svg_native_with_size(300, 200)
        .unwrap();

    // One box (<rect>) and two text lines (<text>), all tagged
    // data-geom="label" — three marks total; and only one <rect> of
    // those (the plot/panel backgrounds are the other two <rect>s).
    assert_eq!(label_mark_count(&svg), 3, "{svg}");
    assert_eq!(svg.matches("<rect").count(), 3, "{svg}");
    // Never one <text> with a raw, unbroken "\n" inside it.
    assert!(!svg.contains("5.4K\norders"), "{svg}");
    assert!(svg.contains(">5.4K</text>"), "{svg}");
    assert!(svg.contains(">orders</text>"), "{svg}");
}

#[test]
fn a_forced_linebreak_box_is_sized_to_fit_the_widest_line() {
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        // The second line is much wider than the first — the box must
        // fit it, not just the first line.
        ("label", strs(&["x\nsome much longer second line"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_label()
        .render_svg_native_with_size(400, 300)
        .unwrap();
    let width = label_rect_width(&svg);
    // A single-char line alone would give a tiny box; the real box must
    // be noticeably wider to fit "some much longer second line".
    assert!(width > 100.0, "box too narrow: {width} in {svg}");
}

#[test]
fn no_linebreak_single_line_box_width_is_unchanged() {
    // Byte-identical guard: a plain (no "\n") label's box width must stay
    // exactly the old char-count estimate.
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("label", strs(&["plain label"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_label()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    let width = label_rect_width(&svg);
    // "plain label" is 11 chars; old formula: 11 * size * 0.6 + 2*padding
    // (size 10.0, padding 3.0, GeomLabel's defaults).
    let expected = 11.0 * 10.0 * 0.6 + 2.0 * 3.0;
    assert!((width - expected).abs() < 1e-6, "{width} vs {expected}");
}

#[test]
fn forced_linebreak_combines_with_explicit_wrap() {
    // A "\n" paragraph that itself needs word-wrapping: each paragraph is
    // wrapped independently, then concatenated in order.
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("label", strs(&["a b c d e f\nz"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_label_with(GeomLabel::default().with_wrap(20.0))
        .render_svg_native_with_size(300, 300)
        .unwrap();
    // "z" must still be its own last line, after however many lines "a b
    // c d e f" wraps to — and it's still a single box: the plot/panel
    // backgrounds are the only other `<rect>`s.
    assert!(svg.contains(">z</text>"), "{svg}");
    assert_eq!(svg.matches("<rect").count(), 3, "{svg}");
}
