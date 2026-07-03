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

/// Tessellate a bar — angle span `[t0, t1]`, radius span `[r0, r1]` in
/// normalized coords — into a radial-sector polygon of pixel points under a
/// polar `coord`. Both arcs are sampled so the fill follows the circle.
pub(crate) fn polar_sector(
    coord: &dyn Coord,
    area: &crate::render::Rect,
    t0: f64,
    t1: f64,
    r0: f64,
    r1: f64,
) -> Vec<(f64, f64)> {
    const N: usize = 24;
    let mut pts = Vec::with_capacity(2 * (N + 1));
    for k in 0..=N {
        let t = t0 + (t1 - t0) * k as f64 / N as f64;
        pts.push(coord.transform((t, r1), area));
    }
    for k in 0..=N {
        let t = t1 + (t0 - t1) * k as f64 / N as f64;
        pts.push(coord.transform((t, r0), area));
    }
    pts
}

/// Column geometry — like GeomBar but with pre-computed heights (uses StatIdentity).
pub struct GeomCol {
    pub fill: (u8, u8, u8),
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub width: f64,
}

impl Default for GeomCol {
    fn default() -> Self {
        GeomCol {
            fill: (97, 156, 255),
            color: (50, 50, 50),
            alpha: 1.0,
            width: 0.9,
        }
    }
}

impl Geom for GeomCol {
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

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);
        let x_is_discrete = x_scale.map(|s| s.is_discrete()).unwrap_or(false);

        for i in 0..data.nrows() {
            let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
            let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
            let ny_base = y_scale
                .map(|s| s.map(&crate::data::Value::Float(0.0)))
                .unwrap_or(0.0);

            let half_width = if x_is_discrete {
                let n_breaks = x_scale.map(|s| s.breaks().len()).unwrap_or(1);
                let bar_frac = self.width / (n_breaks.max(1) as f64 * 1.1);
                bar_frac / 2.0
            } else {
                0.02
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
                // A bar becomes a radial sector: tessellate the outer arc
                // (radius = value) and the inner arc (radius = base) so the
                // filled polygon follows the circle instead of a warped quad.
                let points = polar_sector(
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
        "col"
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }
}
