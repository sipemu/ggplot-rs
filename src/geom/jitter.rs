use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::jitter::PositionJitter;
use crate::position::Position;
use crate::render::backend::{DrawBackend, PointShape, PointStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Jittered scatter plot — like GeomPoint but with PositionJitter by default.
pub struct GeomJitter {
    pub size: f64,
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub width: f64,
    pub height: f64,
}

impl Default for GeomJitter {
    fn default() -> Self {
        GeomJitter {
            size: 3.0,
            color: (0, 0, 0),
            alpha: 1.0,
            width: 0.4,
            height: 0.4,
        }
    }
}

impl Geom for GeomJitter {
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
        let size_col = data.column("size");
        let alpha_col = data.column("alpha");
        let shape_col = data.column("shape");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        for i in 0..data.nrows() {
            let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
            let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
            let (px, py) = coord.transform((nx, ny), &plot_area);

            let (r, g, b) = if let Some(cc) = color_col {
                scales
                    .map_color(&Aesthetic::Color, &cc[i])
                    .unwrap_or(self.color)
            } else {
                self.color
            };

            let alpha = alpha_col.and_then(|c| c[i].as_f64()).unwrap_or(self.alpha);
            let size = size_col.and_then(|c| c[i].as_f64()).unwrap_or(self.size);
            let shape = shape_col
                .and_then(|c| scales.map_shape(&c[i]))
                .unwrap_or(PointShape::Circle);

            backend.draw_shape(
                (px, py),
                size,
                &PointStyle {
                    color: (r, g, b),
                    alpha,
                    filled: true,
                    shape,
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
        Box::new(PositionJitter {
            width: self.width,
            height: self.height,
        })
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "jitter"
    }
}
