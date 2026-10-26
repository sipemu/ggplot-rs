//! sipemu/ggplot-rs#545: an ordinary per-aesthetic legend (not the
//! bivariate corner layout — see anofox-visualization#549/#546) placed at
//! `Top`/`Bottom` ran its keys/labels straight past the panel's right edge
//! in a narrow plot, with no wrap and no shrink. Two independent bugs
//! combined to cause this:
//!
//! 1. `plan_all_legends`/`draw_legend_planned` (B3-2's wrap/grid
//!    machinery, `src/guide/legend.rs`) only ever planned a *discrete*
//!    guide. A continuous, non-colourbar guide (`Size`/`Linewidth`/…
//!    sampled breaks) drew unconditionally via `draw_discrete_legend_at`
//!    with no width check at all, so it just kept drawing keys left to
//!    right past the row's available width.
//! 2. `plot.rs`'s B3-2 reservation pass was gated on `legend_aes`
//!    containing at least one *discrete* guide (`has_discrete_guide`), so
//!    a plot mapping only a continuous `Size`/`Linewidth` (no discrete
//!    guide alongside it) skipped the whole pass — `extras.legend_extra_rows`
//!    stayed `0` even once (1) was fixed and the legend itself wanted to
//!    wrap onto more than one row.
//!
//! Both are now fixed: every continuous guide except a true colourbar
//! (`Color`/`Fill`) is wrap-eligible exactly like a discrete guide, and
//! the reservation pass runs whenever *any* legend aesthetic — discrete or
//! wrap-eligible continuous — is present.

use ggplot_rs::prelude::*;

