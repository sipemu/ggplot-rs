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

/// Shorten `label` with a trailing ellipsis ("…") until it (plus the
/// ellipsis) fits within `max_width` px at font `size` (#159: a vertical
/// multi-column legend grid that still overflows after exhausting column
/// reduction falls back to truncating its labels). `None` means `label`
/// already fits — no truncation needed, so the caller keeps the
/// byte-identical untruncated path. A `max_width` too narrow for even a
/// bare "…" still returns `Some("…")` — the best any truncation can do —
/// rather than an empty string.
pub fn truncate_to_width(label: &str, max_width: f64, size: f64) -> Option<String> {
    if text_width(label, size) <= max_width {
        return None;
    }
    let chars: Vec<char> = label.chars().collect();
    for keep in (0..chars.len()).rev() {
        let candidate: String = chars[..keep].iter().collect::<String>() + "…";
        if text_width(&candidate, size) <= max_width {
            return Some(candidate);
        }
    }
    Some("…".to_string())
}

/// Characters after which a hard break in [`wrap_text`] is preferred over a
/// blind character-count cut — the separators a URL or path is built from
/// (sipemu/ggplot-rs#264: a bare URL has no whitespace at all, so without
/// this it fell back to slicing mid-word, e.g. `"eodatenzentr"` / `"um.de"`).
const WRAP_HARD_BREAK_AFTER: [char; 7] = ['/', '.', '_', '-', '?', '&', '='];

/// Word-wraps `text` to lines no wider than `max_width` px at font `size`,
/// measured with [`text_width`] (ggtext / `str_wrap` behaviour: break at
/// whitespace; a single word wider than `max_width` on its own is hard-split
/// so it never overflows the box). Returns `vec![String::new()]` for empty
/// input and otherwise always at least one line, so a caller can take
/// `lines.len()` as the wrapped block's line count without a special case.
///
/// `max_width <= 0.0` (or non-finite) disables wrapping — each word still
/// splits only when it alone would overflow a *positive* width, so a
/// degenerate width never panics or infinite-loops; it returns `text`
/// unsplit as a single line (matching "no wrap requested").
pub fn wrap_text(text: &str, max_width: f64, size: f64) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    if !max_width.is_finite() || max_width <= 0.0 {
        return vec![text.to_string()];
    }
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in text.split_whitespace() {
        let mut word = word.to_string();
        // A single word that alone overflows max_width: hard-break it by
        // character rather than let it overflow the box.
        while text_width(&word, size) > max_width {
            let chars: Vec<char> = word.chars().collect();
            if chars.len() <= 1 {
                break;
            }
            let mut cut = chars.len() - 1;
            while cut > 1 && text_width(&chars[..cut].iter().collect::<String>(), size) > max_width
            {
                cut -= 1;
            }
            // Prefer breaking right after the last `/ . _ - ? & =` that
            // still fits, rather than wherever the width-based cut above
            // landed — but only when that leaves a non-trivial chunk (at
            // least a third of what fits), so a separator near the very
            // start of the fitting prefix doesn't produce a near-empty
            // line (#264).
            if let Some(break_at) = chars[..cut]
                .iter()
                .rposition(|c| WRAP_HARD_BREAK_AFTER.contains(c))
            {
                let candidate = break_at + 1;
                if candidate > 0 && (candidate as f64) >= (cut as f64) / 3.0 {
                    cut = candidate;
                }
            }
            if !cur.is_empty() {
                lines.push(std::mem::take(&mut cur));
            }
            lines.push(chars[..cut].iter().collect());
            word = chars[cut..].iter().collect();
        }
        if cur.is_empty() {
            cur = word;
        } else {
            let candidate = format!("{cur} {word}");
            if text_width(&candidate, size) <= max_width {
                cur = candidate;
            } else {
                lines.push(std::mem::replace(&mut cur, word));
            }
        }
    }
    if !cur.is_empty() || lines.is_empty() {
        lines.push(cur);
    }
    lines
}

