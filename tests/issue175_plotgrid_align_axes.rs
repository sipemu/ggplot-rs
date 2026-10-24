//! sipemu/ggplot-rs#175: `PlotGrid::axis_collect` aligns a column's plot
//! areas and/or shares its x domain — the in-house alternative to
//! anofox-visualization's `track.rs` render-measure-render dance. "Done
//! when" (from the issue): a column of plots with different y-label widths
//! renders with pixel-identical plot-area x ranges and one shared x domain,
//! both read straight off the root `data-plot`/`data-xdomain` attributes
//! `track.rs`'s own `plot_rect`/`root_pair` already parse.

use ggplot_rs::prelude::*;

/// A plot whose y values are all near `magnitude`, so its y-axis break
/// labels are roughly `digits` characters wide — the wider the labels, the
/// more left margin an *unaligned* panel reserves for them.
fn plot_with_y_magnitude(magnitude: f64, x_lo: f64, x_hi: f64) -> GGPlot {
    let n = 20;
    let x: Vec<Value> = (0..n)
        .map(|i| Value::Float(x_lo + (x_hi - x_lo) * i as f64 / (n - 1) as f64))
        .collect();
    let y: Vec<Value> = (0..n)
        .map(|i| Value::Float(magnitude + (i as f64 - n as f64 / 2.0)))
        .collect();
    GGPlot::new(vec![("x".to_string(), x), ("y".to_string(), y)])
        .aes(Aes::new().x("x").y("y"))
        .geom_line()
}

/// `(x, y, w, h)` of every nested `<svg>`'s `data-plot`, in document order.
fn plot_rects(svg: &str) -> Vec<[f64; 4]> {
    svg.match_indices("data-plot=\"")
        .map(|(at, m)| {
            let start = at + m.len();
            let rest = &svg[start..];
            let end = rest.find('"').unwrap();
            let v: Vec<f64> = rest[..end]
                .split_whitespace()
                .map(|t| t.parse().unwrap())
                .collect();
            v.try_into().unwrap()
        })
        .collect()
}

/// `(x0, x1)` of every nested `<svg>`'s `data-xdomain`, in document order.
fn xdomains(svg: &str) -> Vec<(f64, f64)> {
    svg.match_indices("data-xdomain=\"")
        .map(|(at, m)| {
            let start = at + m.len();
            let rest = &svg[start..];
            let end = rest.find('"').unwrap();
            let v: Vec<f64> = rest[..end]
                .split_whitespace()
                .map(|t| t.parse().unwrap())
                .collect();
            (v[0], v[1])
        })
        .collect()
}

/// Three plots with deliberately different y-label widths ("1"-ish,
/// "1,234,567"-ish, "0.00001"-ish via a narrow integer-looking range) and
/// overlapping-but-different x ranges, so a shared x domain is a real
/// change, not a no-op.
fn three_plots() -> Vec<GGPlot> {
    vec![
        plot_with_y_magnitude(1.0, 0.0, 100.0),
        plot_with_y_magnitude(1_234_567.0, 10.0, 90.0),
        plot_with_y_magnitude(42.0, 0.0, 50.0),
    ]
}

#[test]
fn collect_x_aligns_plot_areas_and_shares_one_x_domain() {
    let svg = PlotGrid::new()
        .add_all(three_plots())
        .ncol(1)
        .axis_collect(AxisCollect::CollectX)
        .render_svg_native_with_size(800, 600)
        .expect("render");

    let rects = plot_rects(&svg);
    assert_eq!(rects.len(), 3, "expected 3 panels: {svg}");
    let (x0, _, w0, _) = (rects[0][0], rects[0][1], rects[0][2], rects[0][3]);
    for (i, r) in rects.iter().enumerate() {
        assert!(
            (r[0] - x0).abs() < 1e-6 && (r[2] - w0).abs() < 1e-6,
            "panel {i} plot-area x/width {r:?} does not match panel 0's ({x0}, {w0})"
        );
    }

    let domains = xdomains(&svg);
    assert_eq!(domains.len(), 3);
    let d0 = domains[0];
    for (i, d) in domains.iter().enumerate() {
        assert!(
            (d.0 - d0.0).abs() < 1e-6 && (d.1 - d0.1).abs() < 1e-6,
            "panel {i} x domain {d:?} does not match panel 0's {d0:?}"
        );
    }

    // The shared domain is the union of the three plots' own ranges (0..100
    // is the widest), not any single panel's original (narrower) range.
    assert!(
        d0.0 <= 0.5,
        "shared domain should cover the widest range: {d0:?}"
    );
    assert!(
        d0.1 >= 99.5,
        "shared domain should cover the widest range: {d0:?}"
    );

    // Only the bottom panel keeps its x-axis tick labels: compared to the
    // same plot's panel under `Keep` (which draws its own x axis), the top
    // two panels lose `<text>` elements (their x tick labels) while the
    // bottom panel's count is unchanged.
    let keep_svg = PlotGrid::new()
        .add_all(three_plots())
        .ncol(1)
        .render_svg_native_with_size(800, 600)
        .expect("render");
    let collected = panel_fragments(&svg);
    let kept = panel_fragments(&keep_svg);
    assert_eq!(collected.len(), 3);
    assert_eq!(kept.len(), 3);
    for i in 0..2 {
        assert!(
            count_text_elements(collected[i]) < count_text_elements(kept[i]),
            "panel {i} should lose its x tick labels under CollectX"
        );
    }
    // The bottom panel's x domain is now the (wider) union, so its break
    // count can legitimately differ from `Keep`'s — just check it still
    // draws *some* x tick labels.
    assert!(
        count_text_elements(collected[2]) > 0,
        "the bottom panel should still draw its own x tick labels under CollectX"
    );
}

