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
    reverse_x: bool,
    reverse_y: bool,
    /// Keep the scales' expansion on each side: top, right, bottom, left
    /// (ggplot2's `coord_cartesian(expand =)`, #411).
    expand: [bool; 4],
}

impl CoordCartesian {
    pub fn new() -> Self {
        CoordCartesian {
            xlim: None,
            ylim: None,
            reverse_x: false,
            reverse_y: false,
            expand: [true; 4],
        }
    }

    /// ggplot2's `coord_cartesian(expand = c(top, right, bottom, left))`
    /// (#411): `false` on a side drops the continuous scale's padding there,
    /// so the data (or the zoom limit) meets that panel edge. Sides are as
    /// drawn, so they follow [`reverse`](Self::reverse). `expand(false,
    /// false, false, false)` is `expand = FALSE`.
    pub fn expand(mut self, top: bool, right: bool, bottom: bool, left: bool) -> Self {
        self.expand = [top, right, bottom, left];
        self
    }

    /// Reverse the x and / or y direction at draw time (ggplot2 4.0's
    /// `coord_cartesian(reverse = "x" | "y" | "xy")`) — the scales and stats
    /// are unchanged, the panel is mirrored.
    pub fn reverse(mut self, x: bool, y: bool) -> Self {
        self.reverse_x = x;
        self.reverse_y = y;
        self
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
        let nx = if self.reverse_x { 1.0 - nx } else { nx };
        let ny = if self.reverse_y { 1.0 - ny } else { ny };
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

    fn train_scales(&mut self, scales: &mut crate::scale::ScaleSet) {
        let [top, right, bottom, left] = self.expand.map(|e| !e);
        // (lower, upper) data ends per axis, mirrored when reversed.
        let x = if self.reverse_x {
            (right, left)
        } else {
            (left, right)
        };
        let y = if self.reverse_y {
            (top, bottom)
        } else {
            (bottom, top)
        };
        for (aes, (lo, hi)) in [(crate::aes::Aesthetic::X, x), (crate::aes::Aesthetic::Y, y)] {
            if lo || hi {
                if let Some(s) = scales.get_mut(&aes) {
                    s.clear_expand(lo, hi);
                }
            }
        }
    }
}
