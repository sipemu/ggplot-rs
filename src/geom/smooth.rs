use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineStyle, Linetype, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::smooth::{SmoothMethod, StatSmooth};
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Smooth line with optional confidence ribbon.
pub struct GeomSmooth {
    pub color: (u8, u8, u8),
    pub fill: (u8, u8, u8),
    pub line_width: f64,
    pub alpha: f64,
    pub se: bool,
    pub n_points: usize,
    pub method: SmoothMethod,
}

impl Default for GeomSmooth {
    fn default() -> Self {
        GeomSmooth {
            color: (51, 102, 204),
            fill: (51, 102, 204),
            line_width: 1.5,
            alpha: 0.2,
            se: true,
            n_points: 80,
            method: SmoothMethod::Lm,
        }
    }
}

impl GeomSmooth {
    /// Use LOESS smoothing with the given span.
    pub fn loess(mut self, span: f64) -> Self {
        self.method = SmoothMethod::Loess { span };
        self
    }
}

impl Geom for GeomSmooth {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let x_col = data.column("x").ok_or(RenderError::MissingAesthetic("x".into()))?;
        let y_col = data.column("y").ok_or(RenderError::MissingAesthetic("y".into()))?;
        let ymin_col = data.column("ymin");
        let ymax_col = data.column("ymax");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        // Draw confidence ribbon first (behind line)
        if self.se {
            if let (Some(ymin), Some(ymax)) = (ymin_col, ymax_col) {
                let mut upper_points: Vec<(f64, f64)> = Vec::new();
                let mut lower_points: Vec<(f64, f64)> = Vec::new();

                for i in 0..data.nrows() {
                    let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
                    let ny_max = y_scale.map(|s| s.map(&ymax[i])).unwrap_or(0.0);
                    let ny_min = y_scale.map(|s| s.map(&ymin[i])).unwrap_or(0.0);

                    upper_points.push(coord.transform((nx, ny_max), &plot_area));
                    lower_points.push(coord.transform((nx, ny_min), &plot_area));
                }

                // Build polygon: upper left-to-right, then lower right-to-left
                let mut polygon = upper_points.clone();
                lower_points.reverse();
                polygon.extend(lower_points);

                if polygon.len() >= 3 {
                    backend.draw_polygon(
                        &polygon,
                        &RectStyle {
                            fill: Some(self.fill),
                            stroke: None,
                            stroke_width: 0.0,
                            alpha: self.alpha,
                        },
                    )?;
                }
            }
        }

        // Draw fitted line
        let points: Vec<(f64, f64)> = (0..data.nrows())
            .map(|i| {
                let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
                let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
                coord.transform((nx, ny), &plot_area)
            })
            .collect();

        if points.len() >= 2 {
            backend.draw_line(
                &points,
                &LineStyle {
                    color: self.color,
                    alpha: 1.0,
                    width: self.line_width,
                    linetype: Linetype::Solid,
                },
            )?;
        }

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatSmooth {
            n_points: self.n_points,
            se: self.se,
            method: self.method.clone(),
        })
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "smooth"
    }
}
