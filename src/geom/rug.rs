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

/// Rug geometry — tick marks along axis margins.
pub struct GeomRug {
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub length: f64,
    pub sides: String,
}

impl Default for GeomRug {
    fn default() -> Self {
        GeomRug {
            color: (0, 0, 0),
            alpha: 0.5,
            length: 0.03,
            sides: "bl".to_string(),
        }
    }
}

impl Geom for GeomRug {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let x_col = data.column("x");
        let y_col = data.column("y");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        let style = LineStyle {
            color: self.color,
            alpha: self.alpha,
            width: 0.5,
            linetype: Linetype::Solid,
        };

        let len = self.length;

        if self.sides.contains('b') {
            if let Some(xc) = x_col {
                for val in xc {
                    let nx = x_scale.map(|s| s.map(val)).unwrap_or(0.0);
                    let (px, py_bottom) = coord.transform((nx, 0.0), &plot_area);
                    let (_, py_up) = coord.transform((nx, len), &plot_area);
                    backend.draw_line(&[(px, py_bottom), (px, py_up)], &style)?;
                }
            }
        }

        if self.sides.contains('t') {
            if let Some(xc) = x_col {
                for val in xc {
                    let nx = x_scale.map(|s| s.map(val)).unwrap_or(0.0);
                    let (px, py_top) = coord.transform((nx, 1.0), &plot_area);
                    let (_, py_down) = coord.transform((nx, 1.0 - len), &plot_area);
                    backend.draw_line(&[(px, py_top), (px, py_down)], &style)?;
                }
            }
        }

        if self.sides.contains('l') {
            if let Some(yc) = y_col {
                for val in yc {
                    let ny = y_scale.map(|s| s.map(val)).unwrap_or(0.0);
                    let (px_left, py) = coord.transform((0.0, ny), &plot_area);
                    let (px_right, _) = coord.transform((len, ny), &plot_area);
                    backend.draw_line(&[(px_left, py), (px_right, py)], &style)?;
                }
            }
        }

        if self.sides.contains('r') {
            if let Some(yc) = y_col {
                for val in yc {
                    let ny = y_scale.map(|s| s.map(val)).unwrap_or(0.0);
                    let (px_right, py) = coord.transform((1.0, ny), &plot_area);
                    let (px_left, _) = coord.transform((1.0 - len, ny), &plot_area);
                    backend.draw_line(&[(px_right, py), (px_left, py)], &style)?;
                }
            }
        }

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X]
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
        "rug"
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.color = color;
    }
}
