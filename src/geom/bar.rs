use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::stack::PositionStack;
use crate::position::Position;
use crate::render::backend::{DrawBackend, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::count::StatCount;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Bar geometry for bar charts.
pub struct GeomBar {
    pub fill: (u8, u8, u8),
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub width: f64,
}

impl Default for GeomBar {
    fn default() -> Self {
        GeomBar {
            fill: (97, 156, 255),
            color: (50, 50, 50),
            alpha: 1.0,
            width: 0.9,
        }
    }
}

impl Geom for GeomBar {
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
        let ymin_col = data.column("ymin");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        let x_is_discrete = x_scale.map(|s| s.is_discrete()).unwrap_or(false);

        for i in 0..data.nrows() {
            let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
            let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);

            let ny_base = ymin_col
                .and_then(|c| c[i].as_f64())
                .and_then(|v| y_scale.map(|s| s.map(&crate::data::Value::Float(v))))
                .unwrap_or_else(|| {
                    y_scale
                        .map(|s| s.map(&crate::data::Value::Float(0.0)))
                        .unwrap_or(0.0)
                });

            // Bar width in normalized coords
            let half_width = if x_is_discrete {
                // Band-based: each category occupies 1/n of the axis
                let n_breaks = x_scale.map(|s| s.breaks().len()).unwrap_or(1);
                let band_width = 1.0 / n_breaks.max(1) as f64;
                band_width * self.width / 2.0
            } else {
                0.02 // Thin bars for continuous
            };

            let (fr, fg, fb) = if let Some(fc) = fill_col {
                scales
                    .map_color(&Aesthetic::Fill, &fc[i])
                    .unwrap_or(self.fill)
            } else {
                self.fill
            };
            let style = RectStyle {
                fill: Some((fr, fg, fb)),
                stroke: Some(self.color),
                stroke_width: 0.5,
                alpha: self.alpha,
                clip: !coord.is_polar(),
            };

            if coord.is_polar() {
                let points = super::col::polar_sector(
                    coord,
                    &plot_area,
                    nx - half_width,
                    nx + half_width,
                    ny_base,
                    ny,
                );
                backend.draw_polygon(&points, &style)?;
            } else {
                let (left_px, top_px) = coord.transform((nx - half_width, ny), &plot_area);
                let (right_px, bottom_px) = coord.transform((nx + half_width, ny_base), &plot_area);
                backend.draw_rect(
                    (left_px, top_px.min(bottom_px)),
                    (right_px, top_px.max(bottom_px)),
                    &style,
                )?;
            }
        }

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatCount)
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionStack)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "bar"
    }

    fn include_zero_baseline(&self) -> bool {
        true
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }
}
