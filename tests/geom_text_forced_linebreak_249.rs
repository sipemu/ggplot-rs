//! #249: `geom_text` honours an embedded `"\n"` as a forced line break,
//! the same behaviour `geom_label` got in #229.
//!
//! Before this fix, `GeomText::draw` only ever split a label into
//! multiple lines when `.with_wrap(..)` was set; with no `.with_wrap(..)`
//! (the common case), a `"\n"`-containing label drew a single `<text>`
//! with the raw `"\n"` byte sitting unbroken inside it — SVG has no
//! line-break effect for that, so it rendered as one squished line, not
//! two. A caller wanting a multi-line `geom_text` label with a halo
//! (`bg_colour`/`bg_r`, #229) had to split the text into separate rows
//! and offset them itself (DataZooDE/anofox-visualization#487: diagram
//! edge labels like "5.4K\norders"). `geom_text` now splits on `"\n"`
//! into forced lines *before* any width-based wrapping, drawing one
//! `<text>` per line, vertically centred around the anchor per `vjust`,
//! with the halo (if any) applied to every line.

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

/// This mark's own `<text ... data-geom="text" ...>` elements, in
/// document order (axis tick labels are plain `<text>` too, but not that
/// `data-geom`).
fn text_elements(svg: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = svg;
    let mut base = 0usize;
    while let Some(marker) = rest.find("data-geom=\"text\"") {
        let abs_marker = base + marker;
        let start = svg[..abs_marker].rfind("<text").expect("its <text> start");
        let end = svg[abs_marker..].find("</text>").unwrap() + abs_marker + "</text>".len();
        out.push(&svg[start..end]);
        base = end;
        rest = &svg[base..];
    }
    out
}

fn attr_f64(tag: &str, key: &str) -> f64 {
    let needle = format!(" {key}=\"");
    let at = tag.find(&needle).unwrap() + needle.len();
    let rest = &tag[at..];
    rest[..rest.find('"').unwrap()].parse().unwrap()
}

#[test]
fn a_forced_linebreak_with_no_wrap_set_draws_two_text_elements() {
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("label", strs(&["5.4K\norders"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text()
        .render_svg_native_with_size(300, 200)
        .unwrap();

    let elems = text_elements(&svg);
    assert_eq!(elems.len(), 2, "{svg}");
    // Never one <text> with a raw, unbroken "\n" inside it.
    assert!(!svg.contains("5.4K\norders"), "{svg}");
    assert!(elems[0].contains(">5.4K</text>"), "{}", elems[0]);
    assert!(elems[1].contains(">orders</text>"), "{}", elems[1]);
}

#[test]
fn the_two_lines_are_vertically_centred_around_the_anchor() {
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("label", strs(&["5.4K\norders"])),
    ]);
    let size = 12.0;
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text_with(GeomText {
            size,
            ..GeomText::default()
        })
        .render_svg_native_with_size(300, 200)
        .unwrap();
    let elems = text_elements(&svg);
    assert_eq!(elems.len(), 2);
    let y0 = attr_f64(elems[0], "y");
    let y1 = attr_f64(elems[1], "y");
    // Default line_height 1.2: consecutive lines are `size * 1.2` apart.
    let gap = y1 - y0;
    assert!((gap - size * 1.2).abs() < 0.5, "gap = {gap}, size = {size}");
}

#[test]
fn no_linebreak_single_line_output_is_byte_identical() {
    // Byte-identical guard: a plain (no "\n") label renders exactly as
    // before #249 — one <text> at the pre-#249 single-line y.
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("label", strs(&["plain label"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert_eq!(text_elements(&svg).len(), 1, "{svg}");
    assert!(svg.contains(">plain label</text>"), "{svg}");
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
        .geom_text_with(GeomText::default().with_wrap(20.0))
        .render_svg_native_with_size(300, 300)
        .unwrap();
    // "z" must still be its own last line, after however many lines "a b
    // c d e f" wraps to.
    assert!(svg.contains(">z</text>"), "{svg}");
    let lines = ggplot_rs::render::text::wrap_text("a b c d e f", 20.0, 10.0);
    assert_eq!(text_elements(&svg).len(), lines.len() + 1, "{svg}");
}

#[test]
fn the_halo_is_applied_to_every_line() {
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("label", strs(&["5.4K\norders"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text_with(GeomText::default().bg_colour((255, 255, 255)).bg_r(2.0))
        .render_svg_native_with_size(300, 200)
        .unwrap();
    let elems = text_elements(&svg);
    assert_eq!(elems.len(), 2, "{svg}");
    for e in elems {
        assert!(e.contains("paint-order=\"stroke\""), "{e}");
        assert!(e.contains("stroke=\"#FFFFFF\""), "{e}");
    }
}

#[test]
fn an_empty_label_still_draws_the_single_empty_line_as_before() {
    let data = frame(vec![
        ("x", floats(&[1.0])),
        ("y", floats(&[1.0])),
        ("label", strs(&[""])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").label("label"))
        .geom_text()
        .render_svg_native_with_size(300, 200)
        .unwrap();
    assert_eq!(text_elements(&svg).len(), 1, "{svg}");
}