/// Every nested `<svg>`'s own tag + body, in document order (skipping the
/// grid's root `<svg>`).
fn panel_fragments(svg: &str) -> Vec<&str> {
    svg.split("<svg ").skip(2).collect()
}

fn count_text_elements(svg: &str) -> usize {
    svg.matches("<text ").count()
}

/// Without any alignment (the default `AxisCollect::Keep`), the three
/// plots' very different y-label widths produce visibly different
/// plot-area left edges and keep each one's own x domain.
#[test]
fn keep_is_the_default_and_leaves_panels_unaligned() {
    let svg = PlotGrid::new()
        .add_all(three_plots())
        .ncol(1)
        .render_svg_native_with_size(800, 600)
        .expect("render");

    let rects = plot_rects(&svg);
    assert_eq!(rects.len(), 3);
    let lefts: Vec<f64> = rects.iter().map(|r| r[0]).collect();
    assert!(
        (lefts[0] - lefts[1]).abs() > 1.0,
        "unaligned panels should have different left edges (1-digit vs 7-digit y labels): {lefts:?}"
    );

    let domains = xdomains(&svg);
    assert_eq!(domains.len(), 3);
    assert!(
        (domains[0].0 - domains[1].0).abs() > 1.0 || (domains[0].1 - domains[1].1).abs() > 1.0,
        "unaligned panels should keep their own, different x domains: {domains:?}"
    );
}

/// `Align` alone equalises the plot areas but does not touch the x domain
/// or the per-cell x axis — the orthogonal half of `CollectX`.
#[test]
fn align_alone_equalises_plot_areas_but_keeps_each_panels_own_x_domain() {
    let svg = PlotGrid::new()
        .add_all(three_plots())
        .ncol(1)
        .axis_collect(AxisCollect::Align)
        .render_svg_native_with_size(800, 600)
        .expect("render");

    let rects = plot_rects(&svg);
    assert_eq!(rects.len(), 3);
    let (x0, w0) = (rects[0][0], rects[0][2]);
    for (i, r) in rects.iter().enumerate() {
        assert!(
            (r[0] - x0).abs() < 1e-6 && (r[2] - w0).abs() < 1e-6,
            "panel {i} plot-area {r:?} should align under `Align` too"
        );
    }

    let domains = xdomains(&svg);
    assert!(
        (domains[0].0 - domains[1].0).abs() > 1.0 || (domains[0].1 - domains[1].1).abs() > 1.0,
        "`Align` alone must not collect the x domain: {domains:?}"
    );
}

/// Each nested `<svg>`'s own `width="…" height="…"` — the cell's actual
/// rendered canvas size, as opposed to `plot_rects`' `data-plot` (the
/// *panel*, which sits inside that canvas with margins around it).
fn fragment_sizes(svg: &str) -> Vec<(f64, f64)> {
    panel_fragments(svg)
        .into_iter()
        .map(|frag| {
            let tag = &frag[..frag.find('>').unwrap_or(frag.len())];
            let attr = |n: &str| -> f64 {
                tag.split(&format!(" {n}=\""))
                    .nth(1)
                    .and_then(|r| r.split('"').next())
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(|| panic!("no {n} attr in {tag}"))
            };
            (attr("width"), attr("height"))
        })
        .collect()
}

