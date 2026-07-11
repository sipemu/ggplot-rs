use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineStyle, Linetype, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Area geometry — filled polygon from line to x-axis baseline.
pub struct GeomArea {
    pub fill: (u8, u8, u8),
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub line_width: f64,
}

impl Default for GeomArea {
    fn default() -> Self {
        GeomArea {
            fill: (97, 156, 255),
            color: (50, 50, 50),
            alpha: 0.4,
            line_width: 1.0,
        }
    }
}

impl Geom for GeomArea {
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

        // A fill/color aesthetic (or an explicit group) splits the data into one
        // filled band per series — as in a grouped or stacked area chart.
        let group_col = data
            .column("fill")
            .or_else(|| data.column("color"))
            .or_else(|| data.column("group"));
        // `ymin` is set by position="stack"/"fill" (the bottom of each segment);
        // without it the band runs from the y=0 baseline.
        let ymin_col = data.column("ymin");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);
        let base_ny = y_scale.map(|s| s.map(&Value::Float(0.0))).unwrap_or(0.0);

        // Group row indices by the series key, preserving first-seen order (which
        // is the stacking order the position adjustment produced).
        let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
        match group_col {
            Some(gc) => {
                for (i, v) in gc.iter().enumerate() {
                    let key = v.to_group_key();
                    match groups.iter_mut().find(|(k, _)| k == &key) {
                        Some((_, idx)) => idx.push(i),
                        None => groups.push((key, vec![i])),
                    }
                }
            }
            None => groups.push((String::new(), (0..data.nrows()).collect())),
        }

        for (_, indices) in &groups {
            // Order this series left-to-right so the band is a clean polygon.
            let mut idx = indices.clone();
            idx.sort_by(|&a, &b| {
                let xa = x_scale.map(|s| s.map(&x_col[a])).unwrap_or(0.0);
                let xb = x_scale.map(|s| s.map(&x_col[b])).unwrap_or(0.0);
                xa.partial_cmp(&xb).unwrap_or(std::cmp::Ordering::Equal)
            });

            let fill = group_col
                .and_then(|gc| scales.map_color(&Aesthetic::Fill, &gc[idx[0]]))
                .or_else(|| {
                    group_col.and_then(|gc| scales.map_color(&Aesthetic::Color, &gc[idx[0]]))
                })
                .unwrap_or(self.fill);

            let mut upper: Vec<(f64, f64)> = Vec::with_capacity(idx.len());
            let mut lower: Vec<(f64, f64)> = Vec::with_capacity(idx.len());
            for &i in &idx {
                let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
                let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
                let nb = ymin_col
                    .and_then(|c| c[i].as_f64())
                    .and_then(|v| y_scale.map(|s| s.map(&Value::Float(v))))
                    .unwrap_or(base_ny);
                upper.push(coord.transform((nx, ny), &plot_area));
                lower.push(coord.transform((nx, nb), &plot_area));
            }

            let mut polygon = upper.clone();
            lower.reverse();
            polygon.extend(lower);
            if polygon.len() >= 3 {
                backend.draw_polygon(
                    &polygon,
                    &RectStyle {
                        fill: Some(fill),
                        stroke: None,
                        stroke_width: 0.0,
                        alpha: self.alpha,
                        clip: true,
                    },
                )?;
            }
            if upper.len() >= 2 {
                // A single-series area keeps its dark outline; multi-series bands
                // outline in their own fill so stacked bands stay legible.
                let line_color = if groups.len() > 1 { fill } else { self.color };
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
        "area"
    }

    fn include_zero_baseline(&self) -> bool {
        true
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }
}
