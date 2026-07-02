use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineStyle, Linetype};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Curve geometry — curved segment via quadratic Bezier approximation.
pub struct GeomCurve {
    pub color: (u8, u8, u8),
    pub width: f64,
    pub alpha: f64,
    pub curvature: f64,
    pub ncp: usize,
}

impl Default for GeomCurve {
    fn default() -> Self {
        GeomCurve {
            color: (0, 0, 0),
            width: 1.0,
            alpha: 1.0,
            curvature: 0.5,
            ncp: 5,
        }
    }
}

impl GeomCurve {
    /// Generate points along a quadratic Bezier from p0 to p2 with control point p1.
    fn bezier_points(
        p0: (f64, f64),
        p2: (f64, f64),
        curvature: f64,
        ncp: usize,
    ) -> Vec<(f64, f64)> {
        // Control point perpendicular to midpoint
        let mx = (p0.0 + p2.0) / 2.0;
        let my = (p0.1 + p2.1) / 2.0;
        let dx = p2.0 - p0.0;
        let dy = p2.1 - p0.1;
        // Perpendicular direction
        let p1x = mx - dy * curvature;
        let p1y = my + dx * curvature;

        let n = ncp + 2; // include start and end
        (0..n)
            .map(|i| {
                let t = i as f64 / (n - 1) as f64;
                let u = 1.0 - t;
                let x = u * u * p0.0 + 2.0 * u * t * p1x + t * t * p2.0;
                let y = u * u * p0.1 + 2.0 * u * t * p1y + t * t * p2.1;
                (x, y)
            })
            .collect()
    }
}

impl Geom for GeomCurve {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let x_col = data
            .column("x")
            .ok_or(RenderError::MissingAesthetic("x".into()))?;
        let y_col = data
            .column("y")
            .ok_or(RenderError::MissingAesthetic("y".into()))?;
        let xend_col = data
            .column("xend")
            .ok_or(RenderError::MissingAesthetic("xend".into()))?;
        let yend_col = data
            .column("yend")
            .ok_or(RenderError::MissingAesthetic("yend".into()))?;
        let color_col = data.column("color");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        for i in 0..data.nrows() {
            let nx1 = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
            let ny1 = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
            let nx2 = x_scale.map(|s| s.map(&xend_col[i])).unwrap_or(0.0);
            let ny2 = y_scale.map(|s| s.map(&yend_col[i])).unwrap_or(0.0);

            let p0 = coord.transform((nx1, ny1), &plot_area);
            let p2 = coord.transform((nx2, ny2), &plot_area);

            let curve_points = Self::bezier_points(p0, p2, self.curvature, self.ncp);

            let line_color = color_col
                .and_then(|cc| scales.map_color(&Aesthetic::Color, &cc[i]))
                .unwrap_or(self.color);

            if curve_points.len() >= 2 {
                backend.draw_line(
                    &curve_points,
                    &LineStyle {
                        color: line_color,
                        alpha: self.alpha,
                        width: self.width,
                        linetype: Linetype::Solid,
                    },
                )?;
            }
        }

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y, Aesthetic::Xend, Aesthetic::Yend]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatIdentity)
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "curve"
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.color = color;
    }
}
