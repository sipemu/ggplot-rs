use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineStyle, Linetype};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Step direction: horizontal-then-vertical or vertical-then-horizontal.
pub enum StepDirection {
    /// Draw horizontal first, then vertical (default).
    Hv,
    /// Draw vertical first, then horizontal.
    Vh,
    /// Step half-way between adjacent x values (ggplot2's `"mid"`).
    Mid,
}

/// Step function line geometry.
pub struct GeomStep {
    pub color: (u8, u8, u8),
    pub width: f64,
    pub alpha: f64,
    pub direction: StepDirection,
}

impl Default for GeomStep {
    fn default() -> Self {
        GeomStep {
            color: (0, 0, 0),
            width: 1.5,
            alpha: 1.0,
            direction: StepDirection::Hv,
        }
    }
}

impl Geom for GeomStep {
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
        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        // One step line per group (colour / group / linetype level), so e.g.
        // a Kaplan–Meier curve per stratum or an ECDF per group.
        for rows in super::support::row_groups(data) {
            let mut raw: Vec<(f64, f64)> = rows
                .iter()
                .map(|&i| {
                    let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
                    let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
                    (nx, ny)
                })
                .collect();
            // Sort by x (stable, so tied x keep their row order).
            raw.sort_by(|a, b| a.0.total_cmp(&b.0));
            let step_points = super::ribbon::stepped(&raw, &self.direction);

            let points: Vec<(f64, f64)> = step_points
                .iter()
                .map(|&(nx, ny)| {
                    let (px, py) = coord.transform((nx, ny), &plot_area);
                    // Clamp non-finite coords (e.g. stat_ecdf's ±Inf padding, which
                    // extends the step to the panel edge) to the panel border so the
                    // flat segments draw to the edge instead of off-canvas.
                    let px = if px.is_finite() {
                        px
                    } else if nx < 0.0 {
                        plot_area.x
                    } else {
                        plot_area.x + plot_area.width
                    };
                    let py = if py.is_finite() {
                        py
                    } else if ny < 0.0 {
                        plot_area.y + plot_area.height
                    } else {
                        plot_area.y
                    };
                    (px, py)
                })
                .collect();

            let first = rows[0];
            let line_color = data
                .column("color")
                .and_then(|cc| scales.map_color(&Aesthetic::Color, &cc[first]))
                .unwrap_or(self.color);
            let linetype = data
                .column("linetype")
                .and_then(|c| scales.map_linetype(&c[first]))
                .unwrap_or(Linetype::Solid);
            let series = super::series_key(data, first);
            super::set_mark(backend, series.clone(), None, series, None);

            if points.len() >= 2 {
                backend.draw_line(
                    &points,
                    &LineStyle {
                        color: line_color,
                        alpha: self.alpha,
                        width: self.width,
                        linetype,
                    },
                )?;
            }
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
        "step"
    }

    /// stat_ecdf pads the step with ±Inf (ggplot2), drawn to the panel edge.
    fn allows_infinite(&self) -> bool {
        true
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.color = color;
    }
}
