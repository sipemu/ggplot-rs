//! Regression tests for sipemu/ggplot-rs#162 and #163.
//!
//! #162: the discrete-axis auto-rotate heuristic counted every grid column
//! (`breaks.len()`), not just the labels actually drawn, so a
//! `with_labels`-thinned axis over-rotated even when the printed labels
//! would fit flat.
//!
//! #163 (a regression introduced by the first #162 fix, PR #163's review):
//! whenever labels did end up rotated (by auto-rotate *or* by an explicit
//! `axis_text_x_angle`), the layout didn't reserve the bottom space the
//! rotated bboxes need, so they could overlap the panel — but the first
//! attempt at reserving that space always ended the text at the tick
//! (`TextAnchor::End`) and pushed the whole label down by its rotated
//! rise, which is right for a counter-clockwise (e.g. −45°) rotation but
//! wrong for a clockwise one (e.g. +45°, +90°): the label's *end* landed
//! near its tick but far below the axis, while its *start* — the end
//! nearest the axis — ended up shifted sideways under the *previous*
//! category. The fix: auto-rotate now picks −45° (ggplot2's own
//! `angle = 45, hjust = 1` idiom, mirrored to this library's angle sign)
//! instead of +45°, and any clockwise (positive) angle anchors with
//! `TextAnchor::Start` instead, so in both cases the label's near-axis end
//! stays pinned at its own tick.
//!
//! `render::text::label_bbox` (the production bbox helper `layout_built`
//! and `draw_x_axis` both use) is `pub(crate)`, not reachable from an
//! integration test, so `local_label_bbox` below is a verbatim copy of its
//! geometry — plain trigonometry over the publicly-available
//! `render::text::text_width`, not a re-implementation of any decision
//! logic — used only to check the *rendered* SVG's own numbers for overlap
//! and anchor placement.

use ggplot_rs::prelude::*;
use ggplot_rs::render::text::text_width;

fn render(plot: GGPlot, w: u32, h: u32) -> String {
    plot.render_svg_native_with_size(w, h)
        .expect("native svg render")
}

fn render_with_area(plot: GGPlot, w: u32, h: u32) -> (String, [f64; 4]) {
    plot.render_svg_area_with_size(w, h)
        .expect("native svg render with area")
}

/// Verbatim copy of `src/render/text.rs::label_bbox`'s geometry (that
/// function is `pub(crate)`): the estimated bounding box `[x0, y0, x1, y1]`
/// of a label anchored at `(x, y)`, set at font `size`, rotated `angle`
/// degrees (SVG convention, clockwise), with `anchor` in `0.0..=1.0`.
fn local_label_bbox(text: &str, size: f64, x: f64, y: f64, angle: f64, anchor: f64) -> [f64; 4] {
    let w = text_width(text, size);
    let h = size * 1.2;
    let x0 = -w * anchor;
    let (s, c) = angle.to_radians().sin_cos();
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for (dx, dy) in [
        (x0, -h / 2.0),
        (x0 + w, -h / 2.0),
        (x0, h / 2.0),
        (x0 + w, h / 2.0),
    ] {
        let (px, py) = (x + dx * c - dy * s, y + dx * s + dy * c);
        b = [b[0].min(px), b[1].min(py), b[2].max(px), b[3].max(py)];
    }
    b
}

/// The `<text ...>` tag immediately preceding `>{label}<` in the SVG
/// (order-sensitive, but every label used here is unique within its plot).
fn text_tag<'a>(svg: &'a str, label: &str) -> Option<&'a str> {
    let needle = format!(">{label}<");
    let end = svg.find(&needle)?;
    let start = svg[..end].rfind("<text ")?;
    Some(&svg[start..end])
}

fn attr_f64(tag: &str, name: &str) -> Option<f64> {
    let needle = format!("{name}=\"");
    let s = tag.find(&needle)? + needle.len();
    let rest = &tag[s..];
    let e = rest.find('"')?;
    rest[..e].parse().ok()
}

