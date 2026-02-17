pub mod cartesian;
pub mod fixed;
pub mod flip;
pub mod polar;

use crate::render::Rect;

/// Trait for coordinate systems.
pub trait Coord: Send + Sync {
    /// Transform normalized (0..1, 0..1) coordinates to pixel coordinates.
    fn transform(&self, point: (f64, f64), plot_area: &Rect) -> (f64, f64);

    /// Whether to draw grid lines.
    fn gridlines(&self) -> bool {
        true
    }

    /// Whether this coordinate system flips X and Y.
    fn is_flipped(&self) -> bool {
        false
    }

    /// Zoom limits for x-axis (data coordinates). Clips viewport without filtering data.
    fn zoom_x(&self) -> Option<(f64, f64)> {
        None
    }

    /// Zoom limits for y-axis (data coordinates). Clips viewport without filtering data.
    fn zoom_y(&self) -> Option<(f64, f64)> {
        None
    }
}
