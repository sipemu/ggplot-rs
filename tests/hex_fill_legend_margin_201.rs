//! #201: a `Fill` legend mapped only on a *layer* (not the plot-level
//! `.aes(...)`) — `geom_hex`'s usual pattern, since its fill is a
//! stat-computed column pre-binned by the consumer and mapped with
//! `.layer_aes(...).fill(...)` + `StatIdentity` — got no panel-margin
//! reservation at all. The legend still drew, with correct colours, but
//! ran straight past the canvas edge: on a narrow panel that reads as
//! "overlapping the plot", more generally as a colourbar clipped off
//! entirely. A *second* layer (even one that doesn't map `fill` at all,
//! e.g. `geom_abline`) made it worse: found via DataZooDE/anofox-visualization#439.

use ggplot_rs::prelude::*;
use ggplot_rs::scale::color::{RGBAColor, ScaleColorContinuous};
use ggplot_rs::scale::Scale;

const W: u32 = 640;
const H: u32 = 400;

fn render(plot: GGPlot) -> String {
    plot.render_svg_native_with_size(W, H)
        .expect("native svg render")
}

/// `[x0, y0, x1, y1]` of the plotted data area (`data-plot`, pixels).
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

/// The legend's background `<rect>` (drawn right after its `>count<`
/// title), as `(x, width)`.
fn legend_rect(svg: &str) -> (f64, f64) {
    let at = svg.find(">count<").expect("legend title");
    let after = &svg[at..];
    let rect_start = after.find("<rect").expect("a rect after the legend title");
    let tag_end = after[rect_start..].find('>').unwrap() + rect_start;
    let tag = &after[rect_start..tag_end];
    let attr = |name: &str| -> f64 {
        let k = tag.find(&format!(" {name}=\"")).unwrap() + name.len() + 3;
        tag[k..k + tag[k..].find('"').unwrap()].parse().unwrap()
    };
    (attr("x"), attr("width"))
}

fn gradient() -> ScaleColorContinuous {
    let mut s = ScaleColorContinuous::new(ggplot_rs::aes::Aesthetic::Fill).with_colors(
        RGBAColor {
            r: 230,
            g: 235,
            b: 250,
            a: 1.0,
        },
        RGBAColor {
            r: 20,
            g: 50,
            b: 160,
            a: 1.0,
        },
    );
    s.set_name("count");
    s
}

/// A pre-binned hex frame: `x`, `y`, `count`, `width`, `height` — the shape
/// `StatBinHex::compute_group` produces, fed back through `GeomHex` with
/// `StatIdentity` (the pattern a consumer uses when it pre-bins itself,
/// e.g. to pick its own bin count or reuse a fixed lattice).
fn hex_frame() -> Vec<(String, Vec<Value>)> {
    let n = 12;
    let x: Vec<Value> = (0..n).map(|i| Value::Float((i * 7) as f64)).collect();
    let y: Vec<Value> = (0..n).map(|i| Value::Float((i * 5) as f64)).collect();
    let count: Vec<Value> = (0..n).map(|i| Value::Float((i + 1) as f64)).collect();
    let width = vec![Value::Float(6.0); n];
    let height = vec![Value::Float(6.0); n];
    vec![
        ("x".into(), x),
        ("y".into(), y),
        ("count".into(), count),
        ("width".into(), width),
        ("height".into(), height),
    ]
}

