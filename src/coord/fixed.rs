use crate::render::Rect;

use super::Coord;

/// Fixed-ratio coordinate system — maintains aspect ratio.
pub struct CoordFixed {
    pub ratio: f64,
}

impl CoordFixed {
    pub fn new(ratio: f64) -> Self {
        CoordFixed { ratio }
    }
}

impl Coord for CoordFixed {
    fn transform(&self, point: (f64, f64), plot_area: &Rect) -> (f64, f64) {
        // Compute the effective area that maintains the aspect ratio
        let data_aspect = self.ratio; // data units_y / units_x
        let pixel_aspect = plot_area.height / plot_area.width;

        let (eff_w, eff_h, off_x, off_y) = if pixel_aspect > data_aspect {
            // Too tall — use full width, reduce height
            let h = plot_area.width * data_aspect;
            (plot_area.width, h, 0.0, (plot_area.height - h) / 2.0)
        } else {
            // Too wide — use full height, reduce width
            let w = plot_area.height / data_aspect;
            (w, plot_area.height, (plot_area.width - w) / 2.0, 0.0)
        };

        let (nx, ny) = point;
        let px = plot_area.x + off_x + nx * eff_w;
        let py = plot_area.y + off_y + (1.0 - ny) * eff_h;
        (px, py)
    }
}
