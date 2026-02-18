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
        let y_col = data
            .column("y")
            .ok_or(RenderError::MissingAesthetic("y".into()))?;
        let xmin_col = data
            .column("xmin")
            .ok_or(RenderError::MissingAesthetic("xmin".into()))?;
        let xmax_col = data
            .column("xmax")
            .ok_or(RenderError::MissingAesthetic("xmax".into()))?;
        let fill_col = data.column("fill");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        let fill_color = fill_col
            .and_then(|fc| {
                if fc.is_empty() {
                    None
                } else {
                    scales.map_color(&Aesthetic::Fill, &fc[0])
                }
            })
            .unwrap_or(self.fill);

        // Build right side (xmax, y) top to bottom
        let mut right_side: Vec<(f64, f64)> = Vec::new();
        // Build left side (xmin, y) bottom to top
        let mut left_side: Vec<(f64, f64)> = Vec::new();

        for i in 0..data.nrows() {
            let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
            let nxmax = x_scale.map(|s| s.map(&xmax_col[i])).unwrap_or(0.0);
            let nxmin = x_scale.map(|s| s.map(&xmin_col[i])).unwrap_or(0.0);

            right_side.push(coord.transform((nxmax, ny), &plot_area));
            left_side.push(coord.transform((nxmin, ny), &plot_area));
        }

        // Polygon: right side forward, left side reversed
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
}
