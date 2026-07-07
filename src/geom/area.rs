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

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        let base_ny = y_scale.map(|s| s.map(&Value::Float(0.0))).unwrap_or(0.0);

        let mut upper: Vec<(f64, f64)> = Vec::new();
        let mut lower: Vec<(f64, f64)> = Vec::new();

        for i in 0..data.nrows() {
            let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
            let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
            upper.push(coord.transform((nx, ny), &plot_area));
            lower.push(coord.transform((nx, base_ny), &plot_area));
        }

        // Build polygon: upper left-to-right, then lower right-to-left
        let mut polygon = upper.clone();
        lower.reverse();
        polygon.extend(lower);

        if polygon.len() >= 3 {
            backend.draw_polygon(
                &polygon,
                &RectStyle {
                    fill: Some(self.fill),
                    stroke: None,
                    stroke_width: 0.0,
                    alpha: self.alpha,
                    clip: true,
                },
            )?;
        }

        // Draw the top line
        if upper.len() >= 2 {
            backend.draw_line(
                &upper,
                &LineStyle {
                    color: self.color,
                    alpha: 1.0,
                    width: self.line_width,
                    linetype: Linetype::Solid,
                },
            )?;
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
