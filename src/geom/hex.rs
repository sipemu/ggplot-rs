use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::binhex::StatBinHex;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Hexagonal binning geometry — draws regular hexagon polygons per cell.
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
    /// Generate 6 vertices of a regular hexagon centered at (cx, cy) with given radius.
    fn hex_vertices(cx: f64, cy: f64, rx: f64, ry: f64) -> Vec<(f64, f64)> {
        (0..6)
            .map(|k| {
                let angle = std::f64::consts::PI / 3.0 * k as f64 + std::f64::consts::PI / 6.0;
                (cx + rx * angle.cos(), cy + ry * angle.sin())
            })
            .collect()
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

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        // Determine hex size in pixel space from data density
        // Use a fixed fraction of the plot area
        let plot_w = plot_area.width;
        let plot_h = plot_area.height;
        let n_hex = (data.nrows() as f64).sqrt().max(1.0);
        let hex_rx = plot_w / (n_hex * 2.5);
        let hex_ry = plot_h / (n_hex * 2.5);

        for i in 0..data.nrows() {
            let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
            let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
            let (px, py) = coord.transform((nx, ny), &plot_area);

            let fill_color = fill_col
                .and_then(|fc| scales.map_color(&Aesthetic::Fill, &fc[i]))
                .unwrap_or((97, 156, 255));

            let vertices = Self::hex_vertices(px, py, hex_rx, hex_ry);

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
