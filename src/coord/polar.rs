use crate::render::Rect;

use super::Coord;

/// Polar coordinate system.
///
/// Maps one aesthetic to angle and the other to radius.
/// - `theta = "x"` (default): x maps to angle, y to radius (pie charts, wind roses)
/// - `theta = "y"`: y maps to angle, x to radius (Coxcomb charts)
pub struct CoordPolar {
    /// Which variable maps to angle: "x" or "y".
    pub theta: String,
    /// Start angle in radians (0 = 12 o'clock position).
    pub start: f64,
    /// Direction: 1 = clockwise, -1 = counterclockwise.
    pub direction: f64,
    /// Inner radius as a fraction of the outer radius (0 = pie, 0.5 = donut).
    pub inner_radius: f64,
    /// Angle swept by the full theta range, in radians (default `2π`). Set via
    /// [`CoordPolar::with_span`] for partial arcs such as gauges.
    pub span: f64,
}

impl CoordPolar {
    pub fn new() -> Self {
        CoordPolar {
            theta: "x".to_string(),
            start: 0.0,
            direction: 1.0,
            inner_radius: 0.0,
            span: std::f64::consts::TAU,
        }
    }

    /// Sweep the theta range over `[start, end]` (radians, 0 = 12 o'clock,
    /// clockwise positive) instead of the full circle — e.g.
    /// `with_span(-PI/2, PI/2)` for a half-donut gauge opening downward, or
    /// `with_span(-0.75*PI, 0.75*PI)` for a 270° gauge. The partial arc is
    /// scaled and centred to fill the panel.
    pub fn with_span(mut self, start: f64, end: f64) -> Self {
        if start.is_finite() && end.is_finite() && (end - start).abs() > 1e-9 {
            self.start = start;
            self.span = (end - start).abs().min(std::f64::consts::TAU);
            self.direction = if end >= start { 1.0 } else { -1.0 };
        }
        self
    }

    /// Bounding box `(min_x, max_x, min_y, max_y)` of the swept annulus
    /// sector, for a unit outer radius centred at the origin (screen axes:
    /// x right, y down).
    fn unit_bbox(&self) -> (f64, f64, f64, f64) {
        use std::f64::consts::{FRAC_PI_2, TAU};
        let (a0, sweep) = (self.start, self.direction * self.span);
        let pt = |a: f64, r: f64| (r * a.sin(), -r * a.cos());
        let mut pts = vec![
            pt(a0, 1.0),
            pt(a0 + sweep, 1.0),
            pt(a0, self.inner_radius),
            pt(a0 + sweep, self.inner_radius),
        ];
        // Cardinal directions inside the sweep reach the outer extremes.
        let (lo, hi) = if sweep >= 0.0 {
            (a0, a0 + sweep)
        } else {
            (a0 + sweep, a0)
        };
        let mut k = (lo / FRAC_PI_2).ceil();
        while k * FRAC_PI_2 <= hi + 1e-12 && (k * FRAC_PI_2 - lo) <= TAU {
            pts.push(pt(k * FRAC_PI_2, 1.0));
            k += 1.0;
        }
        let mut b = (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        );
        for (x, y) in pts {
            b = (b.0.min(x), b.1.max(x), b.2.min(y), b.3.max(y));
        }
        b
    }

    /// Set the inner radius (fraction of outer, `0.0`..`1.0`) to punch a donut hole.
    pub fn inner_radius(mut self, frac: f64) -> Self {
        self.inner_radius = frac.clamp(0.0, 0.95);
        self
    }

    pub fn theta(mut self, theta: &str) -> Self {
        self.theta = theta.to_string();
        self
    }

    pub fn start(mut self, start: f64) -> Self {
        self.start = start;
        self
    }

    pub fn direction(mut self, dir: f64) -> Self {
        self.direction = dir;
        self
    }
}

impl Default for CoordPolar {
    fn default() -> Self {
        Self::new()
    }
}

impl Coord for CoordPolar {
    fn transform(&self, point: (f64, f64), plot_area: &Rect) -> (f64, f64) {
        let (nx, ny) = point;

        // Determine which normalized value maps to angle vs radius
        let (angle_norm, radius_norm) = if self.theta == "x" {
            (nx, ny)
        } else {
            (ny, nx)
        };

        // Convert to angle (the full theta range sweeps `span`, 2π by default)
        let angle = self.start + self.direction * angle_norm * self.span;

        // Fit the swept sector's bounding box into the panel: a full circle
        // gets radius = half the smaller side, centred; a partial arc (gauge)
        // is scaled up and shifted so it fills the panel.
        let (bx0, bx1, by0, by1) = self.unit_bbox();
        let (bw, bh) = ((bx1 - bx0).max(1e-9), (by1 - by0).max(1e-9));
        let max_radius = (plot_area.width / bw).min(plot_area.height / bh);
        // Map [0,1] into [inner_radius, 1] so a donut leaves a centre hole.
        let radius = (self.inner_radius + radius_norm * (1.0 - self.inner_radius)) * max_radius;

        // Center of the polar plot
        let cx = plot_area.x + plot_area.width / 2.0 - max_radius * (bx0 + bx1) / 2.0;
        let cy = plot_area.y + plot_area.height / 2.0 - max_radius * (by0 + by1) / 2.0;

        // Convert polar to Cartesian pixel coordinates
        // angle=0 points up (12 o'clock), increases clockwise
        let px = cx + radius * angle.sin();
        let py = cy - radius * angle.cos();

        (px, py)
    }

    fn gridlines(&self) -> bool {
        false
    }

    fn is_flipped(&self) -> bool {
        false
    }

    fn is_polar(&self) -> bool {
        true
    }

    fn polar_theta_is_x(&self) -> bool {
        self.theta == "x"
    }
}
