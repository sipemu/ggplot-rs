//! `coord_sf` — spatial coordinate system (feature `sf`).

use crate::render::Rect;

use super::{AxisSpan, Coord};

/// Equal-aspect coordinate system for maps: one map-unit in x covers the same
/// number of pixels as one map-unit in y, so projected geometry keeps its shape.
/// The aspect is derived from the trained data extent (via [`Coord::set_domains`]).
/// Pair it with a `geom_sf` projection (e.g. Mercator) for a conformal map.
#[derive(Default)]
pub struct CoordSf {
    x_span: Option<AxisSpan>,
    y_span: Option<AxisSpan>,
}

impl CoordSf {
    pub fn new() -> Self {
        Self::default()
    }

    /// Data y-range / x-range → the panel aspect that makes x and y isometric.
    fn ratio(&self) -> f64 {
        match (&self.x_span, &self.y_span) {
            (Some(x), Some(y)) if (x.max - x.min).abs() > f64::EPSILON => {
                ((y.max - y.min) / (x.max - x.min)).abs()
            }
            _ => 1.0,
        }
    }
}

impl Coord for CoordSf {
    fn set_domains(&mut self, x: Option<AxisSpan>, y: Option<AxisSpan>) {
        self.x_span = x;
        self.y_span = y;
    }

    fn transform(&self, point: (f64, f64), plot_area: &Rect) -> (f64, f64) {
        // Letterbox the panel to the data aspect ratio (like CoordFixed).
        let data_aspect = self.ratio();
        let pixel_aspect = plot_area.height / plot_area.width;
        let (eff_w, eff_h, off_x, off_y) = if pixel_aspect > data_aspect {
            let h = plot_area.width * data_aspect;
            (plot_area.width, h, 0.0, (plot_area.height - h) / 2.0)
        } else {
            let w = plot_area.height / data_aspect;
            (w, plot_area.height, (plot_area.width - w) / 2.0, 0.0)
        };
        let (nx, ny) = point;
        let px = plot_area.x + off_x + nx * eff_w;
        let py = plot_area.y + off_y + (1.0 - ny) * eff_h;
        (px, py)
    }
}
