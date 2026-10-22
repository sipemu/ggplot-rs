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

/// Regression for review #158's second re-review (LOW): when a vertical
/// multi-column grid is simply too wide for the canvas — many columns,
/// long labels, a narrow chart — even after reserving `(cols - 1) *
/// col_pitch`, it still runs off the canvas; the build must warn about
/// it rather than silently clip.
#[test]
fn vertical_legend_ncol_grid_too_wide_for_the_canvas_warns() {
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
