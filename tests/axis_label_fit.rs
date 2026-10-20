//! B3-1: text metrics, outer-label fit, `check.overlap` (D5/D6).
//!
//! `axis_label_priority`'s R parity (ggplot2 4.0.3: `axis_label_priority(5)`
//! is `1 5 3 2 4`, `axis_label_priority(10)` is `1 10 5 3 2 4 7 6 8 9`) is
//! unit-tested in `src/render/text.rs` (it's `pub(crate)`, not reachable
//! from here). The "tick outside the SVG", "Clip doesn't shift" and "Shift
//! moves the label inside" cases are unit-tested in `src/guide/axis.rs`
//! with a toy scale, for the same reason. This file covers everything
//! reachable through the public `GGPlot` API.

use ggplot_rs::prelude::*;

fn render(plot: GGPlot, w: u32, h: u32) -> String {
    plot.render_svg_native_with_size(w, h)
        .expect("native svg render")
}

/// Extract the `x="..."` attribute of the `<text ...>` tag immediately
/// preceding `>{label}<` in the SVG (order-sensitive, but every label here
/// is unique within its plot).
fn label_x(svg: &str, label: &str) -> Option<f64> {
    let needle = format!(">{label}<");
    let end = svg.find(&needle)?;
    let start = svg[..end].rfind("<text ")?;
    let tag = &svg[start..end];
    let after = tag.split("x=\"").nth(1)?;
    after.split('"').next()?.parse().ok()
}

fn count_text_elements(svg: &str) -> usize {
    svg.matches("<text ").count()
}

/// One point per month across two years (epoch seconds), so the date
/// labels (`%Y-%m-%d`) are long — ~10 characters — and crowd a narrow axis.
fn timeseries(n: usize) -> Vec<(String, Vec<Value>)> {
    let base = 1_609_459_200.0; // 2021-01-01 UTC
    let x: Vec<Value> = (0..n)
        .map(|m| Value::Float(base + m as f64 * 2_629_800.0))
        .collect();
    let y: Vec<Value> = (0..n).map(|m| Value::Float((m as f64).sin())).collect();
    vec![("t".to_string(), x), ("y".to_string(), y)]
}

fn date_plot(n: usize) -> GGPlot {
    GGPlot::new(timeseries(n))
        .aes(Aes::new().x("t").y("y"))
        .geom_line()
        .scale_x_datetime(
            // An explicit `Secs` step (parsed from "14 days") takes the
            // generic `stepped_breaks` path, which honours
            // `with_date_labels` — the calendar-snapped month/year paths
            // don't (a pre-existing limitation, out of scope for B3-1).
            ScaleDateTime::new()
                .with_date_breaks("14 days")
                .with_date_labels("%Y-%m-%d"),
        )
}

/// A 280-px date axis: the last `%Y-%m-%d` label (~10 chars) overflows the
/// right edge under the 0.21 behaviour (`Clip`); `Reserve` (the default)
/// and `Shift` both correct it.
#[test]
fn narrow_date_axis_reserve_and_shift_fit_clip_does_not() {
    const W: u32 = 280;
    const H: u32 = 200;

    let clip_svg = render(date_plot(6).axis_text_x_fit(AxisLabelFit::Clip), W, H);
    let reserve_svg = render(date_plot(6).axis_text_x_fit(AxisLabelFit::Reserve), W, H);
    let shift_svg = render(date_plot(6).axis_text_x_fit(AxisLabelFit::Shift), W, H);

    let last_label = "2021-05-27"; // the last 14-day break in range

    let clip_x = label_x(&clip_svg, last_label).expect("clip label");
    let reserve_x = label_x(&reserve_svg, last_label).expect("reserve label");
    let shift_x = label_x(&shift_svg, last_label).expect("shift label");

    let half_w = ggplot_rs::render::text::text_width(last_label, 8.8) / 2.0;

    assert!(
        clip_x + half_w > W as f64 - 1.0,
        "expected Clip's unshifted label to overflow the right edge: x={clip_x} half_w={half_w}"
    );
    assert!(
        reserve_x + half_w <= W as f64 - 1.0 + 0.1,
        "Reserve should keep the label on-canvas: x={reserve_x} half_w={half_w}"
    );
    assert!(
        shift_x + half_w <= W as f64 - 1.0 + 0.1,
        "Shift should keep the label on-canvas: x={shift_x} half_w={half_w}"
    );
    // Reserve's layout-level correction and Shift's draw-time-only
    // correction land the label at different x, both inside the canvas.
    assert!(
        (reserve_x - clip_x).abs() > 1e-6 || (shift_x - clip_x).abs() > 1e-6,
        "at least one fit mode must move the label from Clip's raw position"
    );
}

