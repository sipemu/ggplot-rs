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

/// Error bar geometry — vertical bars with whisker caps.
pub struct GeomErrorbar {
    pub color: (u8, u8, u8),
    pub width: f64,
    pub cap_width: f64,
    pub alpha: f64,
}

impl Default for GeomErrorbar {
    fn default() -> Self {
        GeomErrorbar {
            color: (0, 0, 0),
            width: 1.0,
            cap_width: 0.03,
            alpha: 1.0,
        }
    }
}

impl Geom for GeomErrorbar {
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

        let plot_area = backend.plot_area();
        let y_scale = scales.get(&Aesthetic::Y);

        let style = LineStyle {
            color: self.color,
            alpha: self.alpha,
            width: self.width,
            linetype: Linetype::Solid,
        };

        let xm = super::support::XMapper::new(data, scales);
        let color_col = data.column("color");
        for i in 0..data.nrows() {
            let nx = xm.map(&x_col[i], i);
            let ny_min = y_scale.map(|s| s.map(&ymin_col[i])).unwrap_or(0.0);
            let ny_max = y_scale.map(|s| s.map(&ymax_col[i])).unwrap_or(0.0);
            let color = color_col
                .and_then(|cc| scales.map_color(&Aesthetic::Color, &cc[i]))
                .unwrap_or(self.color);
            let style = LineStyle {
                color,
                ..style.clone()
            };

            // One connected polyline per bar — cap, bar, cap — transformed
            // point by point so it is correct under coord_flip too.
            let c = self.cap_width;
            let t = |p: (f64, f64)| coord.transform(p, &plot_area);
            super::support::mark_interval(backend, data, i, &x_col[i], &ymin_col[i], &ymax_col[i]);
            backend.draw_line(
                &[
                    t((nx - c, ny_max)),
                    t((nx + c, ny_max)),
                    t((nx, ny_max)),
                    t((nx, ny_min)),
                    t((nx - c, ny_min)),
                    t((nx + c, ny_min)),
                ],
                &style,
            )?;
        }
        super::clear_mark(backend);

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
        "errorbar"
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.color = color;
    }
}
