use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::bin2d::StatBin2d;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// 2D binning geometry — filled rectangles with fill color from count.
pub struct GeomBin2d {
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub line_width: f64,
}

impl Default for GeomBin2d {
    fn default() -> Self {
        GeomBin2d {
            color: (50, 50, 50),
            alpha: 1.0,
            line_width: 0.2,
        }
    }
}

impl Geom for GeomBin2d {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let xmin_col = data
            .column("xmin")
            .ok_or(RenderError::MissingAesthetic("xmin".into()))?;
        let xmax_col = data
            .column("xmax")
            .ok_or(RenderError::MissingAesthetic("xmax".into()))?;
        let ymin_col = data
            .column("ymin")
            .ok_or(RenderError::MissingAesthetic("ymin".into()))?;
        let ymax_col = data
            .column("ymax")
            .ok_or(RenderError::MissingAesthetic("ymax".into()))?;
        let fill_col = data.column("fill");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        for i in 0..data.nrows() {
            let nxmin = x_scale.map(|s| s.map(&xmin_col[i])).unwrap_or(0.0);
            let nxmax = x_scale.map(|s| s.map(&xmax_col[i])).unwrap_or(0.0);
            let nymin = y_scale.map(|s| s.map(&ymin_col[i])).unwrap_or(0.0);
            let nymax = y_scale.map(|s| s.map(&ymax_col[i])).unwrap_or(0.0);

            let (left, top) = coord.transform((nxmin, nymax), &plot_area);
            let (right, bottom) = coord.transform((nxmax, nymin), &plot_area);

            let fill_color = fill_col
                .and_then(|fc| scales.map_color(&Aesthetic::Fill, &fc[i]))
                .unwrap_or((97, 156, 255));

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
        Box::new(StatBin2d::default())
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "bin2d"
    }
}
