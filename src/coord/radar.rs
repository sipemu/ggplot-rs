//! Radar (spider) coordinates: polar with *straight* segments between spokes.

use crate::aes::Aesthetic;
use crate::data::Value;
use crate::render::Rect;
use crate::scale::ScaleSet;

use super::Coord;

/// Radar / spider-chart coordinates (`coord_radar()`).
///
/// `x` (usually discrete — one spoke per level) maps to angle, with the first
/// level at 12 o'clock and levels running clockwise; `y` maps to the distance
/// from the centre, which is the y scale's value `0` (the y scale is trained
/// to include 0). Geoms connect points with straight segments: `geom_polygon`
/// draws closed, filled series polygons and `geom_line` closes its path.
/// Gridlines become rings (polygons through the spokes at the y breaks) and
/// spokes, with the x levels labelled around the outside.
pub struct CoordRadar {
    /// Angle of the first spoke (radians, 0 = 12 o'clock, clockwise).
    pub start: f64,
    /// Outer radius as a fraction of half the panel's smaller side; the rest
    /// is left for the spoke labels.
    pub radius_frac: f64,
    /// Number of discrete x levels (spokes), learned from the x scale.
    n_spokes: Option<usize>,
    /// Normalized y position of the centre (the y scale's `0`).
    r0: f64,
}

impl CoordRadar {
    pub fn new() -> Self {
        CoordRadar {
            start: 0.0,
            radius_frac: 0.78,
            n_spokes: None,
            r0: 0.0,
        }
    }

    /// Rotate the first spoke to `start` radians (0 = 12 o'clock).
    pub fn start(mut self, start: f64) -> Self {
        self.start = start;
        self
    }

    /// Outer radius as a fraction of half the panel's smaller side.
    pub fn radius_frac(mut self, frac: f64) -> Self {
        self.radius_frac = frac.clamp(0.1, 1.0);
        self
    }

    /// Spoke positions: the discrete x level centres sit at `(i + 0.5) / n`;
    /// shift so level 0 points at `start`.
    fn angle(&self, nx: f64) -> f64 {
        let offset = match self.n_spokes {
            Some(n) if n > 0 => -std::f64::consts::PI / n as f64,
            _ => 0.0,
        };
        self.start + offset + nx * std::f64::consts::TAU
    }
}

impl Default for CoordRadar {
    fn default() -> Self {
        Self::new()
    }
}

impl Coord for CoordRadar {
    fn transform(&self, (nx, ny): (f64, f64), plot_area: &Rect) -> (f64, f64) {
        let max_r = plot_area.width.min(plot_area.height) / 2.0 * self.radius_frac;
        let span = (1.0 - self.r0).max(1e-9);
        let frac = ((ny - self.r0) / span).max(0.0);
        let r = if frac.is_finite() { frac * max_r } else { 0.0 };
        let a = self.angle(nx);
        let cx = plot_area.x + plot_area.width / 2.0;
        let cy = plot_area.y + plot_area.height / 2.0;
        (cx + r * a.sin(), cy - r * a.cos())
    }

    fn gridlines(&self) -> bool {
        false
    }

    fn is_polar(&self) -> bool {
        true
    }

    fn is_radar(&self) -> bool {
        true
    }

    fn train_scales(&mut self, scales: &mut ScaleSet) {
        if let Some(y) = scales.get_mut(&Aesthetic::Y) {
            if !y.is_discrete() {
                y.train(&[Value::Float(0.0)]);
                self.r0 = y.map(&Value::Float(0.0));
            }
        }
        self.n_spokes = scales
            .get(&Aesthetic::X)
            .filter(|s| s.is_discrete())
            .map(|s| s.breaks().len());
    }
}
