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

/// Step direction: horizontal-then-vertical or vertical-then-horizontal.
pub enum StepDirection {
    /// Draw horizontal first, then vertical (default).
    Hv,
    /// Draw vertical first, then horizontal.
    Vh,
}

/// Step function line geometry.
pub struct GeomStep {
    pub color: (u8, u8, u8),
    pub width: f64,
    pub alpha: f64,
    pub direction: StepDirection,
}

impl Default for GeomStep {
    fn default() -> Self {
        GeomStep {
            color: (0, 0, 0),
            width: 1.5,
            alpha: 1.0,
            direction: StepDirection::Hv,
        }
    }
}

impl Geom for GeomStep {
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
        let color_col = data.column("color");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        // Sort by x
        let mut sorted: Vec<usize> = (0..data.nrows()).collect();
        sorted.sort_by(|&a, &b| {
            let xa = x_col[a].as_f64().unwrap_or(0.0);
            let xb = x_col[b].as_f64().unwrap_or(0.0);
            xa.partial_cmp(&xb).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Build raw normalized points
        let raw: Vec<(f64, f64)> = sorted
            .iter()
            .map(|&i| {
                let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
                let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
                (nx, ny)
            })
            .collect();

        // Insert step points
        let mut step_points: Vec<(f64, f64)> = Vec::new();
        for (j, &(nx, ny)) in raw.iter().enumerate() {
            if j > 0 {
                let (prev_nx, prev_ny) = raw[j - 1];
                match self.direction {
                    StepDirection::Hv => step_points.push((nx, prev_ny)),
                    StepDirection::Vh => step_points.push((prev_nx, ny)),
                }
            }
            step_points.push((nx, ny));
        }

        let points: Vec<(f64, f64)> = step_points
            .iter()
            .map(|&(nx, ny)| coord.transform((nx, ny), &plot_area))
            .collect();

        let line_color = color_col
            .and_then(|cc| {
                if cc.is_empty() {
                    None
                } else {
                    scales.map_color(&Aesthetic::Color, &cc[0])
                }
            })
            .unwrap_or(self.color);

        if points.len() >= 2 {
            backend.draw_line(
                &points,
                &LineStyle {
                    color: line_color,
                    alpha: self.alpha,
                    width: self.width,
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
        Box::new(StatIdentity)
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "step"
    }
}
