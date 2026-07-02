use crate::render::Rect;
use crate::scale::transform::ScaleTransform;

use super::{AxisSpan, Coord};

/// Transformed Cartesian coordinates (R's `coord_trans`).
///
/// Unlike a *scale* transform (which transforms data before stats), this warps
/// the **coordinate space at draw time** — stats are computed on the raw data
/// but drawn on a non-linear axis. Data points, gridlines, and ticks all pass
/// through `transform`, so they warp consistently and the tick labels keep their
/// original (untransformed) values.
///
/// The trained data domain is supplied via [`Coord::set_domains`] during build,
/// so the warp is faithful. If a transform is invalid over the domain (e.g.
/// `log10` with a non-positive minimum) the axis falls back to linear.
pub struct CoordTrans {
    x: Option<ScaleTransform>,
    y: Option<ScaleTransform>,
    x_span: Option<AxisSpan>,
    y_span: Option<AxisSpan>,
}

impl CoordTrans {
    pub fn new(x: Option<ScaleTransform>, y: Option<ScaleTransform>) -> Self {
        CoordTrans {
            x,
            y,
            x_span: None,
            y_span: None,
        }
    }
}

/// Warp a normalized position `n` by applying `trans` across the domain.
///
/// The scale maps data linearly onto `[pmin, pmax]` (with expansion margins
/// outside), so we invert that to recover the data value, apply the transform
/// within the domain, and place the result back into `[pmin, pmax]`. Positions
/// in the expansion margins, or where the transform is undefined, stay linear.
fn warp(n: f64, trans: &Option<ScaleTransform>, span: Option<AxisSpan>) -> f64 {
    let (trans, s) = match (trans, span) {
        (Some(t), Some(s)) if s.max > s.min && (s.pmax - s.pmin).abs() > 1e-12 => (t, s),
        _ => return n,
    };
    let v = s.min + (n - s.pmin) / (s.pmax - s.pmin) * (s.max - s.min);
    if v < s.min || v > s.max {
        return n; // expansion margin — keep linear
    }
    let (fmin, fmax, fv) = (trans.apply(s.min), trans.apply(s.max), trans.apply(v));
    if fmin.is_finite() && fmax.is_finite() && fv.is_finite() && (fmax - fmin).abs() > 1e-12 {
        let tf = (fv - fmin) / (fmax - fmin);
        s.pmin + tf * (s.pmax - s.pmin)
    } else {
        n
    }
}

impl Coord for CoordTrans {
    fn transform(&self, point: (f64, f64), plot_area: &Rect) -> (f64, f64) {
        let wx = warp(point.0, &self.x, self.x_span);
        let wy = warp(point.1, &self.y, self.y_span);
        let px = plot_area.x + wx * plot_area.width;
        let py = plot_area.y + (1.0 - wy) * plot_area.height;
        (px, py)
    }

    fn set_domains(&mut self, x: Option<AxisSpan>, y: Option<AxisSpan>) {
        self.x_span = x;
        self.y_span = y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        }
    }

    fn span(min: f64, max: f64) -> AxisSpan {
        AxisSpan {
            min,
            max,
            pmin: 0.0,
            pmax: 1.0,
        }
    }

    #[test]
    fn log_warps_axis_nonlinearly() {
        let mut c = CoordTrans::new(None, Some(ScaleTransform::Log10));
        c.set_domains(None, Some(span(1.0, 100.0)));
        // 10 is the geometric midpoint of [1, 100] → mid height on a log axis.
        // Linear n for v=10 is (10-1)/99 ≈ 0.0909; the warp pushes it to 0.5.
        let n10 = (10.0 - 1.0) / 99.0;
        let (_, py) = c.transform((0.0, n10), &area());
        assert!((py - 50.0).abs() < 1.0, "py = {py}, expected ~50");
    }

    #[test]
    fn falls_back_to_linear_without_span_or_invalid() {
        // No span set → linear passthrough.
        let c = CoordTrans::new(None, Some(ScaleTransform::Log10));
        let (_, py) = c.transform((0.0, 0.25), &area());
        assert!((py - 75.0).abs() < 1e-9); // (1 - 0.25)*100

        // Non-positive domain for log → linear fallback (no NaN/inf).
        let mut c2 = CoordTrans::new(Some(ScaleTransform::Log10), None);
        c2.set_domains(Some(span(-5.0, 5.0)), None);
        let (px, _) = c2.transform((0.5, 0.0), &area());
        assert!(px.is_finite());
    }
}
