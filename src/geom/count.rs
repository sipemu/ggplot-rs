use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, PointShape, PointStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::sum::StatSum;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Count geometry — draws points sized by the number of overlapping observations.
pub struct GeomCount {
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub min_size: f64,
    pub max_size: f64,
}

impl Default for GeomCount {
    fn default() -> Self {
        GeomCount {
            color: (50, 50, 50),
            alpha: 0.7,
            min_size: 2.0,
            max_size: 12.0,
        }
    }
}

impl Geom for GeomCount {
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
        let n_col = data
            .column("n")
            .ok_or(RenderError::MissingAesthetic("n".into()))?;

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        // Compute n range for size scaling
        let n_values: Vec<f64> = n_col.iter().filter_map(|v| v.as_f64()).collect();
        let n_min = n_values.iter().cloned().fold(f64::INFINITY, f64::min);
        let n_max = n_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let n_range = n_max - n_min;

        for i in 0..data.nrows() {
            let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
            let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
            let (px, py) = coord.transform((nx, ny), &plot_area);

            let n = n_col[i].as_f64().unwrap_or(1.0);
            let size = if n_range > 0.0 {
                self.min_size + (n - n_min) / n_range * (self.max_size - self.min_size)
            } else {
                (self.min_size + self.max_size) / 2.0
            };

            let color = scales
                .map_color(&Aesthetic::Color, &x_col[i])
                .unwrap_or(self.color);

            backend.draw_shape(
                (px, py),
                size,
                &PointStyle {
                    color,
                    alpha: self.alpha,
                    filled: true,
                    shape: PointShape::Circle,
                },
            )?;
        }

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatSum)
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "count"
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.color = color;
    }
}