fn attr_str<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("{name}=\"");
    let s = tag.find(&needle)? + needle.len();
    let rest = &tag[s..];
    let e = rest.find('"')?;
    Some(&rest[..e])
}

/// `(x, y, font_size, anchor_frac, angle_degrees)` of the `<text>` tag
/// drawing `label` in `svg`.
fn label_geometry(svg: &str, label: &str) -> (f64, f64, f64, f64, f64) {
    let tag = text_tag(svg, label).unwrap_or_else(|| panic!("label {label:?} not drawn: {svg}"));
    let x = attr_f64(tag, "x").expect("x");
    let y = attr_f64(tag, "y").expect("y");
    let size = attr_f64(tag, "font-size").expect("font-size");
    let anchor = match attr_str(tag, "text-anchor") {
        Some("start") => 0.0,
        Some("end") => 1.0,
        _ => 0.5,
    };
    let angle = tag
        .find("rotate(")
        .and_then(|s| {
            let rest = &tag[s + "rotate(".len()..];
            rest.find(' ').and_then(|e| rest[..e].parse().ok())
        })
        .unwrap_or(0.0);
    (x, y, size, anchor, angle)
}

/// The x pixel of the `i`-th (0-indexed, of `n`) discrete category's tick,
/// for a plain (unflipped) Cartesian coord: `ScaleDiscrete::map` places
/// category `i` at normalized position `(i + 0.5) / n`, which a linear
/// Cartesian transform maps straight onto `[area.x, area.x + area.width]`.
fn tick_x_for_category(area: [f64; 4], i: usize, n: usize) -> f64 {
    area[0] + (i as f64 + 0.5) / n as f64 * area[2]
}

/// 9 discrete day-of-data x categories, thinned to 5 printed labels via
/// `with_labels` (the documented way to keep every discrete *position*
/// while only printing a subset) — the exact device from #162's own repro.
fn thinned_date_levels() -> Vec<&'static str> {
    vec![
        "2026-09-01",
        "2026-09-02",
        "2026-09-03",
        "2026-09-04",
        "2026-09-05",
        "2026-09-06",
        "2026-09-07",
        "2026-09-08",
        "2026-09-09",
    ]
}

fn thinned_date_plot() -> GGPlot {
    let levels = thinned_date_levels();
    let labels: Vec<String> = levels
        .iter()
        .enumerate()
        .map(|(i, l)| {
            if i % 2 == 0 {
                l.to_string()
            } else {
                String::new()
            }
        })
        .collect(); // 5 real labels: index 0, 2, 4, 6, 8
    let data = vec![
        (
            "x".to_string(),
            levels.iter().map(|s| Value::Str(s.to_string())).collect(),
        ),
        ("y".to_string(), vec![Value::Float(1.0); levels.len()]),
    ];
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_x_discrete(
            ScaleDiscrete::new()
                .with_limits(levels.clone())
                .with_labels(labels),
        )
}

/// #162's own repro: with the fix, `n` (the crowding count) and the
/// max-label-width estimate only consider the 5 *drawn* labels, not the 4
/// blanked `with_labels` entries — at a comfortably wide 1200px canvas they
/// fit flat, so the axis must not auto-rotate.
#[test]
fn issue_162_repro_thinned_labels_fit_flat_at_1200px() {
    let svg = render(thinned_date_plot(), 1200, 320);

    assert!(
        !svg.contains("rotate(-45.0") && !svg.contains("rotate(270.0"),
        "a thinned (with_labels-blanked) discrete axis must not auto-rotate \
         when the printed labels comfortably fit flat: {svg}"
    );

    // One of the real, drawn labels renders flat: angle 0 (no `transform`)
    // and middle-anchored.
    let tag = text_tag(&svg, "2026-09-01").expect("first real label drawn");
    assert!(
        !tag.contains("transform"),
        "flat label must carry no rotate transform: {tag}"
    );
    assert_eq!(attr_str(tag, "text-anchor"), Some("middle"));

    // The blanked entries are never drawn at all.
    for blanked in ["2026-09-02", "2026-09-04", "2026-09-06", "2026-09-08"] {
        assert!(
            text_tag(&svg, blanked).is_none(),
            "a with_labels-blanked entry must not be drawn as a tick label"
        );
    }
}

