//! Text-metric helpers for axis-label fit (B3-1): ggplot-rs owns the
//! outer-label-overflow and `check.overlap` logic centrally, instead of a
//! downstream consumer post-processing rendered SVG (the old approach had
//! to re-parse `<text>` elements out of the markup).
//!
//! Ported verbatim from the consumer's `ticks.rs` em table and
//! `TickLabel::bbox`, which is the only reason the measurements match what
//! callers previously saw.

/// Estimated width (px) of `text` set at font size `size`, in the house sans
/// (Hanken Grotesk, or the system UI font it falls back to). This is a
/// heuristic per-character average-width table, not real font metrics —
/// good enough for layout decisions (fit, overlap, auto-wrap), not for
/// pixel-perfect text extents.
pub fn text_width(text: &str, size: f64) -> f64 {
    let em: f64 = text
        .chars()
        .map(|ch| match ch {
            ' ' => 0.27,
            '.' | ',' | ':' | ';' | '\'' | '|' | '!' | 'i' | 'j' | 'l' | 'I' => 0.27,
            'f' | 't' | 'r' | '(' | ')' | '[' | ']' | '-' | '/' => 0.36,
            '0'..='9' | '$' | '€' | '£' | '+' | '=' | '<' | '>' | '#' | '*' | '?' => 0.57,
            'm' | 'w' => 0.8,
            'M' | 'W' | '%' | '@' => 0.86,
            'a'..='z' => 0.53,
            'A'..='Z' | '&' => 0.66,
            _ => 0.62,
        })
        .sum();
    em * size
}

/// Estimated bounding box `[x0, y0, x1, y1]` of a label anchored at
/// `(x, y)`, set at font `size`, rotated `angle` degrees (SVG convention,
/// clockwise), with `anchor` in `0.0` (text starts at `x`) `..=1.0` (text
/// ends at `x`) — `0.5` centres it, matching `TextAnchor::{Start,Middle,
/// End}`. Ported from the consumer's `TickLabel::bbox`.
pub(crate) fn label_bbox(
    text: &str,
    size: f64,
    x: f64,
    y: f64,
    angle: f64,
    anchor: f64,
) -> [f64; 4] {
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

/// For rotated bottom-axis tick labels, the extra vertical *rise* (above
/// the un-rotated anchor point) and *fall* (below it) the rotated bounding
/// boxes need, maxed over `labels` (sipemu/ggplot-rs#162). A label
/// end-anchored at a point just below the panel and rotated toward the
/// panel (e.g. the usual 45° "reads up-and-to-the-right" tilt) swings its
/// far end — `rise` — back up by roughly `width · sin(angle)`; a label
/// rotated away from the panel (e.g. 270°, hanging straight down) has no
/// rise at all, only `fall`.
///
/// Both `GGPlot::layout_built` (to reserve enough panel-bottom margin that
/// `fall` stays on-canvas) and `draw_x_axis` (to place the rotation anchor
/// `rise` further from the axis line than the flat-label offset, so the
/// rotated label never swings back over the panel) need the *same*
/// numbers — this is their one shared source, so the two can never drift
/// apart.
pub(crate) fn rotated_label_rise_fall<'a>(
    labels: impl Iterator<Item = &'a str>,
    size: f64,
    angle: f64,
    anchor: f64,
) -> (f64, f64) {
    let mut rise = 0.0_f64;
    let mut fall = 0.0_f64;
    for label in labels {
        let b = label_bbox(label, size, 0.0, 0.0, angle, anchor);
        rise = rise.max(-b[1]);
        fall = fall.max(b[3]);
    }
    (rise.max(0.0), fall.max(0.0))
}

