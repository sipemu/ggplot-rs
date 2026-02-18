use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineStyle, Linetype, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::density::StatDensity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Density curve geometry — filled area under a kernel density estimate.
pub struct GeomDensity {
    pub fill: (u8, u8, u8),
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub line_width: f64,
}

impl Default for GeomDensity {
    fn default() -> Self {
        GeomDensity {
            fill: (97, 156, 255),
            color: (50, 50, 50),
            alpha: 0.3,
            line_width: 1.0,
        }
    }
}

impl Geom for GeomDensity {
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
        let color_col = data.column("color");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        let base_ny = y_scale.map(|s| s.map(&Value::Float(0.0))).unwrap_or(0.0);

        // Determine groups from color or fill column
        let group_col = color_col.or(fill_col);
        let groups: Vec<(String, Vec<usize>)> = if let Some(gc) = group_col {
            let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
            for (i, v) in gc.iter().enumerate() {
                let key = v.to_group_key();
                if let Some(entry) = groups.iter_mut().find(|(k, _)| k == &key) {
                    entry.1.push(i);
                } else {
                    groups.push((key, vec![i]));
                }
            }
            groups
        } else {
            // Single group with all indices
            vec![("".to_string(), (0..data.nrows()).collect())]
        };

        for (_, indices) in &groups {
            if indices.is_empty() {
                continue;
            }
            let first_idx = indices[0];

            // Resolve fill: use fill scale if fill column exists, else use color scale, else default
            let fill_color = fill_col
                .and_then(|fc| scales.map_color(&Aesthetic::Fill, &fc[first_idx]))
                .or_else(|| {
                    color_col.and_then(|cc| scales.map_color(&Aesthetic::Color, &cc[first_idx]))
                })
                .unwrap_or(self.fill);

            // Resolve line: use color scale if color column exists, else use fill scale, else default
            let line_color = color_col
                .and_then(|cc| scales.map_color(&Aesthetic::Color, &cc[first_idx]))
                .or_else(|| {
                    fill_col.and_then(|fc| scales.map_color(&Aesthetic::Fill, &fc[first_idx]))
                })
                .unwrap_or(self.color);

            let mut upper: Vec<(f64, f64)> = Vec::new();
            let mut lower: Vec<(f64, f64)> = Vec::new();

            for &i in indices {
                let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
                let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
                upper.push(coord.transform((nx, ny), &plot_area));
                lower.push(coord.transform((nx, base_ny), &plot_area));
            }

            // Filled polygon
            let mut polygon = upper.clone();
            lower.reverse();
            polygon.extend(lower);

            if polygon.len() >= 3 {
                backend.draw_polygon(
                    &polygon,
                    &RectStyle {
                        fill: Some(fill_color),
                        stroke: None,
                        stroke_width: 0.0,
                        alpha: self.alpha,
                        clip: true,
                    },
                )?;
            }

            // Top line
            if upper.len() >= 2 {
                backend.draw_line(
                    &upper,
                    &LineStyle {
                        color: line_color,
                        alpha: 1.0,
                        width: self.line_width,
                        linetype: Linetype::Solid,
                    },
                )?;
            }
        }

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatDensity::default())
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "density"
    }
}
