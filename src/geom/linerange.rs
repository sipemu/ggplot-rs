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

/// Linerange geometry — vertical line from (x, ymin) to (x, ymax), no caps.
pub struct GeomLinerange {
    pub color: (u8, u8, u8),
    pub width: f64,
    pub alpha: f64,
}

impl Default for GeomLinerange {
    fn default() -> Self {
        GeomLinerange {
            color: (0, 0, 0),
            width: 1.0,
            alpha: 1.0,
        }
    }
}

impl Geom for GeomLinerange {
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
        let ymin_col = data
            .column("ymin")
            .ok_or(RenderError::MissingAesthetic("ymin".into()))?;
        let ymax_col = data
            .column("ymax")
            .ok_or(RenderError::MissingAesthetic("ymax".into()))?;
        let color_col = data.column("color");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        for i in 0..data.nrows() {
            let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
            let ny_min = y_scale.map(|s| s.map(&ymin_col[i])).unwrap_or(0.0);
            let ny_max = y_scale.map(|s| s.map(&ymax_col[i])).unwrap_or(0.0);

            let (cx, top) = coord.transform((nx, ny_max), &plot_area);
            let (_, bottom) = coord.transform((nx, ny_min), &plot_area);

            let line_color = color_col
                .and_then(|cc| scales.map_color(&Aesthetic::Color, &cc[i]))
                .unwrap_or(self.color);

            backend.draw_line(
                &[(cx, top), (cx, bottom)],
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
        vec![Aesthetic::X, Aesthetic::Ymin, Aesthetic::Ymax]
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
        "linerange"
    }
}