/// 10 long, fully-printed (no blanking) discrete category labels packed
/// into a narrow 260px canvas: genuinely crowded, so the axis must still
/// auto-rotate — and, now that rotation also reserves real bottom space,
/// every drawn label's rotated bounding box must sit fully below (at or
/// past) the panel's own bottom edge: no overlap with the panel.
fn crowded_category_plot() -> GGPlot {
    let cats: Vec<String> = (0..10).map(|i| format!("Category Alpha {i:02}")).collect();
    let x: Vec<Value> = cats.iter().map(|s| Value::Str(s.clone())).collect();
    let y: Vec<Value> = (0..10)
        .map(|i| Value::Float((i % 5) as f64 + 1.0))
        .collect();
    let data = vec![("x".to_string(), x), ("y".to_string(), y)];
    GGPlot::new(data).aes(Aes::new().x("x").y("y")).geom_col()
}

#[test]
fn genuinely_crowded_discrete_axis_still_rotates_without_overlapping_the_panel() {
    const W: u32 = 260;
    const H: u32 = 220;
    let cats: Vec<String> = (0..10).map(|i| format!("Category Alpha {i:02}")).collect();

    let (svg, area) = render_with_area(crowded_category_plot(), W, H);
    let panel_bottom = area[1] + area[3];

    assert!(
        svg.contains("rotate(-45.0") || svg.contains("rotate(270.0"),
        "10 long category labels in a {W}px canvas must still auto-rotate: {svg}"
    );

    const OVERLAP_EPS: f64 = 0.5;
    for cat in &cats {
        let (x, y, size, anchor, angle) = label_geometry(&svg, cat);
        let bbox = local_label_bbox(cat, size, x, y, angle, anchor);
        let top_of_label = bbox[1]; // smallest y = the corner closest to the panel
        assert!(
            top_of_label >= panel_bottom - OVERLAP_EPS,
            "label {cat:?} overlaps the panel: bbox={bbox:?} panel_bottom={panel_bottom}"
        );
    }
}

/// An explicit `axis_text_x_angle(45)` rotates labels outside the
/// auto-rotate decision entirely, but the layout must still reserve the
/// real bottom space those rotated labels need (#162: before the fix, an
/// explicit angle got only a blind constant reservation, not one measured
/// from the actual labels) — so, same as the auto-rotate case, every
/// label's rotated bbox must sit fully below the panel's bottom edge.
#[test]
fn explicit_axis_text_x_angle_also_reserves_its_height() {
    const W: u32 = 260;
    const H: u32 = 220;
    let cats: Vec<String> = (0..10).map(|i| format!("Category Alpha {i:02}")).collect();

    let (svg, area) = render_with_area(crowded_category_plot().axis_text_x_angle(45.0), W, H);
    let panel_bottom = area[1] + area[3];

    assert!(
        svg.contains("rotate(45.0"),
        "explicit angle must render: {svg}"
    );

    const OVERLAP_EPS: f64 = 0.5;
    for cat in &cats {
        let (x, y, size, anchor, angle) = label_geometry(&svg, cat);
        assert!(
            (angle - 45.0).abs() < 0.05,
            "angle must stay the explicit 45°: {angle}"
        );
        let bbox = local_label_bbox(cat, size, x, y, angle, anchor);
        let top_of_label = bbox[1];
        assert!(
            top_of_label >= panel_bottom - OVERLAP_EPS,
            "label {cat:?} overlaps the panel under an explicit angle: bbox={bbox:?} panel_bottom={panel_bottom}"
        );
    }
}

