//! Shared geometry for stacking panel-corner "furniture" clear of each
//! other — `annotation_scale()`/`annotation_attribution()`'s scale bar and
//! attribution text, `bivariate_legend()`'s square guide, and an ordinary
//! legend placed with `legend_position_inside` — so two of them anchored
//! to the same or adjacent corners don't draw on top of one another
//! (sipemu/ggplot-rs#273).
//!
//! Each corner geom draws in plot-layer order (the order layers were added
//! to the plot). Before drawing, it asks [`DrawBackend::corner_claims`]
//! what boxes earlier geoms in this render pass already placed,
//! [`stack_clear`]s its own measured box away from its own anchor edge
//! until it no longer overlaps any of them, draws at the (possibly
//! shifted) result, then registers that box with
//! [`DrawBackend::claim_corner_box`] so a later geom in the same render
//! pass sees it too.
//!
//! An ordinary inside legend (`legend_position_inside`) draws *after*
//! every geom (`renderer.rs`'s draw order: layers, then the legend), so
//! there's no later geom for it to defer to in the usual way — instead
//! its box is pre-claimed *before* any geom draws, computed the same way
//! `draw_legend_planned` itself positions it (see
//! `guide::legend::inside_legend_box`), so attribution/scale-bar/bivariate
//! geoms drawn first still see it and stack clear of it.
//!
//! [`DrawBackend::claim_corner_box`]: crate::render::backend::DrawBackend::claim_corner_box
//! [`DrawBackend::corner_claims`]: crate::render::backend::DrawBackend::corner_claims

use super::Rect;

/// `true` if `a` and `b` overlap with positive area — touching edges
/// (sharing a boundary with zero-width overlap) don't count, so two items
/// placed flush against each other aren't flagged as colliding.
pub fn rects_intersect(a: &Rect, b: &Rect) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

/// Real breathing room left between two stacked boxes, beyond exact
/// zero-gap touching (#273's real-world repro, DataZooDE/anofox-
/// visualization#549 on delivery-coverage.sql: `GeomAttribution`'s own
/// box height is only an *approximation* of its real ink extent — a
/// fixed ascent/descent fraction of `text_size`, not the real font's
/// metrics — so two boxes `stack_clear` leaves exactly touching can still
/// render with a few visible pixels of overlap once real glyphs (a
/// taller x-height, a letter with a true descender, …) are drawn.
/// Shifting a little past exact contact absorbs that slack, the same
/// reasoning `geom::annotation_scale::ATTRIBUTION_FIT_SAFETY_MARGIN`
/// already applies to `fit`'s own width estimate.
const CORNER_STACK_GAP: f64 = 4.0;

