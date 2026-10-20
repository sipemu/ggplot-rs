//! Arrowheads for segment/curve/path/line, modelled on R grid's `arrow()`.
//!
//! `Arrow` only describes *where* and *how* a head should be drawn; the
//! geometry is computed in pixel space from an already-projected polyline by
//! [`heads`], and drawn by [`draw_heads`]. Geoms call these two functions —
//! see `geom::segment`/`geom::curve`/`geom::path`/`geom::line` (B1-3).

use crate::render::backend::{DrawBackend, LineEnd, LineStyle, Linetype, RectStyle};
use crate::render::RenderError;

/// Which end(s) of a path get an arrowhead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrowEnds {
    /// Head at the end of the path (the default — like R's `arrow()`).
    Last,
    /// Head at the start of the path.
    First,
    /// A head at both ends.
    Both,
}

/// Arrowhead shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArrowType {
    /// Two open strokes meeting at the tip (not filled).
    Open,
    /// A filled triangle.
    Closed,
}

/// Describes an arrowhead to draw at the end(s) of a line-like geom,
/// following R grid's `arrow()`. `position` and `length_per_linewidth` are
/// ggplot-rs extensions, not part of grid's API.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Arrow {
    /// Half-angle of the arrowhead, in degrees. The full opening angle
    /// (between the two wings) is `2 * angle`.
    pub angle: f64,
    /// Wing length in pixels.
    pub length: f64,
    /// Which end(s) of the path get a head. Default [`ArrowEnds::Last`].
    pub ends: ArrowEnds,
    /// Open (stroked wings) or closed (filled triangle). Default
    /// [`ArrowType::Open`].
    pub kind: ArrowType,
    /// Where along the path's arc length the head sits, as a fraction in
    /// `[0, 1]` measured from the path's start (`1.0`, the default, is the
    /// very end for `ArrowEnds::Last`, the very start for `ArrowEnds::First`).
    /// Extension: not part of grid's `arrow()`.
    pub position: f64,
    /// When set above 0, the effective wing length is
    /// `max(length, length_per_linewidth * linewidth)`, so arrowheads scale
    /// up with a thick line. Extension: not part of grid's `arrow()`.
    pub length_per_linewidth: f64,
}

impl Default for Arrow {
    fn default() -> Self {
        Arrow {
            angle: 30.0,
            length: 18.0,
            ends: ArrowEnds::Last,
            kind: ArrowType::Open,
            position: 1.0,
            length_per_linewidth: 0.0,
        }
    }
}

impl Arrow {
    /// Half-angle of the wings, in degrees. Default `30.0`.
    pub fn angle(mut self, degrees: f64) -> Self {
        self.angle = degrees;
        self
    }

    /// Wing length in pixels. Default `18.0`.
    pub fn length(mut self, px: f64) -> Self {
        self.length = px;
        self
    }

    /// Which end(s) of the path get a head.
    pub fn ends(mut self, ends: ArrowEnds) -> Self {
        self.ends = ends;
        self
    }

    /// Open (stroked wings) or closed (filled triangle).
    pub fn kind(mut self, kind: ArrowType) -> Self {
        self.kind = kind;
        self
    }

    /// Shorthand for `.kind(ArrowType::Closed)`.
    pub fn closed(mut self) -> Self {
        self.kind = ArrowType::Closed;
        self
    }

    /// Shorthand for `.kind(ArrowType::Open)`.
    pub fn open(mut self) -> Self {
        self.kind = ArrowType::Open;
        self
    }

    /// Where along the path's arc length the head sits — see the
    /// [`position`](Self::position) field doc.
    pub fn position(mut self, fraction: f64) -> Self {
        self.position = fraction;
        self
    }

    /// Scale the effective wing length with the stroke width — see the
    /// [`length_per_linewidth`](Self::length_per_linewidth) field doc.
    pub fn length_per_linewidth(mut self, k: f64) -> Self {
        self.length_per_linewidth = k;
        self
    }
}

/// Build an [`Arrow`] with the default settings (open, 30° half-angle, 18px
/// wings, a head at the end): `arrow()`, or `arrow().closed().angle(20.0)`
/// for a filled, narrower head.
pub fn arrow() -> Arrow {
    Arrow::default()
}

/// One computed arrowhead, in the same pixel space as `path_px`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ArrowHead {
    pub tip: (f64, f64),
    pub left: (f64, f64),
    pub right: (f64, f64),
}

fn rotate((x, y): (f64, f64), radians: f64) -> (f64, f64) {
    let (s, c) = radians.sin_cos();
    (x * c - y * s, x * s + y * c)
}