/// The widest wrapped line's width (px) at font `size` — the natural width
/// of a wrapped text block, e.g. for a [`GeomLabel`](crate::geom::text::GeomLabel)
/// background box sized to fit its wrapped text.
pub fn wrapped_block_width(lines: &[String], size: f64) -> f64 {
    lines
        .iter()
        .map(|l| text_width(l, size))
        .fold(0.0, f64::max)
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
    fn truncate_to_width_keeps_short_labels_untouched() {
        assert_eq!(truncate_to_width("ab", 1000.0, 10.0), None);
    }

    #[test]
    fn truncate_to_width_shortens_with_an_ellipsis() {
        let full = "a very long legend label indeed";
        let w = text_width(full, 10.0);
        let truncated = truncate_to_width(full, w / 2.0, 10.0).expect("should truncate");
        assert!(truncated.ends_with('…'));
        assert!(text_width(&truncated, 10.0) <= w / 2.0);
        assert!(truncated.chars().count() < full.chars().count());
    }

    #[test]
    fn truncate_to_width_falls_back_to_a_bare_ellipsis() {
        // Even "…" alone doesn't fit a near-zero width — still the best
        // truncation can do, not an empty string.
        assert_eq!(
            truncate_to_width("hello", 0.01, 10.0),
            Some("…".to_string())
        );
    }

    #[test]
    fn text_width_matches_consumer_em_table() {
        assert_eq!(text_width("", 10.0), 0.0);
        assert!((text_width("i", 10.0) - 2.7).abs() < 1e-9);
        assert!((text_width("M", 10.0) - 8.6).abs() < 1e-9);
    }

    #[test]
    fn wrap_text_breaks_at_whitespace_within_width() {
        let lines = wrap_text("alpha beta gamma delta", 1000.0, 10.0);
        assert_eq!(lines, vec!["alpha beta gamma delta".to_string()]);
    }

    #[test]
    fn wrap_text_wraps_to_multiple_lines_when_narrow() {
        let w = text_width("alpha beta", 10.0);
        let lines = wrap_text("alpha beta gamma delta", w + 1.0, 10.0);
        assert!(lines.len() >= 2, "expected wrapping, got {lines:?}");
        for l in &lines {
            assert!(
                text_width(l, 10.0) <= w + 1.0 + 1e-6,
                "line {l:?} overflows width"
            );
        }
        assert_eq!(lines.join(" "), "alpha beta gamma delta");
    }

    #[test]
    fn wrap_text_hard_breaks_a_single_overlong_word() {
        let lines = wrap_text("supercalifragilisticexpialidocious", 40.0, 10.0);
        assert!(lines.len() > 1);
        assert_eq!(lines.concat(), "supercalifragilisticexpialidocious");
        for l in &lines {
            assert!(text_width(l, 10.0) <= 40.0 + 1e-6);
        }
    }

    #[test]
    fn wrap_text_hard_break_flushes_a_pending_line_first() {
        // "ab" fits its own line (`cur` is non-empty) before the second
        // word triggers a hard break — the hard-break path must flush
        // `cur` as its own line rather than lose or merge it.
        let lines = wrap_text("ab supercalifragilisticexpialidocious", 40.0, 10.0);
        assert_eq!(lines[0], "ab");
        assert!(lines.len() > 2, "{lines:?}");
        assert_eq!(lines[1..].concat(), "supercalifragilisticexpialidocious");
        for l in &lines {
            assert!(text_width(l, 10.0) <= 40.0 + 1e-6);
        }
    }

    #[test]
    fn wrap_text_prefers_url_separators_over_a_blind_char_cut() {
        // A bare URL (no whitespace at all, #264's repro) should break
        // after `/ . _ - ? & =`, not wherever a raw character-count cut
        // happens to land.
        let url =
            "https://sgx.geodatenzentrum.de/web_public/gdz/datenquellen/datenquellen_vg_nuts.pdf";
        let lines = wrap_text(url, 80.0, 7.0);
        assert!(lines.len() > 1, "expected wrapping, got {lines:?}");
        assert_eq!(lines.concat(), url);
        for l in &lines {
            assert!(
                text_width(l, 7.0) <= 80.0 + 1e-6,
                "line {l:?} overflows width"
            );
        }
        // Every line but the last should end right after one of the
        // preferred separators, not mid-token.
        for l in &lines[..lines.len() - 1] {
            let last = l.chars().last().unwrap();
            assert!(
                WRAP_HARD_BREAK_AFTER.contains(&last),
                "line {l:?} did not break after a preferred separator"
            );
        }
    }

    #[test]
    fn wrap_text_still_hard_breaks_when_no_separator_exists() {
        // No `/ . _ - ? & =` anywhere: falls back to the plain
        // character-count cut, same as before #264.
        let lines = wrap_text("supercalifragilisticexpialidocious", 40.0, 10.0);
        assert!(lines.len() > 1);
        assert_eq!(lines.concat(), "supercalifragilisticexpialidocious");
    }

    #[test]
    fn wrap_text_ignores_a_separator_too_close_to_the_start() {
        // A single `-` right after the first character shouldn't force a
        // one-character line; the width-based cut should stand instead.
        let lines = wrap_text("a-supercalifragilisticexpialidocious", 40.0, 10.0);
        assert_ne!(lines[0], "a-", "separator too close to start was used");
        assert_eq!(lines.concat(), "a-supercalifragilisticexpialidocious");
    }

    #[test]
    fn wrap_text_width_too_narrow_for_even_one_character_still_terminates() {
        // A width narrower than any single character: hard-breaking can't
        // cut further (`chars.len() <= 1`), so it gives up and keeps that
        // one character whole rather than looping forever.
        let lines = wrap_text("i", 0.5, 10.0);
        assert_eq!(lines, vec!["i".to_string()]);
    }

    #[test]
    fn wrap_text_empty_input_is_one_empty_line() {
        assert_eq!(wrap_text("", 100.0, 10.0), vec![String::new()]);
    }

    #[test]
    fn wrap_text_non_positive_width_disables_wrapping() {
        assert_eq!(
            wrap_text("alpha beta gamma", 0.0, 10.0),
            vec!["alpha beta gamma".to_string()]
        );
        assert_eq!(
            wrap_text("alpha beta gamma", -5.0, 10.0),
            vec!["alpha beta gamma".to_string()]
        );
    }

    #[test]
    fn wrapped_block_width_is_the_widest_line() {
        let lines = vec!["short".to_string(), "a longer line here".to_string()];
        let w = wrapped_block_width(&lines, 10.0);
        assert_eq!(w, text_width("a longer line here", 10.0));
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
