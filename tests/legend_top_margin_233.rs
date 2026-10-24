//! #233: `LegendPosition::Top` used to draw from an absolute canvas
//! `y = legend_margin.top`, ignoring the title/subtitle/outer-margin band
//! `PlotLayout` reserves above the panel for it — with a title, a
//! subtitle or a non-zero plot margin, the legend drew over them instead
//! of into its reserved band. `legend_position`'s `Top` arm now backs off
//! from `plot_area.y` by the band's own content instead, and the title/
//! subtitle drawing in `renderer.rs` lifts itself above that same band
//! (mirroring its existing `x_axis_lift` for a top x-axis) so neither
//! overlaps the other.

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

/// `plot_area` as `[x, y, width, height]`, from the root `data-plot` attribute.
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

/// The `y` of a `<text ...>needle</text>` element — the first one found.
fn text_y(svg: &str, needle: &str) -> f64 {
    let marker = svg.find(&format!(">{needle}<")).unwrap_or_else(|| {
        panic!("no <text>{needle}</text> in {svg}");
    });
    let tag_start = svg[..marker].rfind("<text").expect("its <text> start");
    let tag = &svg[tag_start..marker];
    let y_start = tag.find("y=\"").unwrap() + 3;
    let y_end = tag[y_start..].find('"').unwrap() + y_start;
    tag[y_start..y_end].parse().unwrap()
}

/// The `y` of the first legend key swatch (a `legend_key_width`-square
/// rect, the default theme's 12×12) — marks exactly where
/// `legend_position` placed the legend's first row, independent of
/// whether it also draws its own title line (the scale's `name()`, empty
/// by default here, so it doesn't).
fn legend_first_row_y(svg: &str) -> f64 {
    let marker = svg
        .find("width=\"12.00\" height=\"12.00\"")
        .expect("a legend key swatch rect");
    let tag_start = svg[..marker].rfind("<rect").expect("its <rect> start");
    let tag = &svg[tag_start..marker];
    let y_start = tag.find("y=\"").unwrap() + 3;
    let y_end = tag[y_start..].find('"').unwrap() + y_start;
    tag[y_start..y_end].parse().unwrap()
}

fn two_point_data() -> Vec<(String, Vec<Value>)> {
    frame(vec![
        ("x", floats(&[1.0, 2.0])),
        ("y", floats(&[1.0, 2.0])),
        ("grp", strs(&["a", "b"])),
    ])
}

fn top_legend_plot(data: Vec<(String, Vec<Value>)>) -> GGPlot {
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("grp"))
        .geom_point()
        .legend_position(LegendPosition::Top)
}

#[test]
fn top_legend_with_no_title_sits_above_the_panel() {
    let svg = top_legend_plot(two_point_data())
        .render_svg_native_with_size(400, 300)
        .unwrap();
    let legend_y = legend_first_row_y(&svg);
    let plot_area = data_plot_rect(&svg);
    assert!(
        legend_y > 0.0 && legend_y < plot_area[1],
        "legend (y={legend_y}) must sit on-canvas, above the panel (plot_area.y={})",
        plot_area[1]
    );
}

#[test]
fn top_legend_with_a_title_sits_below_it_not_overlapping() {
    let svg = top_legend_plot(two_point_data())
        .title("My Title")
        .render_svg_native_with_size(400, 300)
        .unwrap();
    let title_y = text_y(&svg, "My Title");
    let legend_y = legend_first_row_y(&svg);
    let plot_area = data_plot_rect(&svg);
    assert!(
        title_y < legend_y,
        "title (y={title_y}) must render above the legend (y={legend_y}) — #233's bug put the \
         legend above the title instead"
    );
    assert!(
        legend_y < plot_area[1],
        "legend (y={legend_y}) must sit above the panel (plot_area.y={})",
        plot_area[1]
    );
}

#[test]
fn top_legend_with_a_title_and_subtitle_sits_below_both() {
    let svg = top_legend_plot(two_point_data())
        .title("My Title")
        .subtitle("My Subtitle")
        .render_svg_native_with_size(400, 300)
        .unwrap();
    let title_y = text_y(&svg, "My Title");
    let subtitle_y = text_y(&svg, "My Subtitle");
    let legend_y = legend_first_row_y(&svg);
    let plot_area = data_plot_rect(&svg);
    assert!(title_y < subtitle_y, "title must render above the subtitle");
    assert!(
        subtitle_y < legend_y,
        "subtitle (y={subtitle_y}) must render above the legend (y={legend_y})"
    );
    assert!(
        legend_y < plot_area[1],
        "legend (y={legend_y}) must sit above the panel (plot_area.y={})",
        plot_area[1]
    );
}

