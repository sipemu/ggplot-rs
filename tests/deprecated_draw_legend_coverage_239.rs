//! #239 codecov/patch follow-up: PR #246's refactor rewrote the deprecated
//! `draw_legend`'s 3 call sites (wrapping a single aesthetic into a
//! `group_mergeable_legends`-shaped 1-element slice — see `guide/legend.rs`)
//! without changing its *behaviour*. `draw_legend` has been unused
//! internally since 0.23 (superseded by the crate-internal
//! `draw_legend_planned`), so no existing test called it directly and these
//! newly-reformatted lines counted as uncovered diff. It's still a public,
//! if deprecated, function — exercise it directly in both orientations
//! (horizontal/vertical toggle its two discrete-legend call sites) plus its
//! continuous, non-colour/fill branch (a continuous `linewidth` scale,
//! drawn discrete-like from sampled breaks).
#![allow(deprecated)]

use ggplot_rs::guide::legend::draw_legend;
use ggplot_rs::prelude::*;
use ggplot_rs::render::svg_backend::SvgBackend;
use ggplot_rs::render::Rect;
use ggplot_rs::scale::ScaleSet;
use std::collections::HashSet;

fn scales_with_a_discrete_colour_and_a_continuous_linewidth() -> ScaleSet {
    let mut scales = ScaleSet::new();
    scales.add(Box::new(ScaleManual::new(
        Aesthetic::Color,
        vec![
            ("a", RGBAColor::new(200, 0, 0)),
            ("b", RGBAColor::new(0, 0, 200)),
        ],
    )));
    scales.add(Box::new(
        ScaleLinewidthContinuous::new().with_limits(1.0, 10.0),
    ));
    scales
}

#[test]
fn horizontal_top_legend_draws_a_discrete_row_and_a_continuous_non_colour_legend() {
    let scales = scales_with_a_discrete_colour_and_a_continuous_linewidth();
    let theme = Theme::default().set_legend_position(LegendPosition::Top);
    let plot_area = Rect {
        x: 0.0,
        y: 0.0,
        width: 400.0,
        height: 300.0,
    };
    let mut backend = SvgBackend::new(400, 300, plot_area.clone());
    let guide = GuideLegend::default();
    draw_legend(
        &scales,
        &theme,
        &plot_area,
        &mut backend,
        &guide,
        &HashSet::new(),
    )
    .expect("deprecated draw_legend should still draw a horizontal legend without error");
}

#[test]
fn vertical_right_legend_draws_a_discrete_column_and_a_continuous_non_colour_legend() {
    // `Theme::default()`'s legend_position is `Right` (vertical) — the
    // other orientation from the test above, exercising `draw_legend`'s
    // other discrete-legend call site.
    let scales = scales_with_a_discrete_colour_and_a_continuous_linewidth();
    let theme = Theme::default();
    let plot_area = Rect {
        x: 0.0,
        y: 0.0,
        width: 400.0,
        height: 300.0,
    };
    let mut backend = SvgBackend::new(400, 300, plot_area.clone());
    let guide = GuideLegend::default();
    draw_legend(
        &scales,
        &theme,
        &plot_area,
        &mut backend,
        &guide,
        &HashSet::new(),
    )
    .expect("deprecated draw_legend should still draw a vertical legend without error");
}
