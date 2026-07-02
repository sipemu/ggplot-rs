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

/// Spoke geometry — line segments from (x, y) with angle and radius.
pub struct GeomSpoke {
    pub color: (u8, u8, u8),
    pub width: f64,
    pub alpha: f64,
}

impl Default for GeomSpoke {
    fn default() -> Self {
        GeomSpoke {
            color: (0, 0, 0),
            width: 1.0,
            alpha: 1.0,
        }
    }
}

impl Geom for GeomSpoke {
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
        let angle_col = data
            .column("angle")
            .ok_or(RenderError::MissingAesthetic("angle".into()))?;
        let radius_col = data
            .column("radius")
            .ok_or(RenderError::MissingAesthetic("radius".into()))?;
        let color_col = data.column("color");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        for i in 0..data.nrows() {
            let x = x_col[i].as_f64().unwrap_or(0.0);
            let y = y_col[i].as_f64().unwrap_or(0.0);
            let angle = angle_col[i].as_f64().unwrap_or(0.0);
            let radius = radius_col[i].as_f64().unwrap_or(1.0);

            let xend = x + radius * angle.cos();
            let yend = y + radius * angle.sin();

            let nx1 = x_scale
                .map(|s| s.map(&crate::data::Value::Float(x)))
                .unwrap_or(0.0);
            let ny1 = y_scale
                .map(|s| s.map(&crate::data::Value::Float(y)))
                .unwrap_or(0.0);
            let nx2 = x_scale
                .map(|s| s.map(&crate::data::Value::Float(xend)))
                .unwrap_or(0.0);
            let ny2 = y_scale
                .map(|s| s.map(&crate::data::Value::Float(yend)))
                .unwrap_or(0.0);

            let (px1, py1) = coord.transform((nx1, ny1), &plot_area);
            let (px2, py2) = coord.transform((nx2, ny2), &plot_area);

            let line_color = color_col
                .and_then(|cc| scales.map_color(&Aesthetic::Color, &cc[i]))
                .unwrap_or(self.color);

            backend.draw_line(
                &[(px1, py1), (px2, py2)],
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
        vec![
            Aesthetic::X,
            Aesthetic::Y,
            Aesthetic::Angle,
            Aesthetic::Radius,
        ]
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
        "spoke"
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.color = color;
    }
}