#[test]
fn top_legend_shifts_down_with_a_larger_plot_margin() {
    // Increasing plot_margin.top by Δ must shift the legend down by the
    // same Δ — #233's bug (an absolute `legend_margin.top`) ignored the
    // plot's own outer margin entirely, so this would previously fail
    // (the legend wouldn't move at all).
    let small_margin = Theme::default().set_plot_margin(Margin {
        top: 10.0,
        right: 10.0,
        bottom: 10.0,
        left: 10.0,
    });
    let large_margin = Theme::default().set_plot_margin(Margin {
        top: 60.0,
        right: 10.0,
        bottom: 10.0,
        left: 10.0,
    });
    let svg_small = top_legend_plot(two_point_data())
        .theme(small_margin)
        .render_svg_native_with_size(400, 300)
        .unwrap();
    let svg_large = top_legend_plot(two_point_data())
        .theme(large_margin)
        .render_svg_native_with_size(400, 300)
        .unwrap();
    let y_small = legend_first_row_y(&svg_small);
    let y_large = legend_first_row_y(&svg_large);
    assert!(
        (y_large - y_small - 50.0).abs() < 0.5,
        "a 50px larger plot_margin.top should shift the legend down by the same 50px: \
         y_small={y_small} y_large={y_large}"
    );
}

#[test]
fn a_wrapped_multi_row_top_legend_keeps_its_first_row_in_place_and_grows_the_panel_down() {
    // Many groups force the horizontal auto-wrap (D7) onto 2+ rows on a
    // narrow canvas — mirrors legend_wrap.rs's existing Bottom-position
    // convention (`overflowing_bottom_legend_auto_wraps_and_grows_legend_rows`
    // / `the_first_entry_is_unaffected_by_wrapping`), checked for `Top`.
    const W: u32 = 420;
    const H: u32 = 400;
    let one_row_data = frame(vec![
        ("x", floats(&[1.0, 2.0, 3.0])),
        ("y", floats(&[1.0, 2.0, 3.0])),
        ("grp", strs(&["a", "b", "c"])),
    ]);
    let x12: Vec<Value> = (0..12).map(|i| Value::Float(i as f64)).collect();
    let y12: Vec<Value> = (0..12).map(|i| Value::Float((i % 3) as f64)).collect();
    let g12: Vec<Value> = (0..12)
        .map(|i| Value::Str(format!("Group {:02}", i + 1)))
        .collect();
    let many_rows_data = frame(vec![("x", x12), ("y", y12), ("grp", g12)]);

    let one_row = top_legend_plot(one_row_data)
        .render_svg_native_with_size(W, H)
        .unwrap();
    let many_rows = top_legend_plot(many_rows_data)
        .render_svg_native_with_size(W, H)
        .unwrap();

    let one_row_plot_area = data_plot_rect(&one_row);
    let many_rows_plot_area = data_plot_rect(&many_rows);

    // The panel starts further down once the legend needs a second row.
    assert!(
        many_rows_plot_area[1] > one_row_plot_area[1],
        "panel should start lower once the legend wraps onto more rows: \
         one_row_y={} many_rows_y={}",
        one_row_plot_area[1],
        many_rows_plot_area[1]
    );

    // The first row's own position is unaffected by how many extra rows
    // follow it.
    let one_row_legend_y = legend_first_row_y(&one_row);
    let many_rows_legend_y = legend_first_row_y(&many_rows);
    assert!(
        (one_row_legend_y - many_rows_legend_y).abs() < 0.5,
        "the first legend row must sit at the same y whether or not the legend wraps: \
         one_row={one_row_legend_y} many_rows={many_rows_legend_y}"
    );

    // And it must still sit above the (now-lower) panel.
    assert!(
        many_rows_legend_y < many_rows_plot_area[1],
        "legend (y={many_rows_legend_y}) must sit above the panel (plot_area.y={})",
        many_rows_plot_area[1]
    );
}
