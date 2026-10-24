//! #239: ggplot2 merges `fill` and `colour` guides mapped to the same
//! measure, through the same scale spec, into one legend with a
//! combined key — `guide/legend.rs` used to iterate aesthetics
//! independently, so the same legend appeared twice (or, in one specific
//! case, `legend_scales_for` unconditionally dropped `Fill`'s legend
//! whenever `Color` was mapped too, even to an unrelated variable with a
//! different title/breaks — the opposite bug). `group_mergeable_legends`
//! groups aesthetics whose legend would render identically (same
//! effective title, same discrete/continuous-ness, identical `breaks()`)
//! into one combined guide instead.

use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;
use ggplot_rs::scale::binned::ScaleFillBinned;

fn floats(v: &[f64]) -> Vec<Value> {
    v.iter().map(|&f| Value::Float(f)).collect()
}
fn strs(v: &[&str]) -> Vec<Value> {
    v.iter().map(|s| Value::Str(s.to_string())).collect()
}
fn frame(cols: Vec<(&str, Vec<Value>)>) -> Vec<(String, Vec<Value>)> {
    cols.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
}

/// How many legend-key boxes the SVG drew: `theme.legend_background` is
/// a white, strokeless rect (`fill="#FFFFFF" stroke="none"
/// stroke-width="0.00"`), one per legend drawn — the *only* other thing
/// matching that exact attribute combination is the plot background
/// itself (always exactly one, at `x="0.00" y="0.00"`, the full canvas
/// size; the panel background is a different colour, `#EBEBEB`), so
/// subtracting that one out leaves the legend count.
fn legend_box_count(svg: &str, canvas_w: u32, canvas_h: u32) -> usize {
    let plot_bg = format!(
        "x=\"0.00\" y=\"0.00\" width=\"{canvas_w}.00\" height=\"{canvas_h}.00\" \
         fill=\"#FFFFFF\" stroke=\"none\" stroke-width=\"0.00\""
    );
    assert!(svg.contains(&plot_bg), "no plot background found: {svg}");
    svg.matches("fill=\"#FFFFFF\" stroke=\"none\" stroke-width=\"0.00\"")
        .count()
        - 1
}

#[test]
fn fill_and_colour_mapped_to_the_same_scale_merge_into_one_legend() {
    const W: u32 = 400;
    const H: u32 = 300;
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0])),
        ("y", floats(&[1.0, 2.0])),
        ("g", strs(&["a", "b"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("g").fill("g"))
        .geom_point()
        .render_svg_native_with_size(W, H)
        .unwrap();
    assert_eq!(
        legend_box_count(&svg, W, H),
        1,
        "fill and colour mapped to the same scale should merge into one legend: {svg}"
    );
    // Both aesthetics' glyphs are still drawn, at each key: one pair of
    // overlaid (fill, then colour) swatch rects per break (width 12,
    // matching `theme.legend_key_width`), under each key's own key-bg rect.
    assert_eq!(
        svg.matches("width=\"12.00\" height=\"12.00\"").count(),
        // 2 breaks x (1 key-bg rect + 2 overlaid swatch rects) = 6
        6,
        "{svg}"
    );
}

#[test]
fn fill_and_colour_with_different_variables_give_two_legends() {
    const W: u32 = 400;
    const H: u32 = 300;
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0])),
        ("y", floats(&[1.0, 2.0])),
        ("g1", strs(&["a", "b"])),
        ("g2", strs(&["x", "y"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("g1").fill("g2"))
        .geom_point()
        .render_svg_native_with_size(W, H)
        .unwrap();
    assert_eq!(
        legend_box_count(&svg, W, H),
        2,
        "colour and fill mapped to different variables (so different \
         breaks/labels) must draw two separate legends, not merge and \
         not drop one: {svg}"
    );
    // Both variables' own labels are present — #239's predecessor bug
    // (an unconditional `legend_scales_for` rule) silently dropped fill's
    // legend (and so its "x"/"y" labels) whenever colour was mapped too.
    assert!(svg.contains(">a<") && svg.contains(">b<"), "{svg}");
    assert!(svg.contains(">x<") && svg.contains(">y<"), "{svg}");
}

#[test]
fn an_explicit_shared_guide_title_still_merges_fill_and_colour() {
    // `GuideLegend` is one shared config for the whole legend system
    // (not one per aesthetic), so this plot can't actually give fill and
    // colour *different* explicit titles through the public API — this
    // just confirms the merge still finds them identical (both titled
    // the same way) and still applies when a title is set explicitly,
    // not only when it falls back to the (empty, by default) scale name.
    const W: u32 = 400;
    const H: u32 = 300;
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0])),
        ("y", floats(&[1.0, 2.0])),
        ("g", strs(&["a", "b"])),
    ]);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("g").fill("g"))
        .geom_point()
        .guides(GuideLegend {
            title: Some("Colour title".to_string()),
            ..Default::default()
        })
        .render_svg_native_with_size(W, H)
        .unwrap();
    // A single shared `GuideLegend` config applies to the whole legend
    // system here (not per-aesthetic), so this plot can't actually give
    // fill/colour *different* titles through the public API — confirming
    // that instead: the single shared title still merges them into one.
    assert_eq!(legend_box_count(&svg, W, H), 1, "{svg}");
    assert!(svg.contains("Colour title"), "{svg}");
}

#[test]
fn binned_fill_and_colour_scales_with_matching_breaks_merge_with_their_counts() {
    const W: u32 = 400;
    const H: u32 = 300;
    let data = frame(vec![
        ("x", floats(&[1.0, 2.0, 3.0])),
        ("y", floats(&[1.0, 2.0, 3.0])),
        ("v", floats(&[1.0, 5.0, 9.0])),
    ]);
    let breaks = vec![0.0, 4.0, 8.0, 12.0];
    let low = (0, 0, 255);
    let high = (255, 0, 0);
    let fill_scale = ScaleFillBinned::two(Aesthetic::Fill, breaks.clone(), low, high);
    let color_scale = ScaleFillBinned::two(Aesthetic::Color, breaks, low, high);
    let svg = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").color("v").fill("v"))
        .geom_point()
        .scale_fill(fill_scale)
        .scale_color(color_scale)
        .render_svg_native_with_size(W, H)
        .unwrap();
    assert_eq!(
        legend_box_count(&svg, W, H),
        1,
        "matching binned fill/colour scales should merge into one legend: {svg}"
    );
    // Binned labels carry their bin's count in parentheses.
    assert!(
        svg.contains('(') && svg.contains(')'),
        "expected a count-bearing binned label: {svg}"
    );
}
