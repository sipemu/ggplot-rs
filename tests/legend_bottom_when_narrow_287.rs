//! sipemu/ggplot-rs#287: `legend_bottom_when_narrow(share)` moves a `Right`
//! legend under the panel when its band would take more than `share` of
//! the render width; a wide render keeps `Right`, and without the knob
//! nothing changes.

use ggplot_rs::aes::Aes;
use ggplot_rs::data::Value;
use ggplot_rs::prelude::*;
use ggplot_rs::theme::LegendPosition;

fn plot() -> GGPlot {
    let n = 20;
    let col = |f: &dyn Fn(i32) -> f64| (0..n).map(|i| Value::Float(f(i))).collect::<Vec<_>>();
    let data = vec![
        ("x".to_string(), col(&|i| i as f64)),
        ("y".to_string(), col(&|i| (i % 5) as f64)),
        ("z".to_string(), col(&|i| i as f64)),
    ];
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").fill("z"))
        .geom_point()
}

fn panel_width(svg: &str) -> f64 {
    let start = svg.find("data-plot=\"").expect("data-plot") + 11;
    svg[start..]
        .split('"')
        .next()
        .unwrap()
        .split_whitespace()
        .nth(2)
        .unwrap()
        .parse()
        .unwrap()
}

fn render(p: GGPlot, w: u32) -> String {
    p.render_svg_native_with_size(w, 320).expect("renders")
}

#[test]
fn a_narrow_render_moves_the_legend_below_the_panel() {
    let narrow = render(plot().legend_bottom_when_narrow(0.35), 163);
    let explicit = render(plot().legend_position(LegendPosition::Bottom), 163);
    assert_eq!(narrow, explicit, "same as an explicit Bottom legend");
    assert!(panel_width(&narrow) > panel_width(&render(plot(), 163)) + 40.0);
}

#[test]
fn a_wide_render_and_the_default_are_unchanged() {
    assert_eq!(
        render(plot().legend_bottom_when_narrow(0.35), 480),
        render(plot(), 480)
    );
    let left = plot().legend_position(LegendPosition::Left);
    assert_eq!(
        render(left.legend_bottom_when_narrow(0.35), 163),
        render(plot().legend_position(LegendPosition::Left), 163),
        "only a Right legend moves"
    );
}