/// A continuous x/y with 30 explicit breaks 0..29 — continuous axes never
/// get the discrete-axis auto-rotation heuristic, so this is a clean venue
/// for `check.overlap` itself (D5), independent of that other crowding
/// remedy.
fn thirty_breaks() -> Vec<(String, Vec<Value>)> {
    // 4-digit values so each label is wide enough to collide with its
    // neighbours once 30 of them are packed into a narrow panel.
    let x: Vec<Value> = (0..30).map(|i| Value::Float(1000.0 + i as f64)).collect();
    let y: Vec<Value> = (0..30)
        .map(|i| Value::Float((i % 5) as f64 + 1.0))
        .collect();
    vec![("x".to_string(), x), ("y".to_string(), y)]
}

fn thirty_breaks_scale() -> ggplot_rs::scale::continuous::ScaleContinuous {
    ScaleContinuous::new().with_breaks((0..30).map(|i| 1000.0 + i as f64).collect())
}

/// 30 flat continuous breaks at a width that can't show them all without
/// collisions: `check.overlap` thins them in ggplot2's priority order,
/// always keeping the first and last.
#[test]
fn check_overlap_thins_30_crowded_breaks_keeping_first_and_last() {
    let base = || {
        GGPlot::new(thirty_breaks())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .scale_x_continuous(thirty_breaks_scale())
    };

    let off = render(base(), 400, 300);
    let on = render(base().axis_text_x_check_overlap(true), 400, 300);

    // Ticks are unaffected (always drawn); only label <text> count drops.
    let off_labels = count_text_elements(&off);
    let on_labels = count_text_elements(&on);
    assert!(
        on_labels < off_labels,
        "check.overlap should drop some labels: off={off_labels} on={on_labels}"
    );
    assert!(
        on.contains(">1000<"),
        "the first break must survive thinning"
    );
    assert!(
        on.contains(">1029<"),
        "the last break must survive thinning"
    );
}

/// The same thinning, but with the labels rotated 45° — exercises the
/// parallel-distance overlap test (`|Δx|·|sin a| < size`) instead of the
/// flat bbox test.
#[test]
fn check_overlap_thins_rotated_labels_too() {
    let base = || {
        GGPlot::new(thirty_breaks())
            .aes(Aes::new().x("x").y("y"))
            .geom_point()
            .scale_x_continuous(thirty_breaks_scale())
            .axis_text_x_angle(45.0)
    };

    let off = render(base(), 400, 300);
    let on = render(base().axis_text_x_check_overlap(true), 400, 300);

    assert!(count_text_elements(&on) < count_text_elements(&off));
    assert!(on.contains(">1000<"));
    assert!(on.contains(">1029<"));
}

/// A plot whose outer labels already fit comfortably at a generous width:
/// `Reserve` changes nothing, so the rendered SVG is byte-identical to the
/// explicit `Clip` (0.21) behaviour.
#[test]
fn a_wide_chart_is_unaffected_by_reserve() {
    let clip = render(date_plot(4).axis_text_x_fit(AxisLabelFit::Clip), 1200, 500);
    let reserve = render(
        date_plot(4).axis_text_x_fit(AxisLabelFit::Reserve),
        1200,
        500,
    );
    assert_eq!(clip, reserve, "a wide chart's labels already fit");
}
