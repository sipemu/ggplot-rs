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
