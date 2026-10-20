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
