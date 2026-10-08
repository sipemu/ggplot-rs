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
        // Group by `group`, else by a mapped colour / fill (ggplot2 groups by
        // discrete aesthetics), else one polygon.
        let group_col = data
            .column("group")
            .or_else(|| data.column("color"))
            .or_else(|| data.column("fill"));
        let fill_col = data.column("fill");
        let color_col = data.column("color");

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

            let stroke_color = color_col
                .and_then(|cc| scales.map_color(&Aesthetic::Color, &cc[first_idx]))
                .unwrap_or(self.color);

            let mut mapped: Vec<(f64, f64)> = indices
                .iter()
                .map(|&i| {
                    let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
                    let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
                    (nx, ny)
                })
                .collect();
            // Radar series go around the spokes in axis order.
            if coord.is_radar() {
                mapped.sort_by(|a, b| a.0.total_cmp(&b.0));
            }
            let points: Vec<(f64, f64)> = mapped
                .into_iter()
                .map(|p| coord.transform(p, &plot_area))
                .collect();
            let series = super::series_key(data, first_idx);
            super::set_mark(backend, series.clone(), None, series, None);

            backend.draw_polygon(
                &points,
                &RectStyle {
                    fill: Some(fill_color),
                    // No outline when line_width <= 0 (e.g. filled contour bands,
                    // where per-triangle strokes would show the triangulation).
                    stroke: (self.line_width > 0.0).then_some(stroke_color),
                    stroke_width: self.line_width,
                    alpha: self.alpha,
                    clip: true,
                },
            )?;
        }
        super::clear_mark(backend);

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
        "polygon"
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }
}
