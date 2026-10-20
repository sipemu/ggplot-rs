use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::binhex::{HexLattice, StatBinHex};
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Hexagonal binning geometry — draws a hexagon per cell, in data units
/// (not a fixed pixel size): each vertex is computed in data space from the
/// row's `width`/`height` (as [`StatBinHex`] outputs them) and only then
/// mapped and projected, so the lattice stays correct under any coord
/// (including `coord_sf`).
pub struct GeomHex {
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub line_width: f64,
}

impl Default for GeomHex {
    fn default() -> Self {
        GeomHex {
            color: (50, 50, 50),
            alpha: 1.0,
            line_width: 0.2,
        }
    }
}

impl GeomHex {
    pub fn color(mut self, color: (u8, u8, u8)) -> Self {
        self.color = color;
        self
    }

    /// Sets `line_width` (the hex outline's stroke width).
    pub fn linewidth(mut self, width: f64) -> Self {
        self.line_width = width;
        self
    }
}

impl Geom for GeomHex {
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
        let width_col = data.column("width");
        let height_col = data.column("height");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        // Fall back to the data's own resolution (the smallest gap between
        // distinct values, like ggplot2's `resolution()`) when the stat
        // didn't supply `width`/`height` — e.g. a consumer feeding GeomHex
        // pre-binned cells through `StatIdentity`.
        let fallback_w =
            super::continuous_bar_half_width(x_col.iter().filter_map(|v| v.as_f64()), 2.0, 1.0);
        let fallback_h =
            super::continuous_bar_half_width(y_col.iter().filter_map(|v| v.as_f64()), 2.0, 1.0);

        for i in 0..data.nrows() {
            let (Some(cx), Some(cy)) = (x_col[i].as_f64(), y_col[i].as_f64()) else {
                continue;
            };
            let width = width_col
                .and_then(|wc| wc[i].as_f64())
                .filter(|w| w.is_finite() && *w > 0.0)
                .unwrap_or(fallback_w);
            let height = height_col
                .and_then(|hc| hc[i].as_f64())
                .filter(|h| h.is_finite() && *h > 0.0)
                .unwrap_or(fallback_h);

            let fill_color = fill_col
                .and_then(|fc| scales.map_color(&Aesthetic::Fill, &fc[i]))
                .unwrap_or((97, 156, 255));

            super::set_mark(
                backend,
                fill_col.map(|c| format!("n = {}", super::tip_value(&c[i]))),
                None,
                None,
                fill_col.and_then(|c| super::raw_value(&c[i])),
            );
            super::set_row_extras(backend, data, i);

            // Vertices in data space, each individually mapped + projected —
            // correct under any coord, including `coord_sf`.
            let verts_data = HexLattice::new(width, height, (cx, cy)).vertices(0, 0);
            let vertices: Vec<(f64, f64)> = verts_data
                .iter()
                .map(|&(vx, vy)| {
                    let nx = x_scale
                        .map(|s| s.map(&crate::data::Value::Float(vx)))
                        .unwrap_or(0.0);
                    let ny = y_scale
                        .map(|s| s.map(&crate::data::Value::Float(vy)))
                        .unwrap_or(0.0);
                    coord.transform((nx, ny), &plot_area)
                })
                .collect();

            backend.draw_polygon(
                &vertices,
                &RectStyle {
                    fill: Some(fill_color),
                    stroke: Some(self.color),
                    stroke_width: self.line_width,
                    alpha: self.alpha,
                    clip: true,
                },
            )?;
        }
        super::clear_mark(backend);
        super::clear_row_extras(backend);

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatBinHex::default())
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "hex"
    }
}
