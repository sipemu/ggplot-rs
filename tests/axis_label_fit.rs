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

/// The panel's width from `data-plot="x y w h"`.
fn panel_width(svg: &str) -> f64 {
    let start = svg.find("data-plot=\"").unwrap() + "data-plot=\"".len();
    let rest = &svg[start..];
    let end = rest.find('"').unwrap();
    rest[..end]
        .split_whitespace()
        .nth(2)
        .unwrap()
        .parse()
        .unwrap()
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

/// #212: 5 breaks (0/25/50/75/100, as `extended_breaks()` picks for a
/// 0..100-ish domain) over a ~50 px panel — the exact shape a narrow
/// dashboard card's right-hand legend leaves a hex/scatter chart's x axis
/// (DataZooDE/anofox-visualization#450). Without a minimum legible gap, two
/// adjacent kept labels can land a pixel or two apart — not literally
/// overlapping by the bbox test, but close enough that two numbers read as
/// one run (e.g. "25" beside "50" as "2550"). `check_overlap` must thin
/// *below* what a literal "do these bboxes touch" test would keep, for a
/// recognisable gap between every pair of labels that survive.
#[test]
fn check_overlap_leaves_a_legible_gap_not_just_a_non_overlap() {
    let x: Vec<Value> = vec![0.0, 25.0, 50.0, 75.0, 100.0]
        .into_iter()
        .map(Value::Float)
        .collect();
    let y: Vec<Value> = vec![0.0, 1.0, 2.0, 3.0, 4.0]
        .into_iter()
        .map(Value::Float)
        .collect();
    let f = x.clone();
    let data = vec![
        ("x".to_string(), x),
        ("y".to_string(), y),
        ("f".to_string(), f),
    ];
    let scale = ScaleContinuous::new().with_breaks(vec![0.0, 25.0, 50.0, 75.0, 100.0]);
    // A right-hand continuous fill legend, same as the narrow-dashboard-card
    // case that surfaced this (#450) — eats most of a 163 px panel, leaving
    // the x axis only ~50 px.
    let mut fill =
        ggplot_rs::scale::color::ScaleColorContinuous::new(ggplot_rs::aes::Aesthetic::Fill)
            .with_colors(
                ggplot_rs::scale::color::RGBAColor {
                    r: 230,
                    g: 235,
                    b: 250,
                    a: 1.0,
                },
                ggplot_rs::scale::color::RGBAColor {
                    r: 20,
                    g: 50,
                    b: 160,
                    a: 1.0,
                },
            );
    {
        use ggplot_rs::scale::Scale;
        fill.set_name("count");
    }
    let svg = render(
        GGPlot::new(data)
            .aes(Aes::new().x("x").y("y").fill("f"))
            .geom_point()
            .scale_x_continuous(scale)
            .scale_fill(fill)
            .axis_text_x_check_overlap(true),
        163,
        280,
    );
    assert!(
        panel_width(&svg) < 60.0,
        "fixture should reproduce the narrow panel: {}",
        panel_width(&svg)
    );
    let kept: Vec<(&str, f64)> = ["0", "25", "50", "75", "100"]
        .into_iter()
        .filter_map(|l| label_x(&svg, l).map(|x| (l, x)))
        .collect();
    assert!(kept.len() >= 2, "too few labels survived: {kept:?}");
    // No two *kept* labels' bboxes sit closer than one space character's
    // width apart — the minimum edge-to-edge gap `check_overlap` now
    // enforces (not just "centres are a few px apart": a wide label like
    // "100" can still reach into its neighbour even when their centres are
    // comfortably separated).
    let min_gap = ggplot_rs::render::text::text_width(" ", 8.8);
    let half_width = |l: &str| ggplot_rs::render::text::text_width(l, 8.8) / 2.0;
    for w in kept.windows(2) {
        let gap = (w[1].1 - half_width(w[1].0)) - (w[0].1 + half_width(w[0].0));
        assert!(
            gap > min_gap,
            "{} and {} are only {gap}px apart edge to edge (min {min_gap}px): {kept:?}",
            w[0].0,
            w[1].0
        );
    }
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

/// Regression for review #158 finding 1 (HIGH): `Reserve`'s left/right
/// divisors were inverted (`deficit / p` instead of `deficit / (1 − p)` on
/// the left, and the mirror image on the right), over-reserving by about
/// `1 / p_last` — roughly 20× at a typical `p_last ≈ 0.95` — and collapsing
/// ordinary charts to the panel-width floor. A 600×300 date line whose last
/// label overflows the canvas by `d` px must narrow the panel by *about*
/// `d` px (bounded well under 2×d here, since this case's `p_last` is close
/// to 1), never by anywhere near 20×d.
#[test]
fn reserve_narrows_the_panel_by_about_the_overflow_not_20x() {
    const W: u32 = 600;
    const H: u32 = 300;
    let clip = render(date_plot(10).axis_text_x_fit(AxisLabelFit::Clip), W, H);
    let reserve = render(date_plot(10).axis_text_x_fit(AxisLabelFit::Reserve), W, H);

    let clip_w = panel_width(&clip);
    let reserve_w = panel_width(&reserve);
    let narrowed_by = clip_w - reserve_w;
    assert!(
        narrowed_by > 1.0,
        "this case must actually overflow: {narrowed_by}"
    );

    // The last label's measured overflow past the canvas edge under Clip.
    let last_label = "2021-10-14";
    let x = label_x(&clip, last_label).expect("last label");
    let half_w = ggplot_rs::render::text::text_width(last_label, 8.8) / 2.0;
    let deficit = (x + half_w) - (W as f64 - 1.0);
    assert!(deficit > 1.0, "this case must actually overflow: {deficit}");

    // The old (inverted) formula divided by `1 - p_last` (~0.05-0.1 here),
    // which would narrow the panel by roughly 10-20x the deficit — easily
    // 150+ px. The fix divides by `p_last` (close to 1 here), so the panel
    // narrows by close to, and at most ~1.5x, the deficit.
    assert!(
        narrowed_by < deficit * 1.5,
        "narrowed by {narrowed_by}, deficit was {deficit} — looks like the inverted-divisor bug"
    );
    assert!(
        narrowed_by < 30.0,
        "narrowed by {narrowed_by} px — far more than this ~{deficit}px overflow justifies"
    );
}

/// The 280px and 500px narrow-date-axis cases from review #158 stay sane
/// (small, proportional reservations — not collapsed to the panel-width
/// floor of 50px).
#[test]
fn reserve_stays_sane_on_the_280_and_500_px_cases() {
    for w in [280u32, 500] {
        let clip = render(date_plot(6).axis_text_x_fit(AxisLabelFit::Clip), w, 300);
        let reserve = render(date_plot(6).axis_text_x_fit(AxisLabelFit::Reserve), w, 300);
        let clip_w = panel_width(&clip);
        let reserve_w = panel_width(&reserve);
        assert!(
            reserve_w > clip_w - 30.0,
            "w={w}: reserve panel {reserve_w} collapsed too far below clip's {clip_w}"
        );
        assert!(
            reserve_w > 50.1,
            "w={w}: reserve panel {reserve_w} hit the panel-width floor — looks collapsed"
        );
    }
}

/// Regression for review #158's second re-review (LOW): a single, centred
/// discrete break whose label is wider than the whole canvas has
/// `p_first == p_last == 0.5`, the degenerate case where `eL`/`eR` grow
/// independently and could together push `plot_x` past the canvas
/// entirely. The reservation must now cap out and let `draw_x_axis`'s own
/// draw-time shift (the same fallback `Shift` always uses) take over —
/// never produce a NaN, a negative/zero panel, or a panel that starts
/// beyond the canvas.
#[test]
fn reserve_caps_out_for_a_single_centred_break_wider_than_the_canvas() {
    let data = vec![
        (
            "x".to_string(),
            vec![Value::Str(
                "An Absurdly Long Single Category Label Far Wider Than The Canvas".into(),
            )],
        ),
        ("y".to_string(), vec![Value::Float(5.0)]),
    ];
    let (svg, area) = GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_col()
        .render_svg_area_with_size(200, 150)
        .unwrap();
    assert!(!svg.contains("NaN"), "{svg}");
    assert!(
        (0.0..200.0).contains(&area[0]),
        "panel must start on-canvas: {area:?}"
    );
    assert!(area[2] >= 1.0, "panel width must stay positive: {area:?}");
}
