//! Ridgeline plot (`geom_density_ridges`, anofox-visualization#917): one
//! density per group of a discrete y, drawn upward from the group's
//! baseline and overlapping the next row — distributions over time.
use std::collections::HashMap;

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::xdensity_ridges::StatXDensityRidges;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Ridgeline geometry: a filled density per group of a discrete y.
pub struct GeomDensityRidges {
    pub fill: (u8, u8, u8),
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub line_width: f64,
    /// A ridge's peak height in rows (1.0: it just reaches the next
    /// baseline; more: ridges overlap). Give the y scale `scale - 0.5` rows
    /// of room above the last level
    /// ([`ScaleDiscrete::with_expand`](crate::scale::discrete::ScaleDiscrete::with_expand))
    /// or the top ridge is clipped.
    pub scale: f64,
}

impl Default for GeomDensityRidges {
    fn default() -> Self {
        GeomDensityRidges {
            fill: (97, 156, 255),
            color: (255, 255, 255),
            alpha: 0.85,
            line_width: 0.8,
            scale: 1.6,
        }
    }
}

impl Geom for GeomDensityRidges {
    fn key_fill(&self) -> Option<((u8, u8, u8), f64)> {
        Some((self.fill, self.alpha))
    }

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
        let h_col = data
            .column("height")
            .ok_or(RenderError::MissingAesthetic("height".into()))?;
        let fill_col = data.column("fill");
        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        // One row's height in the y scale's mapped space: the smallest gap
        // between two neighbouring levels.
        let mut pos: Vec<f64> = y_scale
            .map(|s| s.breaks().iter().map(|(p, _)| *p).collect())
            .unwrap_or_default();
        pos.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let row = pos
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .filter(|d| *d > 0.0)
            .fold(f64::INFINITY, f64::min);
        let row = if row.is_finite() { row } else { 0.5 };

        // The stat stacks one block of rows per group; collect them by y.
        let mut order: Vec<String> = Vec::new();
        let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, yv) in y_col.iter().enumerate() {
            let key = format!("{yv:?}");
            groups
                .entry(key.clone())
                .or_insert_with(|| {
                    order.push(key.clone());
                    Vec::new()
                })
                .push(i);
        }
        // Back to front: the highest baseline first, so a lower ridge is
        // drawn over the one above it.
        let base = |idxs: &Vec<usize>| y_scale.map(|s| s.map(&y_col[idxs[0]])).unwrap_or(0.0);
        order.sort_by(|a, b| {
            base(&groups[b])
                .partial_cmp(&base(&groups[a]))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for key in &order {
            let idxs = &groups[key];
            let first = idxs[0];
            let ny = base(idxs);
            let fill = fill_col
                .and_then(|fc| scales.map_color(&Aesthetic::Fill, &fc[first]))
                .unwrap_or(self.fill);
            let group = Some(super::tip_value(&y_col[first])).filter(|s| !s.is_empty());
            super::set_mark(
                backend,
                group.clone(),
                group,
                super::series_key(data, first),
                None,
            );
            let mut top: Vec<(f64, f64)> = Vec::with_capacity(idxs.len() + 2);
            let mut x0 = f64::NAN;
            let mut x1 = f64::NAN;
            for &i in idxs {
                let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
                let h = h_col[i].as_f64().unwrap_or(0.0).max(0.0);
                if x0.is_nan() {
                    x0 = nx;
                }
                x1 = nx;
                top.push(coord.transform((nx, ny + h * self.scale * row), &plot_area));
            }
            if top.len() < 2 {
                continue;
            }
            let mut poly = vec![coord.transform((x0, ny), &plot_area)];
            poly.extend(top);
            poly.push(coord.transform((x1, ny), &plot_area));
            backend.draw_polygon(
                &poly,
                &RectStyle {
                    fill: Some(fill),
                    stroke: Some(self.color),
                    stroke_width: self.line_width,
                    alpha: self.alpha,
                    clip: true,
                    corner_radius: 0.0,
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
        Box::new(StatXDensityRidges::default())
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "density_ridges"
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }
}
