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

/// Path geometry — connects points in row order (no x-sort).
pub struct GeomPath {
    pub color: (u8, u8, u8),
    pub width: f64,
    pub alpha: f64,
}

impl Default for GeomPath {
    fn default() -> Self {
        GeomPath {
            color: (0, 0, 0),
            width: 1.5,
            alpha: 1.0,
        }
    }
}

impl Geom for GeomPath {
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
        let linetype_col = data.column("linetype");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        if let Some(cc) = color_col {
            let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
            for (i, v) in cc.iter().enumerate() {
                let key = v.to_group_key();
                if let Some(entry) = groups.iter_mut().find(|(k, _)| k == &key) {
                    entry.1.push(i);
                } else {
                    groups.push((key, vec![i]));
                }
            }

            for (_, indices) in &groups {
                let first_idx = indices[0];
                let line_color = scales
                    .map_color(&Aesthetic::Color, &cc[first_idx])
                    .unwrap_or(self.color);

                let lt = linetype_col
                    .and_then(|lc| scales.map_linetype(&lc[first_idx]))
                    .unwrap_or(Linetype::Solid);

                let points: Vec<(f64, f64)> = indices
                    .iter()
                    .map(|&i| {
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
                            linetype: lt,
                        },
                    )?;
                }
            }
        } else {
            let lt = linetype_col
                .and_then(|lc| {
                    if lc.is_empty() {
                        None
                    } else {
                        scales.map_linetype(&lc[0])
                    }
                })
                .unwrap_or(Linetype::Solid);

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
                        color: self.color,
                        alpha: self.alpha,
                        width: self.width,
                        linetype: lt,
                    },
                )?;
            }
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
        "path"
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.color = color;
    }
}
