use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineStyle, Linetype};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::bin::StatBin;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Frequency polygon — line through bin centers (StatBin output).
pub struct GeomFreqpoly {
    pub color: (u8, u8, u8),
    pub width: f64,
    pub alpha: f64,
}

impl Default for GeomFreqpoly {
    fn default() -> Self {
        GeomFreqpoly {
            color: (0, 0, 0),
            width: 1.5,
            alpha: 1.0,
        }
    }
}

impl Geom for GeomFreqpoly {
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

        let line_color = color_col
            .and_then(|cc| {
                if cc.is_empty() {
                    None
                } else {
                    scales.map_color(&Aesthetic::Color, &cc[0])
                }
            })
            .unwrap_or(self.color);

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
        vec![Aesthetic::X]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatBin::default())
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "freqpoly"
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.color = color;
    }
}
