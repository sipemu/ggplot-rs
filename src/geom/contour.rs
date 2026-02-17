use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineStyle, Linetype};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::contour::StatContour;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Contour geometry — draws contour lines from gridded data.
pub struct GeomContour {
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub width: f64,
    pub bins: usize,
    pub n_levels: usize,
}

impl Default for GeomContour {
    fn default() -> Self {
        GeomContour {
            color: (50, 50, 50),
            alpha: 1.0,
            width: 1.0,
            bins: 50,
            n_levels: 10,
        }
    }
}

impl Geom for GeomContour {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let x_col = match data.column("x") {
            Some(c) => c,
            None => return Ok(()),
        };
        let y_col = match data.column("y") {
            Some(c) => c,
            None => return Ok(()),
        };
        let group_col = data.column("group");
        let level_col = data.column("level");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        // Draw segment pairs grouped by group column
        // Each group is a line segment (2 points)
        let nrows = data.nrows();
        let mut i = 0;
        while i + 1 < nrows {
            // Check if these two points belong to the same group
            let same_group = match group_col {
                Some(gc) => gc[i].to_group_key() == gc[i + 1].to_group_key(),
                None => true,
            };

            if same_group {
                let nx0 = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
                let ny0 = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
                let nx1 = x_scale.map(|s| s.map(&x_col[i + 1])).unwrap_or(0.0);
                let ny1 = y_scale.map(|s| s.map(&y_col[i + 1])).unwrap_or(0.0);

                let p0 = coord.transform((nx0, ny0), &plot_area);
                let p1 = coord.transform((nx1, ny1), &plot_area);

                // Color by level if color scale is available
                let color = if let Some(lc) = level_col {
                    scales
                        .map_color(&Aesthetic::Color, &lc[i])
                        .unwrap_or(self.color)
                } else {
                    self.color
                };

                backend.draw_line(
                    &[p0, p1],
                    &LineStyle {
                        color,
                        alpha: self.alpha,
                        width: self.width,
                        linetype: Linetype::Solid,
                    },
                )?;

                i += 2; // Skip the pair
            } else {
                i += 1;
            }
        }

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatContour {
            bins: self.bins,
            n_levels: self.n_levels,
        })
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "contour"
    }
}
