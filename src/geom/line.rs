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

/// Line geometry — connects points sorted by x.
pub struct GeomLine {
    pub color: (u8, u8, u8),
    pub width: f64,
    pub alpha: f64,
}

impl Default for GeomLine {
    fn default() -> Self {
        GeomLine {
            color: (0, 0, 0),
            width: 1.5,
            alpha: 1.0,
        }
    }
}

impl Geom for GeomLine {
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

        // If there's a color/group aesthetic, draw separate lines per group
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

                // Sort indices by x value
                let mut sorted = indices.clone();
                sorted.sort_by(|&a, &b| {
                    let xa = x_col[a].as_f64().unwrap_or(0.0);
                    let xb = x_col[b].as_f64().unwrap_or(0.0);
                    xa.partial_cmp(&xb).unwrap_or(std::cmp::Ordering::Equal)
                });

                let points: Vec<(f64, f64)> = sorted
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

            // Sort by x value
            let mut sorted_indices: Vec<usize> = (0..data.nrows()).collect();
            sorted_indices.sort_by(|&a, &b| {
                let xa = x_col[a].as_f64().unwrap_or(0.0);
                let xb = x_col[b].as_f64().unwrap_or(0.0);
                xa.partial_cmp(&xb).unwrap_or(std::cmp::Ordering::Equal)
            });

            let points: Vec<(f64, f64)> = sorted_indices
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
        "line"
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.color = color;
    }
}
