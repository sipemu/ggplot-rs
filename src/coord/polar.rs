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
}

impl CoordPolar {
    pub fn new() -> Self {
        CoordPolar {
            theta: "x".to_string(),
            start: 0.0,
            direction: 1.0,
            inner_radius: 0.0,
        }
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

        // Convert to angle (full circle = 2π)
        let angle = self.start + self.direction * angle_norm * std::f64::consts::TAU;

        // Radius: fraction of the available radius (half the smaller dimension)
        let max_radius = plot_area.width.min(plot_area.height) / 2.0;
        // Map [0,1] into [inner_radius, 1] so a donut leaves a centre hole.
        let radius = (self.inner_radius + radius_norm * (1.0 - self.inner_radius)) * max_radius;

        // Center of the polar plot
        let cx = plot_area.x + plot_area.width / 2.0;
        let cy = plot_area.y + plot_area.height / 2.0;

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
