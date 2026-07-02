use std::collections::HashMap;

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::ydensity::StatYDensity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Violin geometry — mirrored density polygon per group.
pub struct GeomViolin {
    pub fill: (u8, u8, u8),
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub line_width: f64,
}

impl Default for GeomViolin {
    fn default() -> Self {
        GeomViolin {
            fill: (97, 156, 255),
            color: (50, 50, 50),
            alpha: 0.7,
            line_width: 0.5,
        }
    }
}

impl Geom for GeomViolin {
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
        let width_col = data
            .column("violinwidth")
            .ok_or(RenderError::MissingAesthetic("violinwidth".into()))?;
        let fill_col = data.column("fill");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        // Half-width of a group's slot, in the X scale's mapped space. Mirror the
        // boxplot: discrete axes share the slot between all groups; continuous
        // axes use a small fixed fraction.
        let x_is_discrete = x_scale.map(|s| s.is_discrete()).unwrap_or(false);
        let max_half_width = if x_is_discrete {
            let n = x_scale.map(|s| s.breaks().len()).unwrap_or(1);
            0.75 / (n.max(1) as f64 * 2.5)
        } else {
            0.03
        };

        // The stat vstacks one contiguous block of points per group; partition
        // rows back into groups (keyed by the x value) and draw a polygon each.
        let mut order: Vec<String> = Vec::new();
        let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, xv) in x_col.iter().enumerate() {
            let key = format!("{xv:?}");
            if !groups.contains_key(&key) {
                order.push(key.clone());
                groups.insert(key.clone(), Vec::new());
            }
            groups.get_mut(&key).unwrap().push(i);
        }

        for key in &order {
            let idxs = &groups[key];
            let first = idxs[0];
            let nx = x_scale.map(|s| s.map(&x_col[first])).unwrap_or(0.5);
            let fill_color = fill_col
                .and_then(|fc| scales.map_color(&Aesthetic::Fill, &fc[first]))
                .unwrap_or(self.fill);

            // Right side (top→bottom) then left side reversed to close the polygon.
            let mut right_side: Vec<(f64, f64)> = Vec::with_capacity(idxs.len());
            let mut left_side: Vec<(f64, f64)> = Vec::with_capacity(idxs.len());
            for &i in idxs {
                let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
                let half = width_col[i].as_f64().unwrap_or(0.0) * max_half_width;
                right_side.push(coord.transform((nx + half, ny), &plot_area));
                left_side.push(coord.transform((nx - half, ny), &plot_area));
            }

            let mut polygon = right_side;
            left_side.reverse();
            polygon.extend(left_side);

            if polygon.len() >= 3 {
                backend.draw_polygon(
                    &polygon,
                    &RectStyle {
                        fill: Some(fill_color),
                        stroke: Some(self.color),
                        stroke_width: self.line_width,
                        alpha: self.alpha,
                        clip: true,
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
        Box::new(StatYDensity::default())
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "violin"
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }
}