/// A continuous x scale, 6 breaks `0..6`, placed on top
/// (`with_position_opposite`), at a given explicit angle and
/// `AxisLabelFit`. `Clip` (rather than the default `Reserve`) keeps the
/// panel's horizontal geometry identical across different angles on this
/// same plot — `Reserve`'s B3-1 left/right margin varies with how far a
/// rotated vs. flat label's bbox reaches past the canvas edge, which would
/// make tick x positions incomparable between an angled and a flat render
/// of the same plot otherwise.
fn top_axis_plot(labels: &[String], angle: f64) -> GGPlot {
    let x: Vec<Value> = (0..labels.len()).map(|i| Value::Float(i as f64)).collect();
    let y: Vec<Value> = (0..labels.len())
        .map(|i| Value::Float((i % 3) as f64 + 1.0))
        .collect();
    let data = vec![("x".to_string(), x), ("y".to_string(), y)];
    GGPlot::new(data)
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .scale_x_continuous(
            ScaleContinuous::new()
                .with_breaks((0..labels.len()).map(|i| i as f64).collect())
                .with_labels(labels.to_vec())
                .with_position_opposite(),
        )
        .axis_text_x_angle(angle)
        .axis_text_x_fit(AxisLabelFit::Clip)
}

/// A rotated axis placed on *top* (`with_position_opposite`) swings its
/// labels the opposite way: the panel sits *below* the axis line, so the
/// overlap risk is from the label's `fall` (the corner swinging down,
/// toward the panel), not its `rise` — `draw_x_axis` picks whichever one
/// applies by the tick direction (`dir`). Every drawn label's rotated bbox
/// must sit fully above (at or past) the panel's own top edge.
#[test]
fn explicit_angle_on_a_top_axis_reserves_height_on_the_fall_side() {
    const W: u32 = 260;
    const H: u32 = 220;
    let labels: Vec<String> = (0..6).map(|i| format!("Category Alpha {i:02}")).collect();

    let (svg, area) = render_with_area(top_axis_plot(&labels, 45.0), W, H);
    let panel_top = area[1];

    assert!(
        svg.contains("rotate(45.0"),
        "explicit angle must render: {svg}"
    );

    const OVERLAP_EPS: f64 = 0.5;
    for label in &labels {
        let (x, y, size, anchor, angle) = label_geometry(&svg, label);
        let bbox = local_label_bbox(label, size, x, y, angle, anchor);
        let bottom_of_label = bbox[3]; // largest y = the corner closest to the panel, from above
        assert!(
            bottom_of_label <= panel_top + OVERLAP_EPS,
            "label {label:?} overlaps the panel from above: bbox={bbox:?} panel_top={panel_top}"
        );
    }
}

