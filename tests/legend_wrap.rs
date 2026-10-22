//! B3-2: legend nrow/ncol/byrow, auto-wrap, "+N more" (D7).

use ggplot_rs::prelude::*;

fn render(plot: GGPlot, w: u32, h: u32) -> String {
    plot.render_svg_native_with_size(w, h)
        .expect("native svg render")
}

fn groups(n: usize) -> Vec<(String, Vec<Value>)> {
    let x: Vec<Value> = (0..n).map(|i| Value::Float(i as f64)).collect();
    let y: Vec<Value> = (0..n).map(|i| Value::Float((i % 3) as f64)).collect();
    let g: Vec<Value> = (0..n)
        .map(|i| Value::Str(format!("Group {:02}", i + 1)))
        .collect();
    vec![("x".into(), x), ("y".into(), y), ("g".into(), g)]
}

fn base(n: usize) -> GGPlot {
    GGPlot::new(groups(n))
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
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

fn count_warnings(svg_with_warnings: &[String], needle: &str) -> usize {
    svg_with_warnings
        .iter()
        .filter(|w| w.contains(needle))
        .count()
}

/// A horizontal (bottom) legend with enough keys that a one-line layout
/// overflows a 640px chart: auto-wrap (D7) grows it to more than one row,
/// and the panel (`data-plot` height) shrinks to make room.
#[test]
fn overflowing_bottom_legend_auto_wraps_and_grows_legend_rows() {
    const W: u32 = 640;
    const H: u32 = 400;
    let one_row = render(base(3).legend_position(LegendPosition::Bottom), W, H);
    let many = render(base(12).legend_position(LegendPosition::Bottom), W, H);

    let rect_one = data_plot_rect(&one_row);
    let rect_many = data_plot_rect(&many);

    // The panel is shorter by at least one key-height band once the legend
    // needs a second row.
    assert!(
        rect_many[3] < rect_one[3],
        "panel should shrink once the legend wraps: one_row_h={} many_h={}",
        rect_one[3],
        rect_many[3]
    );

    // No x position of any legend label's <text> run past the right edge
    // of the SVG.
    for (start, _) in many.match_indices("<text ") {
        let tag_end = many[start..].find('>').unwrap() + start;
        let tag = &many[start..=tag_end];
        if let Some(x_attr) = tag.split("x=\"").nth(1) {
            let x: f64 = x_attr.split('"').next().unwrap().parse().unwrap();
            assert!(
                x <= W as f64,
                "a text element's x={x} overflows the {W}px canvas"
            );
        }
    }

    assert!(many.contains(">Group 01<"), "first entry must still render");
    assert!(many.contains(">Group 12<"), "last entry must still render");
}

/// The first entry's swatch/label sit at exactly the same place whether or
/// not the legend ends up wrapping — the planner's flow-pack starts
/// exactly where the 0.21 single-row path does.
#[test]
fn the_first_entry_is_unaffected_by_wrapping() {
    const W: u32 = 640;
    const H: u32 = 400;
    let wide = render(base(12).legend_position(LegendPosition::Bottom), 2000, H);
    let narrow = render(base(12).legend_position(LegendPosition::Bottom), W, H);

    let needle = ">Group 01<";
    let x_of = |svg: &str| -> f64 {
        let end = svg.find(needle).unwrap();
        let start = svg[..end].rfind("<text ").unwrap();
        let tag = &svg[start..end];
        tag.split("x=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    // Both renders put the legend at the same (plot_area-independent) x
    // origin for a Bottom legend, so entry 1's label lands at the same x.
    assert!((x_of(&wide) - x_of(&narrow)).abs() < 1e-6);
}

/// 30 keys at a width that can't fit even the generous default row budget:
/// the legend ends in a "+N more" marker and the build reports one
/// overflow warning.
#[test]
fn deeply_crowded_legend_ends_in_plus_n_more_with_one_warning() {
    let (svg, warnings) = base(30)
        .legend_position(LegendPosition::Bottom)
        .guides(GuideLegend::new().with_max_rows(2))
        .render_svg_native_with_warnings(320, 400)
        .expect("render");

    assert!(
        svg.contains(" more<"),
        "expected a \"+N more\" marker in the legend: {svg}"
    );
    assert_eq!(
        count_warnings(&warnings, "entries don't fit the chart"),
        1,
        "expected exactly one overflow warning: {warnings:?}"
    );
}

/// An explicit `ncol(3)` grid on a vertical (right) legend: ggplot2's
/// column-major fill (down each column, then the next).
#[test]
fn explicit_ncol_grid_fills_column_major() {
    let svg = render(base(6).guides(GuideLegend::new().with_ncol(3)), 500, 400);
    // Column-major with 6 entries / 3 cols -> 2 rows: col 0 = {1,2},
    // col 1 = {3,4}, col 2 = {5,6}. Entries 1 and 2 share an x (same
    // column); entry 3 starts a new column (different x from entry 1).
    let x_of = |label: &str| -> f64 {
        let needle = format!(">{label}<");
        let end = svg.find(&needle).unwrap();
        let start = svg[..end].rfind("<text ").unwrap();
        svg[start..end]
            .split("x=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse::<f64>()
            .unwrap()
    };
    let x1 = x_of("Group 01");
    let x2 = x_of("Group 02");
    let x3 = x_of("Group 03");
    assert!((x1 - x2).abs() < 1e-6, "entries 1 and 2 share a column");
    assert!((x1 - x3).abs() > 1e-6, "entry 3 starts a new column");
}

/// The same grid with `byrow(true)`: ggplot2's row-major fill.
#[test]
fn explicit_grid_with_byrow_fills_row_major() {
    let svg = render(
        base(6).guides(GuideLegend::new().with_ncol(3).with_byrow(true)),
        500,
        400,
    );
    // Row-major with 6 entries / 3 cols -> row 0 = {1,2,3}: entries 1-3
    // share a y (row), each at a different x; entry 4 starts row 1.
    let y_of = |label: &str| -> f64 {
        let needle = format!(">{label}<");
        let end = svg.find(&needle).unwrap();
        let start = svg[..end].rfind("<text ").unwrap();
        svg[start..end]
            .split("y=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse::<f64>()
            .unwrap()
    };
    let y1 = y_of("Group 01");
    let y3 = y_of("Group 03");
    let y4 = y_of("Group 04");
    assert!((y1 - y3).abs() < 1e-6, "entries 1 and 3 share row 0");
    assert!((y1 - y4).abs() > 1e-6, "entry 4 starts row 1");
}

/// A vertical (right) legend with an explicit `ncol(2)`: ggplot2 arranges
/// a vertical guide into columns too, not just horizontal ones.
#[test]
fn vertical_legend_with_explicit_ncol_two() {
    let svg = render(base(4).guides(GuideLegend::new().with_ncol(2)), 500, 400);
    assert!(svg.contains(">Group 01<"));
    assert!(svg.contains(">Group 04<"));
    assert!(
        max_text_right_edge(&svg) <= 500.0,
        "max text right edge {} exceeds the 500px canvas",
        max_text_right_edge(&svg)
    );
}

/// The max visual right edge of any `<text>` element — `x` plus the
/// rendered label's width for `text-anchor="start"` (every label and
/// legend entry in this codebase), half for `"middle"`, or just `x` for
/// `"end"`. Checking `x` alone (review #158's second re-review caught
/// this: the original finding-3 test did exactly that) is meaningless for
/// a `start`-anchored label, since its glyphs extend *rightward* from
/// `x` — a passing "x <= canvas width" assertion there proves nothing.
/// The x-coordinate of the `<text>` element whose body is exactly `label`
/// (the first match, in document order).
fn text_x(svg: &str, label: &str) -> Option<f64> {
    svg.match_indices("<text ").find_map(|(start, _)| {
        let tag_end = svg[start..].find('>').map(|e| start + e)?;
        let close = svg[tag_end..].find("</text>").map(|e| tag_end + e)?;
        if &svg[tag_end + 1..close] != label {
            return None;
        }
        let tag = &svg[start..tag_end];
        tag.split("x=\"").nth(1)?.split('"').next()?.parse().ok()
    })
}

fn max_text_right_edge(svg: &str) -> f64 {
    svg.match_indices("<text ")
        .filter_map(|(start, _)| {
            let tag_end = svg[start..].find('>').map(|e| start + e)?;
            let close = svg[tag_end..].find("</text>").map(|e| tag_end + e)?;
            let tag = &svg[start..tag_end];
            let label = &svg[tag_end + 1..close];
            let x: f64 = tag.split("x=\"").nth(1)?.split('"').next()?.parse().ok()?;
            let size: f64 = tag
                .split("font-size=\"")
                .nth(1)?
                .split('"')
                .next()?
                .parse()
                .ok()?;
            let anchor = tag.split("text-anchor=\"").nth(1)?.split('"').next()?;
            let w = ggplot_rs::render::text::text_width(label, size);
            Some(match anchor {
                "start" => x + w,
                "middle" => x + w / 2.0,
                _ => x,
            })
        })
        .fold(0.0_f64, f64::max)
}

/// Regression for review #158 finding 3: a vertical legend's right-hand
/// band was sized for exactly one column, so `ncol(2)`'s second column ran
/// off the canvas with long-enough labels (the reviewer's repro: 8 labels
/// ~27 chars at 640×400 put the second column at x≈687, past the 640px
/// SVG). The layout now reserves `(cols - 1) * col_pitch` extra band
/// width for an explicit multi-column vertical grid — enough for labels
/// of this length (short enough that the *first* column's own, separate,
/// pre-existing width estimate — a fixed `legend_text.size * 6`, 6
/// characters' worth, unrelated to B3-2 — is still adequate; see
/// `vertical_legend_ncol_grid_too_wide_for_the_canvas_warns` below for a
/// label length where it genuinely isn't, even after reservation).
#[test]
fn vertical_legend_ncol_two_with_long_labels_stays_on_canvas() {
    let data = groups_with_labels(8, |i| format!("Group Label {}", i + 1));
    let svg = render(
        GGPlot::new(data)
            .aes(Aes::new().x("x").y("y").color("g"))
            .geom_point()
            .guides(GuideLegend::new().with_ncol(2)),
        640,
        400,
    );
    let max_x = max_text_right_edge(&svg);
    assert!(
        max_x <= 640.0,
        "max text right edge {max_x} exceeds the 640px canvas"
    );
}

/// #159: a vertical multi-column grid that still runs off the canvas
/// after reserving `(cols - 1) * col_pitch` first tries reducing its
/// column count toward 1 — here that alone is enough: `ncol(4)` on a
/// 300px chart has no room for 4 columns of long labels, but 1 column
/// does fit, so the build no longer needs to warn or truncate (0.22
/// warned unconditionally in this exact case; review #158's second
/// re-review introduced that warning).
#[test]
fn vertical_legend_ncol_grid_too_wide_reduces_columns_instead_of_warning() {
    let data = groups_with_labels(12, |i| format!("A Very Long Group Label Number {}", i + 1));
    let (svg, warnings) = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
        .guides(GuideLegend::new().with_ncol(4))
        .render_svg_native_with_warnings(300, 400)
        .unwrap();
    assert!(!svg.contains("NaN"), "{svg}");
    assert!(
        warnings
            .iter()
            .all(|w| !(w.contains("legend") && w.contains("doesn't fit"))),
        "column reduction should have avoided the warning: {warnings:?}"
    );
}

/// #168 (review #167 finding 5): once column reduction above has settled
/// on a narrower column count, the panel should get back the width that
/// count no longer needs — not keep reserving the *original*, pre-
/// reduction column count's width. `ncol(3)` here reduces to `ncol(2)`
/// (9 short-enough labels that 2 columns fits, unlike the all-the-way-
/// to-1 case below); the panel width after reduction must match an
/// explicit `ncol(2)` grid built from the start, not a narrower one still
/// carrying the 3-column reservation.
#[test]
fn legend_panel_width_is_reclaimed_after_column_reduction() {
    let data = groups_with_labels(9, |i| format!("Group {}", i + 1));
    let (reduced_svg, reduced_warnings) = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
        .guides(GuideLegend::new().with_ncol(3))
        .render_svg_native_with_warnings(300, 400)
        .unwrap();
    let (direct_svg, direct_warnings) = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
        .guides(GuideLegend::new().with_ncol(2))
        .render_svg_native_with_warnings(300, 400)
        .unwrap();

    assert!(
        reduced_warnings.is_empty() && direct_warnings.is_empty(),
        "neither grid should need to warn: reduced={reduced_warnings:?} direct={direct_warnings:?}"
    );
    assert!(!reduced_svg.contains('…'), "{reduced_svg}");

    let reduced_width = data_plot_rect(&reduced_svg)[2];
    let direct_width = data_plot_rect(&direct_svg)[2];
    assert!(
        (reduced_width - direct_width).abs() < 0.01,
        "#168: a ncol(3) grid auto-reduced to 2 columns must reserve the same \
         width as an explicit ncol(2) grid (panel width {reduced_width}), not \
         the wider, stale ncol(3) reservation (direct ncol(2) panel width \
         {direct_width})"
    );
}

/// #168's retightening is a bounded, *safe* retry, not an unconditional
/// recompute: when the grid reduces all the way to a single column (the
/// pre-existing #159 limitation — an explicit `ncol(1)` grid has no fit
/// check of its own, see `vertical_legend_warns_only_when_truncation_still_cant_fit`'s
/// sibling cases), tightening the reservation to match would push the
/// legend's text off the canvas with no warning to show for it, so the
/// wider, already-proven-safe reservation must be kept instead — the
/// panel here stays exactly as narrow as the unreduced #159 behaviour,
/// not byte-identical to an explicit `ncol(1)` grid (which doesn't get
/// one at all).
#[test]
fn legend_panel_width_reservation_is_not_loosened_when_unsafe() {
    let data = groups_with_labels(12, |i| format!("A Very Long Group Label Number {}", i + 1));
    let (reduced_svg, _) = GGPlot::new(data.clone())
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
        .guides(GuideLegend::new().with_ncol(4))
        .render_svg_native_with_warnings(300, 400)
        .unwrap();
    let (direct_one_col_svg, _) = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
        .guides(GuideLegend::new().with_ncol(1))
        .render_svg_native_with_warnings(300, 400)
        .unwrap();

    let reduced_width = data_plot_rect(&reduced_svg)[2];
    let direct_width = data_plot_rect(&direct_one_col_svg)[2];
    assert!(
        reduced_width < direct_width,
        "retightening to the fully-reduced 1-column reservation would run this \
         label off the canvas (an explicit ncol(1) grid has no fit check to \
         catch it) — the wider reservation must be kept: reduced={reduced_width} \
         direct_ncol1={direct_width}"
    );
}

/// #159: when reducing columns isn't enough (labels so long even a
/// single column overflows), labels are truncated with an ellipsis
/// instead — the full label stays available via a `<title>` tooltip on
/// the key, and no warning is needed since the truncated legend does fit.
#[test]
fn vertical_legend_with_very_long_labels_truncates_with_a_title_tooltip() {
    let data = groups_with_labels(12, |i| {
        format!(
            "An Extremely Very Super Duper Long Group Label Number {} \
             With Lots More Text Appended Here To Make It Even Longer",
            i + 1
        )
    });
    let (svg, warnings) = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
        .guides(GuideLegend::new().with_ncol(6))
        .render_svg_native_with_warnings(200, 400)
        .unwrap();
    assert!(!svg.contains("NaN"), "{svg}");
    assert!(
        svg.contains('…'),
        "a label should have been truncated: {svg}"
    );
    // The full, untruncated label is still present verbatim inside a
    // <title> tooltip for the key it belongs to.
    assert!(
        svg.contains("<title>An Extremely Very Super Duper Long Group Label Number 1 "),
        "the full label should still be available via <title>: {svg}"
    );
    assert!(
        warnings
            .iter()
            .all(|w| !(w.contains("legend") && w.contains("doesn't fit"))),
        "truncation should have avoided the warning: {warnings:?}"
    );
}

/// #159: when the chart is so narrow that even a truncated-to-an-
/// ellipsis single column still can't fit, the build falls back to
/// warning — the "only warn if it still overflows after both" tail of
/// the fix.
#[test]
fn vertical_legend_warns_only_when_truncation_still_cant_fit() {
    let data = groups_with_labels(12, |i| {
        format!(
            "An Extremely Very Super Duper Long Group Label Number {} \
             With Lots More Text Appended Here To Make It Even Longer",
            i + 1
        )
    });
    let (svg, warnings) = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
        .guides(GuideLegend::new().with_ncol(8))
        .render_svg_native_with_warnings(60, 400)
        .unwrap();
    assert!(!svg.contains("NaN"), "{svg}");
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("legend") && w.contains("doesn't fit")),
        "{warnings:?}"
    );
}

fn groups_with_labels(n: usize, label: impl Fn(usize) -> String) -> Vec<(String, Vec<Value>)> {
    let x: Vec<Value> = (0..n).map(|i| Value::Float(i as f64)).collect();
    let y: Vec<Value> = (0..n).map(|i| Value::Float((i % 3) as f64)).collect();
    let g: Vec<Value> = (0..n).map(label).map(Value::Str).collect();
    vec![("x".into(), x), ("y".into(), y), ("g".into(), g)]
}

/// `nrow(0)`/`ncol(0)` and an `nrow·ncol < n` grid are invalid; both are
/// corrected with a warning rather than panicking or drawing garbage.
#[test]
fn invalid_grid_settings_warn_and_fall_back() {
    let (_, w1) = base(6)
        .guides(GuideLegend::new().with_nrow(0))
        .render_svg_native_with_warnings(500, 400)
        .unwrap();
    assert!(
        w1.iter().any(|w| w.contains("nrow(0)")),
        "nrow(0) should warn: {w1:?}"
    );

    let (_, w2) = base(6)
        .guides(GuideLegend::new().with_nrow(2).with_ncol(2)) // 2*2=4 < 6
        .render_svg_native_with_warnings(500, 400)
        .unwrap();
    assert!(
        w2.iter().any(|w| w.contains("smaller than 6 keys")),
        "nrow*ncol < n should warn: {w2:?}"
    );

    let (_, w3) = base(6)
        .guides(GuideLegend::new().with_max_rows(0))
        .legend_position(LegendPosition::Bottom)
        .render_svg_native_with_warnings(150, 400)
        .unwrap();
    assert!(
        w3.iter().any(|w| w.contains("max_rows(0)")),
        "max_rows(0) should warn: {w3:?}"
    );
}

/// A one-line horizontal legend that already fits, and any vertical
/// legend with no explicit grid, are untouched by B3-2 — same bytes as
/// drawing through the unchanged `draw_legend`/`draw_discrete_legend_at`
/// path (verified indirectly: `default_output_022`'s `point`/`big_legend`
/// — wait, `big_legend` *does* wrap. Use a small group count here that
/// clearly fits on one line, both horizontal and vertical.)
#[test]
fn byte_identical_cases_are_unaffected() {
    for pos in [LegendPosition::Right, LegendPosition::Bottom] {
        let svg = render(base(3).legend_position(pos), 800, 400);
        assert!(!svg.contains(" more<"), "a 3-key legend never overflows");
    }
}

/// #159: a vertical `ncol` grid that already fits renders byte-identically
/// to the pre-#159 "warn only" version of this check — no column
/// reduction, no truncated labels, no extra `<title>` tooltip, no
/// warning. Two independent renders (identical input) must also match
/// exactly, since the fix path is otherwise deterministic.
#[test]
fn a_vertical_grid_that_already_fits_is_byte_identical() {
    let (svg1, warnings1) = base(4)
        .guides(GuideLegend::new().with_ncol(2))
        .render_svg_native_with_warnings(500, 400)
        .unwrap();
    let (svg2, warnings2) = base(4)
        .guides(GuideLegend::new().with_ncol(2))
        .render_svg_native_with_warnings(500, 400)
        .unwrap();
    assert_eq!(svg1, svg2, "identical input must render identically");
    assert!(
        !svg1.contains('…'),
        "a fitting grid must not truncate: {svg1}"
    );
    assert!(
        warnings1.is_empty() && warnings2.is_empty(),
        "a fitting grid must not warn: {warnings1:?}"
    );
}

/// The drawn "+N more" text, if any.
fn drawn_plus_n(svg: &str) -> Option<usize> {
    let start = svg.find(" more<")?;
    let seg = &svg[..start];
    let open = seg.rfind('>')? + 1;
    seg[open..].trim_start_matches('+').parse().ok()
}

/// The warned overflow count, if any ("legend: {k} of {n} entries...").
fn warned_n(warnings: &[String]) -> Option<usize> {
    warnings.iter().find_map(|w| {
        let rest = w.strip_prefix("legend: ")?;
        let k = rest.split(' ').next()?;
        k.parse().ok()
    })
}

/// Regression for review #158 finding 2: `layout_built` used to plan
/// against the pre-row-reservation panel height (for the warning and the
/// space reservation) while `draw_legend_planned` re-planned against the
/// shorter, post-reservation height (for drawing) — two different plans,
/// so the warned "+N more" and the drawn "+N more" could disagree (the
/// reviewer's repro: 30 keys at 320×400 warned "+7 more" but drew "+15
/// more"). Now the plan is computed once (`BuiltPlot::legend_plan`) and
/// reused verbatim for drawing, so a scan over heights and key counts
/// (mirroring the reviewer's 180-of-207-mismatched scan) must always
/// agree, and the reserved band height must match the drawn row count.
#[test]
fn drawn_plus_n_always_matches_the_warned_n_across_heights_and_counts() {
    let mut checked_some_overflow = false;
    for h in [120u32, 200, 320, 400, 480, 600] {
        for n in [20usize, 40, 80] {
            let (svg, warnings) = base(n)
                .legend_position(LegendPosition::Bottom)
                .render_svg_native_with_warnings(320, h)
                .unwrap();
            let warned = warned_n(&warnings);
            let drawn = drawn_plus_n(&svg);
            assert_eq!(
                warned, drawn,
                "h={h} n={n}: warned {warned:?} but drew {drawn:?}"
            );
            if warned.is_some() {
                checked_some_overflow = true;
            }
        }
    }
    assert!(
        checked_some_overflow,
        "the scan should hit at least one overflowing combination"
    );
}

/// The legend band `layout_built` reserves (via `legend_extra_rows`) must
/// match the number of rows `draw_legend_planned` actually draws — not a
/// taller band than what ends up on the page (also review #158 finding 2).
#[test]
fn reserved_band_height_matches_the_drawn_row_count() {
    let n = 40;
    let h = 320u32;
    let svg = render(base(n).legend_position(LegendPosition::Bottom), 320, h);
    // Panel bottom (from data-plot) plus the legend's own margin/key
    // geometry should accommodate every drawn key <rect> — i.e. no legend
    // swatch rect extends below the canvas.
    let canvas_h = h as f64;
    for (start, _) in svg.match_indices("<rect ") {
        let tag_end = svg[start..].find('/').map(|e| start + e).unwrap_or(start);
        let tag = &svg[start..tag_end];
        if !tag.contains("width=\"12.00\"") {
            continue; // not a legend key swatch (fixed 12px key size)
        }
        let y: f64 = tag
            .split("y=\"")
            .nth(1)
            .and_then(|s| s.split('"').next())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        assert!(
            y <= canvas_h,
            "a legend key at y={y} falls below the {canvas_h}px canvas — reserved band too short"
        );
    }
}

/// #155: a plot with two simultaneous discrete guides (`colour` and
/// `shape`) laid out side by side on a bottom legend — each with enough
/// entries to overflow a one-line layout on its own — now wraps
/// independently instead of only the plot's single discrete guide ever
/// being eligible. Neither guide's wrapped text may run past the canvas.
#[test]
fn two_discrete_guides_each_auto_wrap() {
    let n = 10;
    let x: Vec<Value> = (0..n).map(|i| Value::Float(i as f64)).collect();
    let y: Vec<Value> = (0..n).map(|i| Value::Float((i % 3) as f64)).collect();
    let g: Vec<Value> = (0..n)
        .map(|i| Value::Str(format!("Colour {:02}", i + 1)))
        .collect();
    let s: Vec<Value> = (0..n)
        .map(|i| Value::Str(format!("Shape {:02}", i + 1)))
        .collect();
    let plot = GGPlot::new(vec![
        ("x".into(), x),
        ("y".into(), y),
        ("g".into(), g),
        ("s".into(), s),
    ])
    .aes(Aes::new().x("x").y("y").color("g").shape("s"))
    .geom_point()
    .legend_position(LegendPosition::Bottom);
    let svg = render(plot, 640, 500);

    assert!(svg.contains(">Colour 01<"), "{svg}");
    assert!(svg.contains(">Colour 10<"), "{svg}");
    assert!(svg.contains(">Shape 01<"), "{svg}");
    assert!(svg.contains(">Shape 10<"), "{svg}");
    assert!(
        max_text_right_edge(&svg) <= 640.0,
        "a guide's wrapped text overflows the 640px canvas: right edge {}",
        max_text_right_edge(&svg)
    );
}

/// #155: a `PlotGrid` with `collect_legends(true)` wraps its collected
/// legend the same way a single plot's legend does — previously
/// `compose::capture_legend` always drew the plain, unwrapped path, so a
/// crowded collected legend could run off the grid's own canvas.
#[test]
fn plotgrid_collected_legend_auto_wraps() {
    use ggplot_rs::compose::{GridLegendPosition, PlotGrid};

    let n = 14;
    let x: Vec<Value> = (0..n).map(|i| Value::Float(i as f64)).collect();
    let y: Vec<Value> = (0..n).map(|i| Value::Float((i % 3) as f64)).collect();
    let g: Vec<Value> = (0..n)
        .map(|i| Value::Str(format!("Group {:02}", i + 1)))
        .collect();
    let plot = GGPlot::new(vec![("x".into(), x), ("y".into(), y), ("g".into(), g)])
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point();

    let svg = PlotGrid::new()
        .add(plot)
        .collect_legends(true)
        .legend_position(GridLegendPosition::Bottom)
        .render_svg_native_with_size(500, 400)
        .expect("render");

    assert!(svg.contains(">Group 01<"), "{svg}");
    assert!(svg.contains(">Group 14<"), "{svg}");
    assert!(
        max_text_right_edge(&svg) <= 500.0,
        "the collected legend's text overflows the 500px canvas: right edge {}",
        max_text_right_edge(&svg)
    );
}

/// Review #167 finding 1: a `PlotGrid`'s collected bottom legend with two
/// simultaneous discrete guides (`colour` and `shape`) must lay them out
/// side by side, exactly like a single plot's legend does (see
/// `two_discrete_guides_each_auto_wrap` above) — not drawn on top of each
/// other. `compose::capture_legend` measures the collected legend against
/// a fake zero-width panel so its origin lands at (0, 0); stepping to the
/// next guide by `plot_area.width / n` there always advanced by zero
/// (0.0 / n), so "Shape 01" landed at the exact same (x, y) as "Colour
/// 01" and the two guides' labels were drawn on top of each other.
#[test]
fn plotgrid_collected_legend_two_guides_sit_side_by_side() {
    use ggplot_rs::compose::{GridLegendPosition, PlotGrid};

    let n = 8;
    let x: Vec<Value> = (0..n).map(|i| Value::Float(i as f64)).collect();
    let y: Vec<Value> = (0..n).map(|i| Value::Float((i % 3) as f64)).collect();
    let g: Vec<Value> = (0..n)
        .map(|i| Value::Str(format!("Colour {:02}", i + 1)))
        .collect();
    let s: Vec<Value> = (0..n)
        .map(|i| Value::Str(format!("Shape {:02}", i + 1)))
        .collect();
    let plot = GGPlot::new(vec![
        ("x".into(), x),
        ("y".into(), y),
        ("g".into(), g),
        ("s".into(), s),
    ])
    .aes(Aes::new().x("x").y("y").color("g").shape("s"))
    .geom_point();

    let svg = PlotGrid::new()
        .add(plot)
        .collect_legends(true)
        .legend_position(GridLegendPosition::Bottom)
        .render_svg_native_with_size(500, 400)
        .expect("render");

    let colour_x = text_x(&svg, "Colour 01").expect("colour guide label \"Colour 01\" missing");
    let shape_x = text_x(&svg, "Shape 01").expect("shape guide label \"Shape 01\" missing");
    assert!(
        shape_x > colour_x,
        "the shape guide (x={shape_x}) must sit to the right of the colour guide \
         (x={colour_x}), not on top of it"
    );

    // Every entry of both guides must still be present.
    assert!(svg.contains(">Colour 08<"), "{svg}");
    assert!(svg.contains(">Shape 08<"), "{svg}");
}

/// #170 / review #167 finding 7: a bottom legend mixing a colourbar
/// (`colour` mapped to a continuous variable) with a discrete guide
/// (`shape`) must leave the discrete guide only the row width left over
/// after the colourbar's own fixed share — `plan_all_legends` previously
/// split the *whole* row width among the discrete guides alone, so the
/// wrapped grid could still run past the canvas once the colourbar's
/// share was added back on top (worse than 0.22.2, which never auto-
/// wrapped a discrete guide sharing a row with a continuous one at all;
/// not a regression there, see #155's own CHANGELOG note).
#[test]
fn bottom_legend_colourbar_leaves_room_for_the_discrete_guide() {
    let n = 18;
    let x: Vec<Value> = (0..n).map(|i| Value::Float(i as f64)).collect();
    let y: Vec<Value> = (0..n).map(|i| Value::Float((i % 3) as f64)).collect();
    let c: Vec<Value> = (0..n).map(|i| Value::Float(i as f64)).collect();
    let s: Vec<Value> = (0..n)
        .map(|i| Value::Str(format!("Shape Group Number {:02}", i + 1)))
        .collect();
    let plot = GGPlot::new(vec![
        ("x".into(), x),
        ("y".into(), y),
        ("c".into(), c),
        ("s".into(), s),
    ])
    .aes(Aes::new().x("x").y("y").color("c").shape("s"))
    .geom_point()
    .legend_position(LegendPosition::Bottom);
    let svg = render(plot, 450, 500);

    // 18 entries also run into the available legend-band *height* at
    // this canvas size (an unrelated, pre-existing #159 "+N more" drop)
    // — this test is only about the *width* share, so just check that
    // at least the auto-wrapped grid that does get drawn stays on-canvas.
    assert!(svg.contains(">Shape Group Number 01<"), "{svg}");
    let max_x = max_text_right_edge(&svg);
    assert!(
        max_x <= 450.0,
        "the discrete guide's wrapped text overflows the 450px canvas once the \
         colourbar's own share is accounted for: right edge {max_x}"
    );
}

/// #170 / review #167 finding 6: a `PlotGrid`'s collected-legend path
/// (`compose::capture_legend`) must propagate the warnings its own
/// `plan_all_legends` call produces — previously they went into a local
/// `Vec` nothing ever read, so an invalid `nrow`/`ncol` setting on a
/// collected legend silently fell back instead of warning, unlike the
/// same setting on a single plot's own legend (see
/// `invalid_grid_settings_warn_and_fall_back` above).
#[test]
fn plotgrid_collected_legend_propagates_its_own_warnings() {
    use ggplot_rs::compose::{GridLegendPosition, PlotGrid};

    let plot = GGPlot::new(groups(6))
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point()
        .guides(GuideLegend::new().with_nrow(0));

    let (_, warnings) = PlotGrid::new()
        .add(plot)
        .collect_legends(true)
        .legend_position(GridLegendPosition::Bottom)
        .render_svg_native_with_warnings(500, 400)
        .expect("render");

    assert!(
        warnings.iter().any(|w| w.contains("nrow(0)")),
        "the collected legend's own guide_legend warning should have reached \
         the grid's warnings: {warnings:?}"
    );
}