/// Shifts `rect` along the y axis, away from its own anchor edge, just far
/// enough to clear every rect in `claims` it would otherwise overlap, plus
/// [`CORNER_STACK_GAP`] of real clearance — "stack them" (#273). `away_sign`
/// is the direction away from the anchor edge: `-1.0` shifts up (decreases
/// y), the right direction for a bottom-anchored corner's items (they
/// stack upward, away from the bottom edge); any other value shifts down
/// (increases y), for a top-anchored corner. A claim with no horizontal
/// overlap with `rect` is never a reason to move (`rects_intersect`
/// requires both axes).
///
/// Iterates a bounded number of passes (`claims.len() + 1`): clearing one
/// claim can push `rect` into a *new* overlap with another claim it
/// already cleared in an earlier pass (claims aren't assumed sorted, or
/// non-overlapping with each other), so each pass re-checks every claim;
/// one extra pass beyond the claim count is enough to reach a fixed point
/// for any realistic claim count (a handful of corner items at once), and
/// a pass that clears nothing stops early — the common case is one pass.
///
/// `bounds` (normally the panel's own `plot_area`) clamps the final
/// result: stacking never pushes `rect` so far that it leaves `bounds`
/// entirely (#273's DataZooDE/anofox-visualization#549 repro — a
/// bivariate legend's tall rotated caption, in a panel too short for
/// every corner item to stack fully clear of it — would otherwise walk
/// the next item up past the panel's own top edge and off the map
/// entirely, strictly worse than the residual overlap it was avoiding).
/// When `rect` is taller than `bounds` itself, it's pinned to `bounds`'s
/// own top rather than centred or otherwise guessed at.
pub fn stack_clear(mut rect: Rect, away_sign: f64, claims: &[Rect], bounds: &Rect) -> Rect {
    let away_sign = if away_sign < 0.0 { -1.0 } else { 1.0 };
    for _ in 0..=claims.len() {
        let mut shifted = false;
        for claim in claims {
            if !rects_intersect(&rect, claim) {
                continue;
            }
            let delta = if away_sign < 0.0 {
                // Shift up until rect's bottom edge sits CORNER_STACK_GAP
                // above claim's top.
                (rect.y + rect.height) - claim.y + CORNER_STACK_GAP
            } else {
                // Shift down until rect's top edge sits CORNER_STACK_GAP
                // below claim's bottom.
                (claim.y + claim.height) - rect.y + CORNER_STACK_GAP
            };
            if delta > 0.0 {
                rect.y += away_sign * delta;
                shifted = true;
            }
        }
        if !shifted {
            break;
        }
    }
    let max_y = (bounds.y + bounds.height - rect.height).max(bounds.y);
    rect.y = rect.y.clamp(bounds.y, max_y);
    rect
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect {
            x,
            y,
            width: w,
            height: h,
        }
    }

    /// A generous panel, big enough that none of the tests below that
    /// don't specifically exercise clamping ever hit it.
    fn bounds() -> Rect {
        r(-1000.0, -1000.0, 2000.0, 2000.0)
    }

    #[test]
    fn disjoint_rects_do_not_intersect() {
        assert!(!rects_intersect(
            &r(0.0, 0.0, 10.0, 10.0),
            &r(20.0, 0.0, 10.0, 10.0)
        ));
    }

    #[test]
    fn touching_edges_do_not_count_as_intersecting() {
        assert!(!rects_intersect(
            &r(0.0, 0.0, 10.0, 10.0),
            &r(10.0, 0.0, 10.0, 10.0)
        ));
        assert!(!rects_intersect(
            &r(0.0, 0.0, 10.0, 10.0),
            &r(0.0, 10.0, 10.0, 10.0)
        ));
    }

    #[test]
    fn overlapping_rects_intersect() {
        assert!(rects_intersect(
            &r(0.0, 0.0, 10.0, 10.0),
            &r(5.0, 5.0, 10.0, 10.0)
        ));
    }

    #[test]
    fn stack_clear_with_no_claims_is_a_no_op() {
        let rect = r(0.0, 90.0, 20.0, 10.0);
        let out = stack_clear(rect.clone(), -1.0, &[], &bounds());
        assert_eq!(
            (out.x, out.y, out.width, out.height),
            (rect.x, rect.y, rect.width, rect.height)
        );
    }

    #[test]
    fn stack_clear_shifts_up_away_from_a_bottom_claim() {
        let rect = r(0.0, 90.0, 20.0, 10.0); // y in [90, 100]
        let claim = r(0.0, 95.0, 20.0, 10.0); // y in [95, 105] — overlaps
        let out = stack_clear(rect, -1.0, std::slice::from_ref(&claim), &bounds());
        // A real gap (CORNER_STACK_GAP), not just zero-touching — see
        // that constant's own docs for why.
        assert!(
            out.y + out.height <= claim.y - CORNER_STACK_GAP + 1e-9,
            "out={out:?}"
        );
        assert!(!rects_intersect(&out, &claim));
    }

    #[test]
    fn stack_clear_shifts_down_away_from_a_top_claim() {
        let rect = r(0.0, 10.0, 20.0, 10.0);
        let claim = r(0.0, 5.0, 20.0, 10.0);
        let out = stack_clear(rect, 1.0, std::slice::from_ref(&claim), &bounds());
        assert!(
            out.y >= claim.y + claim.height + CORNER_STACK_GAP - 1e-9,
            "out={out:?}"
        );
        assert!(!rects_intersect(&out, &claim));
    }

    #[test]
    fn stack_clear_ignores_a_horizontally_disjoint_claim() {
        let rect = r(0.0, 90.0, 20.0, 10.0);
        let claim = r(100.0, 90.0, 20.0, 10.0); // same y band, no x overlap
        let out = stack_clear(rect.clone(), -1.0, &[claim], &bounds());
        assert_eq!((out.x, out.y), (rect.x, rect.y));
    }

    #[test]
    fn stack_clear_clears_several_stacked_claims() {
        let rect = r(0.0, 90.0, 20.0, 10.0);
        let claims = [r(0.0, 95.0, 20.0, 10.0), r(0.0, 80.0, 20.0, 10.0)];
        let out = stack_clear(rect, -1.0, &claims, &bounds());
        for c in &claims {
            assert!(!rects_intersect(&out, c), "out={out:?} still hits {c:?}");
        }
    }

    #[test]
    fn stack_clear_handles_overlapping_claims_in_one_pass_or_more() {
        // The two claims themselves overlap each other — stacking past
        // both must still terminate and clear both.
        let rect = r(0.0, 90.0, 20.0, 10.0);
        let claims = [r(0.0, 92.0, 20.0, 10.0), r(0.0, 85.0, 20.0, 12.0)];
        let out = stack_clear(rect, -1.0, &claims, &bounds());
        for c in &claims {
            assert!(!rects_intersect(&out, c), "out={out:?} still hits {c:?}");
        }
    }

    #[test]
    fn stack_clear_never_pushes_the_box_above_the_bounds_top() {
        // #273's DataZooDE/anofox-visualization#549 repro: a claim tall
        // enough (relative to the panel) that fully clearing it would
        // walk `rect` above the panel's own top edge — residual overlap
        // with the claim is better than leaving the panel entirely.
        let panel = r(0.0, 0.0, 100.0, 60.0);
        let rect = r(0.0, 55.0, 20.0, 10.0); // near the panel's bottom
        let claim = r(0.0, -500.0, 20.0, 600.0); // a huge claim filling the panel
        let out = stack_clear(rect, -1.0, std::slice::from_ref(&claim), &panel);
        assert!(out.y >= panel.y - 1e-9, "out={out:?} left the panel's top");
    }

    #[test]
    fn stack_clear_pins_a_box_taller_than_bounds_to_the_top() {
        let panel = r(0.0, 0.0, 100.0, 20.0);
        let rect = r(0.0, 18.0, 20.0, 50.0); // taller than the panel itself
        let out = stack_clear(rect, -1.0, &[], &panel);
        assert_eq!(out.y, panel.y);
    }

    #[test]
    fn stack_clear_without_claims_still_respects_bounds() {
        // A box that starts outside its own bounds (shouldn't normally
        // happen, but the clamp is unconditional) gets pulled back in
        // even with nothing to stack against.
        let panel = r(0.0, 0.0, 100.0, 60.0);
        let rect = r(0.0, -40.0, 20.0, 10.0);
        let out = stack_clear(rect, -1.0, &[], &panel);
        assert!(out.y >= panel.y - 1e-9);
        assert!(out.y + out.height <= panel.y + panel.height + 1e-9);
    }
}