/// #163's review: the overlap fix must not just stop the panel overlap, it
/// must anchor the *near-axis end* of each rotated label at its own tick —
/// not shift the whole label away to clear the panel. On a top axis this
/// is *also* a direction-aware check (not just angle sign): a clockwise
/// (+45°) angle there needs `TextAnchor::End`, the opposite of what the
/// same angle needs on a bottom axis, or the un-fixed intermediate attempt
/// (right overlap fix, wrong anchor for this side) still pushes the
/// anchor itself ~64px from the axis even though nothing overlaps.
/// Compare each flat tick's x (rendered with a sub-threshold 2° angle,
/// both "flat" and middle-anchored at the exact tick pixel, with the same
/// `Clip` fit so the panel geometry matches the rotated render) against
/// the 45°-rotated version's near-axis-end x.
#[test]
fn near_axis_end_stays_at_the_tick_on_a_top_axis() {
    const W: u32 = 260;
    const H: u32 = 220;
    let labels: Vec<String> = (0..6).map(|i| format!("Category Alpha {i:02}")).collect();

    let (svg, area) = render_with_area(top_axis_plot(&labels, 45.0), W, H);
    let panel_top = area[1];
    let (flat_svg, flat_area) = render_with_area(top_axis_plot(&labels, 2.0), W, H);
    // Only x/width need to match for the tick-x comparison below — y/height
    // legitimately differ (a rotated label reserves more top margin than a
    // flat one).
    assert_eq!(
        (area[0], area[2]),
        (flat_area[0], flat_area[2]),
        "Clip fit must give the flat and rotated renders the same horizontal panel geometry"
    );

    for label in &labels {
        let (flat_x, ..) = label_geometry(&flat_svg, label);
        let (x, y, size, ..) = label_geometry(&svg, label);
        let dx = (x - flat_x).abs();
        assert!(
            dx <= size * 1.5,
            "top axis: label {label:?}'s near-axis end at x={x} is {dx:.1}px from \
             its flat tick x={flat_x:.1} (font size {size})"
        );
        // Just above the axis (a small offset), never the ~64px the
        // direction-blind anchoring bug used to push it up by.
        let dy = panel_top - y;
        assert!(
            (0.0..=size * 2.5).contains(&dy),
            "top axis: label {label:?}'s near-axis end at y={y} is {dy:.1}px above \
             the panel top ({panel_top}) — expected a small offset, not a large push-up"
        );
    }
}

/// #163's review: the overlap fix must not just stop the panel overlap, it
/// must anchor the *near-axis end* of each rotated label at its own tick —
/// not shift the whole label down (and sideways, under the previous
/// category) to clear the panel. The SVG `<text x=".." y="..">`'s own
/// `(x, y)` *is* that near-axis end (it's the point `text-anchor` pins and
/// the rotation pivots around), so this checks it directly against each
/// category's own tick x, and against a small, fixed y-offset below (or,
/// for a top axis, above) the panel edge — not the ~45-53px an End-anchored
/// clockwise rotation used to misplace it by.
#[test]
fn near_axis_end_stays_at_the_tick_for_auto_and_explicit_rotations() {
    const W: u32 = 260;
    const H: u32 = 220;
    const N: usize = 10;
    let cats: Vec<String> = (0..N).map(|i| format!("Category Alpha {i:02}")).collect();

    // `None` angle = auto-rotate (crowded_category_plot() always triggers
    // it at this width); `Some(a)` = an explicit angle.
    let cases: [(&str, Option<f64>); 4] = [
        ("auto-rotate", None),
        ("explicit +45", Some(45.0)),
        ("explicit -45", Some(-45.0)),
        ("explicit +90", Some(90.0)),
    ];

    for (name, angle) in cases {
        let plot = match angle {
            Some(a) => crowded_category_plot().axis_text_x_angle(a),
            None => crowded_category_plot(),
        };
        let (svg, area) = render_with_area(plot, W, H);
        let panel_bottom = area[1] + area[3];

        assert!(
            svg.contains("rotate("),
            "{name}: must render rotated: {svg}"
        );

        for (i, cat) in cats.iter().enumerate() {
            let (x, y, size, _anchor, _angle) = label_geometry(&svg, cat);
            let expected_tick_x = tick_x_for_category(area, i, N);
            let dx = (x - expected_tick_x).abs();
            assert!(
                dx <= size * 1.5,
                "{name}: label {cat:?}'s near-axis end at x={x} is {dx:.1}px from \
                 its own tick x={expected_tick_x:.1} (font size {size}) — it has \
                 shifted toward a neighbouring category"
            );
            // Just below the axis (a small, roughly rotation-independent
            // offset), never the ~45-53px the clockwise-anchoring bug used
            // to push it down by.
            let dy = y - panel_bottom;
            assert!(
                (0.0..=size * 2.5).contains(&dy),
                "{name}: label {cat:?}'s near-axis end at y={y} is {dy:.1}px below \
                 the panel bottom ({panel_bottom}) — expected a small offset, not a \
                 large push-down"
            );
        }
    }
}
