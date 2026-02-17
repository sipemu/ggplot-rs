use crate::render::Rect;

use super::Coord;

/// Standard Cartesian coordinate system.
pub struct CoordCartesian;

impl Coord for CoordCartesian {
    fn transform(&self, point: (f64, f64), plot_area: &Rect) -> (f64, f64) {
        let (nx, ny) = point;
        let px = plot_area.x + nx * plot_area.width;
        // Y is flipped: 0 at bottom, 1 at top
        let py = plot_area.y + (1.0 - ny) * plot_area.height;
        (px, py)
    }
}
