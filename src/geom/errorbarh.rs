//! Horizontal error bars (`geom_errorbarh`): a line from `(xmin, y)` to
//! `(xmax, y)` with vertical end caps — horizontal confidence intervals in a
//! coefficient forest or a time-to-event plot.

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

/// Horizontal error bar geometry (requires `xmin`, `xmax`, `y`).
pub struct GeomErrorbarh {
    pub color: (u8, u8, u8),
    /// Line width.
    pub width: f64,
    /// Half-height of the end caps, in normalized panel units (like
    /// `GeomErrorbar::cap_width`).
    pub cap_height: f64,
    pub alpha: f64,
}

impl Default for GeomErrorbarh {
    fn default() -> Self {
        GeomErrorbarh {
            color: (0, 0, 0),
            width: 1.0,
            cap_height: 0.02,
            alpha: 1.0,
        }
    }
}

impl Geom for GeomErrorbarh {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let y_col = data
            .column("y")
            .ok_or(RenderError::MissingAesthetic("y".into()))?;
        let xmin_col = data
            .column("xmin")
            .ok_or(RenderError::MissingAesthetic("xmin".into()))?;
        let xmax_col = data
            .column("xmax")
            .ok_or(RenderError::MissingAesthetic("xmax".into()))?;
        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        for i in 0..data.nrows() {
            let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
            let nx0 = x_scale.map(|s| s.map(&xmin_col[i])).unwrap_or(0.0);
            let nx1 = x_scale.map(|s| s.map(&xmax_col[i])).unwrap_or(0.0);
            if !(ny.is_finite() && nx0.is_finite() && nx1.is_finite()) {
                continue;
            }
            let color = data
                .column("color")
                .and_then(|c| scales.map_color(&Aesthetic::Color, &c[i]))
                .unwrap_or(self.color);
            let style = LineStyle {
                color,
                alpha: self.alpha,
                width: self.width,
                linetype: Linetype::Solid,
            };
            let series = super::series_key(data, i);
            let range = format!(
                "[{}, {}]",
                super::tip_value(&xmin_col[i]),
                super::tip_value(&xmax_col[i])
            );
            let tip = Some(match &series {
                Some(s) => format!("{s}: {}: {range}", super::tip_value(&y_col[i])),
                None => format!("{}: {range}", super::tip_value(&y_col[i])),
            });
            super::set_mark(
                backend,
                tip,
                Some(super::tip_value(&y_col[i])),
                series,
                super::raw_value(&xmin_col[i])
                    .zip(super::raw_value(&xmax_col[i]))
                    .map(|(a, b)| format!("{a} {b}")),
            );
            let h = self.cap_height;
            let t = |p: (f64, f64)| coord.transform(p, &plot_area);
            // One connected polyline per bar (cap, bar, cap) so each
            // interval is a single hoverable mark.
            backend.draw_line(
                &[
                    t((nx0, ny - h)),
                    t((nx0, ny + h)),
                    t((nx0, ny)),
                    t((nx1, ny)),
                    t((nx1, ny + h)),
                    t((nx1, ny - h)),
                ],
                &style,
            )?;
        }
        super::clear_mark(backend);
        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Y, Aesthetic::Xmin, Aesthetic::Xmax]
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
        "errorbarh"
    }
    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.color = color;
    }
}
