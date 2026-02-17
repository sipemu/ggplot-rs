use crate::render::Rect;

use super::Coord;

/// Flipped coordinate system — swaps X and Y axes.
pub struct CoordFlip;

impl Coord for CoordFlip {
    fn transform(&self, point: (f64, f64), plot_area: &Rect) -> (f64, f64) {
        let (nx, ny) = point;
        // Swap: x maps to vertical, y maps to horizontal
        let px = plot_area.x + ny * plot_area.width;
        let py = plot_area.y + (1.0 - nx) * plot_area.height;
        (px, py)
    }

    fn is_flipped(&self) -> bool {
        true
    }
}
