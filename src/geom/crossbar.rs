use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineStyle, Linetype, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Crossbar geometry — rectangle from ymin to ymax with horizontal line at y.
pub struct GeomCrossbar {
    pub fill: (u8, u8, u8),
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub bar_width: f64,
    pub line_width: f64,
}

impl Default for GeomCrossbar {
    fn default() -> Self {
        GeomCrossbar {
            fill: (255, 255, 255),
            color: (0, 0, 0),
            alpha: 1.0,
            bar_width: 0.5,
            line_width: 1.0,
        }
    }
}

impl Geom for GeomCrossbar {
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

        let half_w = self.bar_width / 2.0;

        for i in 0..data.nrows() {
            let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
            let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
            let ny_min = y_scale.map(|s| s.map(&ymin_col[i])).unwrap_or(0.0);
            let ny_max = y_scale.map(|s| s.map(&ymax_col[i])).unwrap_or(0.0);

            let (left, top) = coord.transform((nx - half_w, ny_max), &plot_area);
            let (right, bottom) = coord.transform((nx + half_w, ny_min), &plot_area);

            let fill_color = fill_col
                .and_then(|fc| scales.map_color(&Aesthetic::Fill, &fc[i]))
                .unwrap_or(self.fill);

            // Rectangle
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

            // Horizontal line at y (median)
            let (l_px, mid_py) = coord.transform((nx - half_w, ny), &plot_area);
            let (r_px, _) = coord.transform((nx + half_w, ny), &plot_area);
            backend.draw_line(
                &[(l_px, mid_py), (r_px, mid_py)],
                &LineStyle {
                    color: self.color,
                    alpha: self.alpha,
                    width: self.line_width * 1.5,
                    linetype: Linetype::Solid,
                },
            )?;
        }

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y, Aesthetic::Ymin, Aesthetic::Ymax]
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
        "crossbar"
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }
}
