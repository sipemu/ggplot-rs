use crate::render::Rect;

use super::Coord;

/// Standard Cartesian coordinate system, with optional zoom limits.
///
/// Unlike `xlim()`/`ylim()` on scales, zoom limits do NOT filter data —
/// they only clip the viewport. Data outside the limits is still computed
/// by stats and positions, just not visible.
pub struct CoordCartesian {
    xlim: Option<(f64, f64)>,
    ylim: Option<(f64, f64)>,
}

impl CoordCartesian {
    pub fn new() -> Self {
        CoordCartesian {
            xlim: None,
            ylim: None,
        }
    }

    /// Set x-axis zoom limits (data coordinates).
    pub fn xlim(mut self, min: f64, max: f64) -> Self {
        self.xlim = Some((min, max));
        self
    }

    /// Set y-axis zoom limits (data coordinates).
    pub fn ylim(mut self, min: f64, max: f64) -> Self {
        self.ylim = Some((min, max));
        self
    }

    /// Get x zoom limits for use by the renderer.
    pub fn get_xlim(&self) -> Option<(f64, f64)> {
        self.xlim
    }

    /// Get y zoom limits for use by the renderer.
    pub fn get_ylim(&self) -> Option<(f64, f64)> {
        self.ylim
    }
}

impl Default for CoordCartesian {
    fn default() -> Self {
        Self::new()
    }
}

impl Coord for CoordCartesian {
    fn transform(&self, point: (f64, f64), plot_area: &Rect) -> (f64, f64) {
        let (nx, ny) = point;
        let px = plot_area.x + nx * plot_area.width;
        // Y is flipped: 0 at bottom, 1 at top
        let py = plot_area.y + (1.0 - ny) * plot_area.height;
        (px, py)
    }

    fn zoom_x(&self) -> Option<(f64, f64)> {
        self.xlim
    }

    fn zoom_y(&self) -> Option<(f64, f64)> {
        self.ylim
    }
}