/// Compute the arrowhead(s) for a polyline already in pixel space.
///
/// Returns no heads for a path with fewer than two points, any non-finite
/// point, or zero total arc length (a degenerate, single-point path).
pub(crate) fn heads(path_px: &[(f64, f64)], arrow: &Arrow, linewidth: f64) -> Vec<ArrowHead> {
    if path_px.len() < 2
        || path_px
            .iter()
            .any(|(x, y)| !x.is_finite() || !y.is_finite())
    {
        return Vec::new();
    }
    let mut cum = vec![0.0f64; path_px.len()];
    for i in 1..path_px.len() {
        let (x0, y0) = path_px[i - 1];
        let (x1, y1) = path_px[i];
        cum[i] = cum[i - 1] + ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
    }
    let total = cum[path_px.len() - 1];
    if total <= 0.0 || !total.is_finite() {
        return Vec::new();
    }

    let effective_length = arrow.length.max(arrow.length_per_linewidth * linewidth);

    // Point and forward-pointing unit tangent at arc-length fraction `frac`.
    let point_at = |frac: f64| -> ((f64, f64), (f64, f64)) {
        let target = frac.clamp(0.0, 1.0) * total;
        let mut idx = 1;
        while idx < cum.len() - 1 && cum[idx] < target {
            idx += 1;
        }
        let seg_start = cum[idx - 1];
        let seg_end = cum[idx];
        let seg_len = seg_end - seg_start;
        let t = if seg_len > 0.0 {
            ((target - seg_start) / seg_len).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let (x0, y0) = path_px[idx - 1];
        let (x1, y1) = path_px[idx];
        let px = x0 + (x1 - x0) * t;
        let py = y0 + (y1 - y0) * t;
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len = (dx * dx + dy * dy).sqrt();
        let unit = if len > 0.0 {
            (dx / len, dy / len)
        } else {
            (0.0, 0.0)
        };
        ((px, py), unit)
    };

    let make_head = |frac: f64, forward: bool| -> Option<ArrowHead> {
        let (tip, tangent) = point_at(frac);
        let dir = if forward {
            tangent
        } else {
            (-tangent.0, -tangent.1)
        };
        if dir.0 == 0.0 && dir.1 == 0.0 {
            return None;
        }
        let back = (-dir.0, -dir.1);
        let angle = arrow.angle.to_radians();
        let l = rotate(back, angle);
        let r = rotate(back, -angle);
        Some(ArrowHead {
            tip,
            left: (
                tip.0 + effective_length * l.0,
                tip.1 + effective_length * l.1,
            ),
            right: (
                tip.0 + effective_length * r.0,
                tip.1 + effective_length * r.1,
            ),
        })
    };

    let mut out = Vec::new();
    match arrow.ends {
        ArrowEnds::Last => out.extend(make_head(arrow.position, true)),
        ArrowEnds::First => out.extend(make_head(1.0 - arrow.position, false)),
        ArrowEnds::Both => {
            out.extend(make_head(arrow.position, true));
            out.extend(make_head(1.0 - arrow.position, false));
        }
    }
    out
}

/// Draw computed arrowheads, marking them as the `"arrow"` mark part (no
/// `<title>`, `data-part="arrow"`, `aria-hidden="true"` on the SVG backend —
/// see [`DrawBackend::set_mark_part`]). `Closed` draws a filled triangle with
/// fill equal to the stroke colour; `Open` draws the two wings as a single
/// 3-point open polyline.
pub(crate) fn draw_heads(
    backend: &mut dyn DrawBackend,
    heads: &[ArrowHead],
    arrow: &Arrow,
    color: (u8, u8, u8),
    alpha: f64,
    linewidth: f64,
) -> Result<(), RenderError> {
    for h in heads {
        backend.set_mark_part(Some("arrow"));
        let result = match arrow.kind {
            ArrowType::Closed => backend.draw_polygon(
                &[h.tip, h.left, h.right],
                &RectStyle {
                    fill: Some(color),
                    stroke: Some(color),
                    stroke_width: linewidth,
                    alpha,
                    clip: true,
                },
            ),
            ArrowType::Open => backend.draw_line(
                &[h.left, h.tip, h.right],
                &LineStyle {
                    color,
                    alpha,
                    width: linewidth,
                    linetype: Linetype::Solid,
                    lineend: LineEnd::Round,
                },
            ),
        };
        backend.set_mark_part(None);
        result?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn angle_between(a: (f64, f64), b: (f64, f64)) -> f64 {
        let dot = a.0 * b.0 + a.1 * b.1;
        let na = (a.0 * a.0 + a.1 * a.1).sqrt();
        let nb = (b.0 * b.0 + b.1 * b.1).sqrt();
        (dot / (na * nb)).clamp(-1.0, 1.0).acos()
    }

    #[test]
    fn tip_lands_on_the_endpoint_at_position_1() {
        let path = [(0.0, 0.0), (10.0, 0.0)];
        let hs = heads(&path, &Arrow::default(), 1.0);
        assert_eq!(hs.len(), 1);
        assert!((hs[0].tip.0 - 10.0).abs() < 1e-9);
        assert!((hs[0].tip.1 - 0.0).abs() < 1e-9);
    }

    #[test]
    fn opening_angle_is_twice_the_half_angle() {
        let path = [(0.0, 0.0), (10.0, 0.0)];
        let a = Arrow::default().angle(30.0);
        let hs = heads(&path, &a, 1.0);
        assert_eq!(hs.len(), 1);
        let h = hs[0];
        let v_left = (h.left.0 - h.tip.0, h.left.1 - h.tip.1);
        let v_right = (h.right.0 - h.tip.0, h.right.1 - h.tip.1);
        let opening = angle_between(v_left, v_right).to_degrees();
        assert!((opening - 60.0).abs() < 1e-6, "opening = {opening}");
    }

    #[test]
    fn both_ends_give_two_heads() {
        let path = [(0.0, 0.0), (10.0, 0.0)];
        let a = Arrow::default().ends(ArrowEnds::Both);
        let hs = heads(&path, &a, 1.0);
        assert_eq!(hs.len(), 2);
    }

    #[test]
    fn position_half_gives_the_midpoint() {
        let path = [(0.0, 0.0), (10.0, 0.0)];
        let a = Arrow::default().position(0.5);
        let hs = heads(&path, &a, 1.0);
        assert_eq!(hs.len(), 1);
        assert!((hs[0].tip.0 - 5.0).abs() < 1e-9);
    }

    #[test]
    fn length_per_linewidth_scales_the_wings() {
        let path = [(0.0, 0.0), (10.0, 0.0)];
        let a = Arrow::default().length(5.0).length_per_linewidth(2.2);
        let hs = heads(&path, &a, 10.0); // effective length = max(5, 2.2*10) = 22
        assert_eq!(hs.len(), 1);
        let h = hs[0];
        let d = ((h.left.0 - h.tip.0).powi(2) + (h.left.1 - h.tip.1).powi(2)).sqrt();
        assert!((d - 22.0).abs() < 1e-6, "wing length = {d}");
    }

    #[test]
    fn zero_length_path_gives_no_heads() {
        let path = [(5.0, 5.0), (5.0, 5.0)];
        assert!(heads(&path, &Arrow::default(), 1.0).is_empty());
    }

    #[test]
    fn non_finite_path_gives_no_heads() {
        let path = [(f64::NAN, 0.0), (1.0, 1.0)];
        assert!(heads(&path, &Arrow::default(), 1.0).is_empty());
        let path2 = [(0.0, 0.0), (f64::INFINITY, 1.0)];
        assert!(heads(&path2, &Arrow::default(), 1.0).is_empty());
    }

    #[test]
    fn first_end_places_tip_at_the_start() {
        let path = [(0.0, 0.0), (10.0, 0.0)];
        let a = Arrow::default().ends(ArrowEnds::First);
        let hs = heads(&path, &a, 1.0);
        assert_eq!(hs.len(), 1);
        assert!((hs[0].tip.0 - 0.0).abs() < 1e-9);
    }

    #[test]
    fn kind_and_open_builders_set_the_arrow_type() {
        let a = Arrow::default().kind(ArrowType::Closed);
        assert_eq!(a.kind, ArrowType::Closed);
        // `.open()` is the shorthand back to the (already-default) Open kind.
        let a = a.open();
        assert_eq!(a.kind, ArrowType::Open);
    }

    /// A zero-length segment in the *middle* of an otherwise normal path
    /// (not a wholly degenerate path) must not produce a NaN/garbage tip —
    /// `point_at`'s `seg_len <= 0` fallback is exercised here.
    #[test]
    fn a_zero_length_segment_mid_path_does_not_produce_a_degenerate_head() {
        let path = [(0.0, 0.0), (0.0, 0.0), (10.0, 0.0)];
        let hs = heads(&path, &Arrow::default(), 1.0);
        assert_eq!(hs.len(), 1);
        assert!(hs[0].tip.0.is_finite() && hs[0].tip.1.is_finite());
        // position 1.0 (default) still lands on the true endpoint.
        assert!((hs[0].tip.0 - 10.0).abs() < 1e-9);
    }

    /// Positioning the head exactly on a degenerate (zero-length) leading
    /// segment gives a zero tangent — `make_head` must recognise it and
    /// produce no head, not a NaN direction.
    #[test]
    fn position_on_a_degenerate_segment_gives_no_head() {
        let path = [(0.0, 0.0), (0.0, 0.0), (10.0, 0.0)];
        let a = Arrow::default().position(0.0);
        assert!(heads(&path, &a, 1.0).is_empty());
    }
}