/// Which end of a rotated axis-tick label should pin to the tick
/// (sipemu/ggplot-rs#163): `1.0` (`TextAnchor::End`, the label's *last*
/// character at the tick) or `0.0` (`TextAnchor::Start`, its *first*
/// character at the tick) — whichever keeps the label's *far* end swinging
/// away from the panel instead of into it.
///
/// `dir` is `draw_x_axis`'s own tick direction: `+1.0` for a bottom axis
/// (the panel sits above the axis line), `-1.0` for a top axis (the panel
/// sits below it). The right anchor depends on *both* the rotation's
/// direction and which side the panel is on — not the angle alone: a
/// counter-clockwise (SVG-negative) angle swings its far end away from the
/// panel when its *last* character is pinned, when the panel is above
/// (bottom axis); a clockwise (SVG-positive) angle needs the opposite —
/// its *first* character pinned — for the same effect. Both flip for a top
/// axis, where "away from the panel" points the other way. Concretely:
/// `End` exactly when `dir * sin(angle) < 0`.
///
/// Picking the wrong one for a given sign pushes the whole label away from
/// the tick instead (0.22.1's bug, on a bottom axis): the tick-anchored
/// end stays near the tick but the free end swings into the previous
/// category's space (or, with the direction only half-fixed, the anchor
/// itself gets pushed far from the tick to clear the panel on a top axis)
/// instead of the near-axis end simply staying at the tick either way.
///
/// Only meaningful when the caller already knows the label is rotated
/// (`angle.abs() > 10.0`); the flat case uses `0.5` (`TextAnchor::Middle`)
/// unconditionally, never this function.
pub(crate) fn rotated_anchor_frac(angle: f64, dir: f64) -> f64 {
    if dir * angle.to_radians().sin() < 0.0 {
        1.0
    } else {
        0.0
    }
}

/// ggplot2's `check.overlap` draw order (its internal `label_priority`):
/// always keep the first and last breaks, then recursively bisect the
/// remaining range (midpoint first, then its two halves), so thinning drops
/// the *most crowded middle* first rather than every other label. R parity
/// (ggplot2 4.0.3): `axis_label_priority(5)` is `1 5 3 2 4`;
/// `axis_label_priority(10)` is `1 10 5 3 2 4 7 6 8 9` (1-indexed).
pub(crate) fn axis_label_priority(n: usize) -> Vec<usize> {
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![1];
    }
    let mut out = vec![1, n];
    bisect(2, n - 1, &mut out);
    out
}

/// 1-indexed inclusive range `[lo, hi]`: midpoint first (floor division),
/// then the left half, then the right half.
fn bisect(lo: usize, hi: usize, out: &mut Vec<usize>) {
    if lo > hi {
        return;
    }
    let mid = (lo + hi) / 2;
    out.push(mid);
    if mid > lo {
        bisect(lo, mid - 1, out);
    }
    bisect(mid + 1, hi, out);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_matches_r_ggplot2_4_0_3() {
        assert_eq!(axis_label_priority(5), vec![1, 5, 3, 2, 4]);
        assert_eq!(axis_label_priority(10), vec![1, 10, 5, 3, 2, 4, 7, 6, 8, 9]);
    }

    #[test]
    fn priority_edge_cases() {
        assert_eq!(axis_label_priority(0), Vec::<usize>::new());
        assert_eq!(axis_label_priority(1), vec![1]);
        assert_eq!(axis_label_priority(2), vec![1, 2]);
    }

    #[test]
    fn text_width_matches_consumer_em_table() {
        assert_eq!(text_width("", 10.0), 0.0);
        assert!((text_width("i", 10.0) - 2.7).abs() < 1e-9);
        assert!((text_width("M", 10.0) - 8.6).abs() < 1e-9);
    }

    #[test]
    fn label_bbox_unrotated_middle_anchor() {
        let b = label_bbox("ab", 10.0, 100.0, 50.0, 0.0, 0.5);
        let w = text_width("ab", 10.0);
        assert!((b[0] - (100.0 - w / 2.0)).abs() < 1e-9);
        assert!((b[2] - (100.0 + w / 2.0)).abs() < 1e-9);
        assert!((b[1] - (50.0 - 6.0)).abs() < 1e-9);
        assert!((b[3] - (50.0 + 6.0)).abs() < 1e-9);
    }
}