/// A `Fill` legend mapped only via `.layer_aes(...)` (not the plot-level
/// `.aes(...)`) still reserves its panel margin — same as mapping it at
/// the plot level.
#[test]
fn layer_level_fill_mapping_reserves_legend_margin() {
    let plot_level = render(
        GGPlot::new(hex_frame())
            .aes(Aes::new().x("x").y("y").fill("count"))
            .geom_hex_with(GeomHex::default())
            .stat(StatIdentity)
            .scale_fill(gradient()),
    );
    let layer_level = render(
        GGPlot::new(hex_frame())
            .aes(Aes::new().x("x").y("y"))
            .geom_hex_with(GeomHex::default())
            .stat(StatIdentity)
            .layer_aes(Aes::new().x("x").y("y").fill("count"))
            .scale_fill(gradient()),
    );

    let plot_level_rect = data_plot_rect(&plot_level);
    let layer_level_rect = data_plot_rect(&layer_level);
    assert_eq!(
        plot_level_rect, layer_level_rect,
        "a layer-only fill mapping should reserve exactly the same margin \
         as a plot-level one: plot-level={plot_level_rect:?} layer-level={layer_level_rect:?}"
    );

    // Not clipped: the legend's own background rect must end at or before
    // the canvas edge, not run past it.
    let (lx, lw) = legend_rect(&layer_level);
    assert!(
        lx + lw <= W as f64,
        "legend rect [x={lx}, width={lw}] overflows the {W}px canvas"
    );
    // And it must actually sit to the right of the plotted data, not
    // overlap it.
    assert!(
        lx >= layer_level_rect[2],
        "legend (x={lx}) overlaps the plot area (right edge={})",
        layer_level_rect[2]
    );
}

/// A second layer — even one that doesn't map `fill` at all, like
/// `geom_abline` — must not undo the first layer's legend-margin
/// reservation. This is the exact shape of DataZooDE/anofox-visualization's
/// `::HEXBIN` + `::IDENTITY` combination.
#[test]
fn a_second_non_fill_layer_does_not_break_the_reservation() {
    let one_layer = render(
        GGPlot::new(hex_frame())
            .aes(Aes::new().x("x").y("y"))
            .geom_hex_with(GeomHex::default())
            .stat(StatIdentity)
            .layer_aes(Aes::new().x("x").y("y").fill("count"))
            .scale_fill(gradient()),
    );
    let two_layers = render(
        GGPlot::new(hex_frame())
            .aes(Aes::new().x("x").y("y"))
            .geom_hex_with(GeomHex::default())
            .stat(StatIdentity)
            .layer_aes(Aes::new().x("x").y("y").fill("count"))
            .scale_fill(gradient())
            .geom_abline_with(GeomAbline {
                slope: 1.0,
                intercept: 0.0,
                color: (0, 0, 0),
                width: 1.0,
                linetype: ggplot_rs::render::backend::Linetype::Dashed,
                alpha: 1.0,
            }),
    );

    let one_rect = data_plot_rect(&one_layer);
    let two_rect = data_plot_rect(&two_layers);
    assert_eq!(
        one_rect, two_rect,
        "adding a second (non-fill-mapped) layer must not change the \
         reserved legend margin: one_layer={one_rect:?} two_layers={two_rect:?}"
    );

    let (lx, lw) = legend_rect(&two_layers);
    assert!(
        lx + lw <= W as f64,
        "legend rect [x={lx}, width={lw}] overflows the {W}px canvas with a second layer"
    );
    assert!(
        two_layers.contains("data-geom=\"abline\""),
        "the abline layer must still draw"
    );
}

/// `show_legend(false)` on the fill-mapping layer still opts it out of the
/// margin reservation (it would be wasted space once the guide itself is
/// suppressed).
#[test]
fn show_legend_false_on_the_mapping_layer_skips_the_reservation() {
    let shown = render(
        GGPlot::new(hex_frame())
            .aes(Aes::new().x("x").y("y"))
            .geom_hex_with(GeomHex::default())
            .stat(StatIdentity)
            .layer_aes(Aes::new().x("x").y("y").fill("count"))
            .scale_fill(gradient()),
    );
    let hidden = render(
        GGPlot::new(hex_frame())
            .aes(Aes::new().x("x").y("y"))
            .geom_hex_with(GeomHex::default())
            .stat(StatIdentity)
            .layer_aes(Aes::new().x("x").y("y").fill("count"))
            .scale_fill(gradient())
            .show_legend(false),
    );

    assert!(
        !hidden.contains(">count<"),
        "legend should not draw: {hidden}"
    );
    let shown_rect = data_plot_rect(&shown);
    let hidden_rect = data_plot_rect(&hidden);
    assert!(
        hidden_rect[2] > shown_rect[2],
        "hiding the legend should give the panel back its margin: \
         shown_right={} hidden_right={}",
        shown_rect[2],
        hidden_rect[2]
    );
}