/// A 300px chart cell plus two 24px `::TRACK`-style strip cells (event
/// ticks / a one-row heatmap, anofox-visualization's motivating case —
/// `chart_theme(o, 11.0)`'s own `plot_margin` is `half_line = 5.5` px a
/// side, same as the default theme used here). Each strip is well under
/// the plotting engine's 50px panel-height floor: `layout_built` would
/// otherwise give its panel `h = 50` regardless of the 24px canvas, badly
/// overflowing it (`y + h` far past the cell's own height) and getting
/// clipped. `CollectX` must still align every plot area exactly and give
/// each strip's *canvas* (the nested `<svg>`'s own size) its requested
/// 24px — `heights()`' weights sum to the render height here, so that
/// maps to an exact pixel count — while keeping its *panel* inside that
/// canvas, not overflowing it.
#[test]
fn collect_x_keeps_thin_strip_cells_at_their_requested_height_and_still_aligns_them() {
    let chart = plot_with_y_magnitude(1_000.0, 0.0, 100.0);
    let strip_a = plot_with_y_magnitude(1.0, 0.0, 100.0);
    let strip_b = plot_with_y_magnitude(1_234_567.0, 0.0, 100.0);

    let svg = PlotGrid::new()
        .add(chart)
        .add(strip_a)
        .add(strip_b)
        .ncol(1)
        .heights(&[300.0, 24.0, 24.0])
        .axis_collect(AxisCollect::CollectX)
        // Weights sum to 348 = the render height, so `tracks()` divides
        // the body into exactly 300/24/24 px with no rounding slack.
        .render_svg_native_with_size(500, 348)
        .expect("render");

    let rects = plot_rects(&svg);
    assert_eq!(rects.len(), 3, "expected 3 panels: {svg}");

    let (x0, w0) = (rects[0][0], rects[0][2]);
    for (i, r) in rects.iter().enumerate() {
        assert!(
            (r[0] - x0).abs() < 1e-6 && (r[2] - w0).abs() < 1e-6,
            "panel {i} plot-area x/width {r:?} does not match panel 0's ({x0}, {w0})"
        );
    }

    let domains = xdomains(&svg);
    assert_eq!(domains.len(), 3);
    let d0 = domains[0];
    for (i, d) in domains.iter().enumerate() {
        assert!(
            (d.0 - d0.0).abs() < 1e-6 && (d.1 - d0.1).abs() < 1e-6,
            "panel {i} x domain {d:?} does not match panel 0's {d0:?}"
        );
    }

    // Each strip cell's own rendered canvas is exactly its requested 24px
    // — the grid never grows a cell to fit an overflowing panel.
    let sizes = fragment_sizes(&svg);
    assert_eq!(sizes.len(), 3);
    for (i, (_, h)) in sizes.iter().enumerate().skip(1) {
        assert!(
            (h - 24.0).abs() <= 1.0,
            "strip panel {i} canvas height {h} should be within 1px of the requested 24px"
        );
    }

    // And the strips' *panels* stay inside that canvas — without the
    // `aspect_ratio` correction, `layout_built`'s 50px panel-height floor
    // would make `y + h` massively overflow a 24px canvas.
    for (i, (r, (_, canvas_h))) in rects.iter().zip(&sizes).enumerate().skip(1) {
        let (y, h) = (r[1], r[3]);
        assert!(
            y + h <= canvas_h + 0.5,
            "strip panel {i}'s plot area (y={y}, h={h}) overflows its {canvas_h}px canvas"
        );
        assert!(
            h < 49.0,
            "strip panel {i} panel height {h} looks like the unfixed 50px floor, not the aspect-ratio correction"
        );
    }
}

/// A column with a discrete x scale can't have a well-defined shared
/// domain: `CollectX` still aligns the plot areas, warns instead of
/// panicking or silently producing nonsense, and leaves each scale as-is.
#[test]
fn collect_x_on_a_discrete_x_scale_warns_and_skips_the_domain_only() {
    let discrete = |cat: &str| {
        GGPlot::new(vec![
            ("x".to_string(), vec![Value::Str(cat.to_string())]),
            ("y".to_string(), vec![Value::Float(1.0)]),
        ])
        .aes(Aes::new().x("x").y("y"))
        .geom_col()
    };
    let (_svg, warnings) = PlotGrid::new()
        .add(discrete("a"))
        .add(discrete("b"))
        .ncol(1)
        .axis_collect(AxisCollect::CollectX)
        .render_svg_native_with_warnings(400, 400)
        .expect("render");
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("x domain not collected")),
        "expected a warning about the discrete x scale: {warnings:?}"
    );
}
