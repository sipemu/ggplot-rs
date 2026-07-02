pub mod cartesian;
pub mod fixed;
pub mod flip;
pub mod polar;
pub mod trans;

use crate::render::Rect;

/// A trained axis domain together with the normalized panel positions of its
/// endpoints. `pmin`/`pmax` are `scale.map(min)`/`scale.map(max)`, so a coord can
/// invert the (linear, expanded) scale mapping exactly: given a normalized
/// position `n`, the data value is `min + (n - pmin)/(pmax - pmin) * (max - min)`.
#[derive(Clone, Copy, Debug)]
pub struct AxisSpan {
    pub min: f64,
    pub max: f64,
    pub pmin: f64,
    pub pmax: f64,
}

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

    /// Supply the trained x/y axis spans after scale training. Coordinate systems
    /// that warp the axis (e.g. `coord_trans`) need this to map a normalized
    /// position back to a data value. Default is a no-op.
    fn set_domains(&mut self, _x: Option<AxisSpan>, _y: Option<AxisSpan>) {}
}
