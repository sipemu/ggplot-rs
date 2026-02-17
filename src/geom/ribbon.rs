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

/// Ribbon geometry — filled band between ymin and ymax.
pub struct GeomRibbon {
    pub fill: (u8, u8, u8),
    pub alpha: f64,
}

impl Default for GeomRibbon {
    fn default() -> Self {
        GeomRibbon {
            fill: (97, 156, 255),
            alpha: 0.3,
        }
    }
}

impl Geom for GeomRibbon {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let x_col = data.column("x").ok_or(RenderError::MissingAesthetic("x".into()))?;
        let ymin_col = data.column("ymin").ok_or(RenderError::MissingAesthetic("ymin".into()))?;
        let ymax_col = data.column("ymax").ok_or(RenderError::MissingAesthetic("ymax".into()))?;

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        let mut upper: Vec<(f64, f64)> = Vec::new();
        let mut lower: Vec<(f64, f64)> = Vec::new();

        for i in 0..data.nrows() {
            let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
            let ny_max = y_scale.map(|s| s.map(&ymax_col[i])).unwrap_or(0.0);
            let ny_min = y_scale.map(|s| s.map(&ymin_col[i])).unwrap_or(0.0);
            upper.push(coord.transform((nx, ny_max), &plot_area));
            lower.push(coord.transform((nx, ny_min), &plot_area));
        }

        let mut polygon = upper;
        lower.reverse();
        polygon.extend(lower);

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

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Ymin, Aesthetic::Ymax]
    }

    fn default_stat(&self) -> Box<dyn Stat> { Box::new(StatIdentity) }
    fn default_position(&self) -> Box<dyn Position> { Box::new(PositionIdentity) }
    fn default_params(&self) -> GeomParams { GeomParams::default() }
    fn name(&self) -> &str { "ribbon" }
}