/// A `linewidth`-only plot (no discrete guide at all) whose continuous
/// legend samples `n_keys` breaks by spanning a wide enough data range
/// — `n_keys` is approximate (ggplot-rs's "pretty" break algorithm picks
/// the exact count), used only to drive the key count up or down.
fn linewidth_plot(max_lw: f64) -> GGPlot {
    let data: Vec<(String, Vec<Value>)> = vec![
        ("x".into(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("y".into(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ("lw".into(), vec![Value::Float(1.0), Value::Float(max_lw)]),
    ];
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").linewidth("lw"))
        .geom_line()
}

/// A `linewidth` + discrete `color` plot sharing one `Top`/`Bottom` row —
/// the combination the original report (anofox-visualization#545, a flow
/// map) actually hit: a discrete guide that *does* get B3-2 treatment
/// sitting beside a continuous one that (before this fix) didn't.
fn linewidth_and_color_plot(max_lw: f64, n_groups: usize) -> GGPlot {
    let n = n_groups.max(1);
    let x: Vec<Value> = (0..n).map(|i| Value::Float(i as f64)).collect();
    let y: Vec<Value> = (0..n).map(|i| Value::Float((i % 3) as f64)).collect();
    let lw: Vec<Value> = (0..n)
        .map(|i| Value::Float(1.0 + max_lw * (i as f64) / (n as f64 - 1.0).max(1.0)))
        .collect();
    let grp: Vec<Value> = (0..n)
        .map(|i| Value::Str(format!("Region with a fairly long name {:02}", i + 1)))
        .collect();
    let data: Vec<(String, Vec<Value>)> = vec![
        ("x".into(), x),
        ("y".into(), y),
        ("lw".into(), lw),
        ("grp".into(), grp),
    ];
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y").linewidth("lw").color("grp"))
        .geom_line()
}

/// Every `<text>` element's `x` attribute (its anchor — `Start` for every
/// legend label this test touches, so `x` is the label's *left* edge) and
/// its own text content.
fn all_text(svg: &str) -> Vec<(f64, String)> {
    let mut out = Vec::new();
    for (start, _) in svg.match_indices("<text ") {
        let end = svg[start..].find("</text>").unwrap() + start;
        let tag = &svg[start..end];
        let x_start = tag.find("x=\"").unwrap() + 3;
        let x_end = tag[x_start..].find('"').unwrap() + x_start;
        let x: f64 = tag[x_start..x_end].parse().unwrap();
        let text_start = tag.rfind('>').unwrap() + 1;
        out.push((x, tag[text_start..].to_string()));
    }
    out
}

fn assert_all_text_on_canvas(svg: &str, w: u32, ctx: &str) {
    for (x, label) in all_text(svg) {
        assert!(
            (0.0..=w as f64).contains(&x),
            "{ctx}: <text>{label}</text> x={x} overflows the {w}px canvas"
        );
    }
}

/// #545's core regression: a width sweep (200-600px) crossed with a key
/// count sweep (driven by the data range, ~3-8 keys) for a `linewidth`-
/// only legend at `Top`/`Bottom` — every label must stay on the canvas,
/// whether or not it needs to wrap onto extra rows to get there.
#[test]
fn linewidth_only_legend_stays_on_canvas_across_a_width_and_key_sweep() {
    const H: u32 = 300;
    for w in [200u32, 250, 330, 390, 480, 600] {
        for max_lw in [20.0, 200.0, 5_000.0, 50_000.0] {
            for pos in [LegendPosition::Top, LegendPosition::Bottom] {
                let svg = linewidth_plot(max_lw)
                    .legend_position(pos.clone())
                    .render_svg_native_with_size(w, H)
                    .unwrap();
                assert_all_text_on_canvas(&svg, w, &format!("{pos:?} w={w} max_lw={max_lw}"));
            }
        }
    }
}

/// The combined case from the actual report: a discrete `color` guide
/// (long labels, several groups so it already wraps on its own) sharing a
/// `Top` row with a continuous `linewidth` guide. Both must stay on
/// canvas at every width in the sweep.
#[test]
fn linewidth_and_discrete_color_legend_stay_on_canvas_across_a_width_and_key_sweep() {
    const H: u32 = 320;
    for w in [250u32, 330, 390, 480, 600] {
        for n_groups in [3usize, 5, 8] {
            let svg = linewidth_and_color_plot(5_000.0, n_groups)
                .legend_position(LegendPosition::Top)
                .render_svg_native_with_size(w, H)
                .unwrap();
            assert_all_text_on_canvas(&svg, w, &format!("w={w} n_groups={n_groups}"));
        }
    }
}

/// A narrow plot (390px, the original report's width) still shows every
/// key — wrapping, not dropping silently, is what should make it fit. A
/// discrete guide already supports "+N more" once it's out of rows; this
/// just confirms the continuous guide's keys are still all present (none
/// were ever meant to be dropped — #545 only scoped wrap, not overflow,
/// to the continuous case) rather than merely off-canvas and invisible.
#[test]
fn every_linewidth_key_is_still_present_once_it_wraps() {
    const W: u32 = 330;
    const H: u32 = 300;
    let svg = linewidth_plot(50_000.0)
        .legend_position(LegendPosition::Top)
        .render_svg_native_with_size(W, H)
        .unwrap();
    for label in ["10000", "20000", "30000", "40000", "50000"] {
        assert!(
            svg.contains(&format!(">{label}<")),
            "expected key {label:?} to still be drawn somewhere on a wrapped row: {svg}"
        );
    }
    assert_all_text_on_canvas(
        &svg,
        W,
        "every_linewidth_key_is_still_present_once_it_wraps",
    );
}

/// Reserving extra rows for a wrapped continuous legend must actually
/// shrink the panel, the same way it does for a discrete one (#233) —
/// otherwise the reservation pass silently no-ops and the wrapped rows
/// just overlap the panel instead of sitting above it.
#[test]
fn a_wrapped_linewidth_legend_reserves_extra_panel_height() {
    const W: u32 = 230;
    const H: u32 = 300;
    let one_row = linewidth_plot(5.0) // small range -> few keys, fits on one row
        .legend_position(LegendPosition::Top)
        .render_svg_native_with_size(W, H)
        .unwrap();
    let many_rows = linewidth_plot(50_000.0) // wide range -> wraps
        .legend_position(LegendPosition::Top)
        .render_svg_native_with_size(W, H)
        .unwrap();

    fn plot_rect_height(svg: &str) -> f64 {
        let start = svg.find("data-plot=\"").unwrap() + "data-plot=\"".len();
        let rest = &svg[start..];
        let end = rest.find('"').unwrap();
        let mut it = rest[..end].split_whitespace();
        it.next(); // x
        it.next(); // y
        it.next(); // width
        it.next().unwrap().parse().unwrap() // height
    }

    assert!(
        plot_rect_height(&many_rows) < plot_rect_height(&one_row),
        "a wrapped linewidth legend should reserve more band height, shrinking the panel: \
         one_row_h={} many_rows_h={}",
        plot_rect_height(&one_row),
        plot_rect_height(&many_rows)
    );
}
