use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Tile geometry — rectangle centered at (x, y) with given width/height.
pub struct GeomTile {
    pub fill: (u8, u8, u8),
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub width: f64,
    pub height: f64,
    pub line_width: f64,
}

impl Default for GeomTile {
    fn default() -> Self {
        GeomTile {
            fill: (97, 156, 255),
            color: (50, 50, 50),
            alpha: 1.0,
            width: 1.0,
            height: 1.0,
            line_width: 0.5,
        }
    }
}

impl Geom for GeomTile {
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
        let fill_col = data.column("fill");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        let half_w = self.width / 2.0;
        let half_h = self.height / 2.0;

        for i in 0..data.nrows() {
            let cx = x_col[i].as_f64().unwrap_or(0.0);
            let cy = y_col[i].as_f64().unwrap_or(0.0);

            let nxmin = x_scale
                .map(|s| s.map(&crate::data::Value::Float(cx - half_w)))
                .unwrap_or(0.0);
            let nxmax = x_scale
                .map(|s| s.map(&crate::data::Value::Float(cx + half_w)))
                .unwrap_or(0.0);
            let nymin = y_scale
                .map(|s| s.map(&crate::data::Value::Float(cy - half_h)))
                .unwrap_or(0.0);
            let nymax = y_scale
                .map(|s| s.map(&crate::data::Value::Float(cy + half_h)))
                .unwrap_or(0.0);

            let (left, top) = coord.transform((nxmin, nymax), &plot_area);
            let (right, bottom) = coord.transform((nxmax, nymin), &plot_area);

            let fill_color = fill_col
                .and_then(|fc| scales.map_color(&Aesthetic::Fill, &fc[i]))
                .unwrap_or(self.fill);

            backend.draw_rect(
                (left, top.min(bottom)),
                (right, top.max(bottom)),
                &RectStyle {
                    fill: Some(fill_color),
                    stroke: Some(self.color),
                    stroke_width: self.line_width,
                    alpha: self.alpha,
                    clip: true,
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
        "tile"
    }
}
