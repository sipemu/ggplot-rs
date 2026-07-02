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

/// Polygon geometry — arbitrary filled polygon from (x, y) grouped by group column.
pub struct GeomPolygon {
    pub fill: (u8, u8, u8),
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub line_width: f64,
}

impl Default for GeomPolygon {
    fn default() -> Self {
        GeomPolygon {
            fill: (97, 156, 255),
            color: (50, 50, 50),
            alpha: 0.5,
            line_width: 0.5,
        }
    }
}

impl Geom for GeomPolygon {
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
        let group_col = data.column("group");
        let fill_col = data.column("fill");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        // Group indices
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
            vec![("".to_string(), (0..data.nrows()).collect())]
        };

        for (_, indices) in &groups {
            if indices.len() < 3 {
                continue;
            }
            let first_idx = indices[0];

            let fill_color = fill_col
                .and_then(|fc| scales.map_color(&Aesthetic::Fill, &fc[first_idx]))
                .unwrap_or(self.fill);

            let points: Vec<(f64, f64)> = indices
                .iter()
                .map(|&i| {
                    let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
                    let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
                    coord.transform((nx, ny), &plot_area)
                })
                .collect();

            backend.draw_polygon(
                &points,
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
        vec![Aesthetic::X, Aesthetic::Y, Aesthetic::Group]
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
        "polygon"
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }
}
