use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineStyle, Linetype};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Horizontal reference line spanning the entire plot width.
pub struct GeomHline {
    pub yintercept: f64,
    pub color: (u8, u8, u8),
    pub width: f64,
    pub linetype: Linetype,
    pub alpha: f64,
}

impl GeomHline {
    pub fn new(yintercept: f64) -> Self {
        GeomHline {
            yintercept,
            color: (0, 0, 0),
            width: 1.0,
            linetype: Linetype::Dashed,
            alpha: 1.0,
        }
    }
}

impl Geom for GeomHline {
    fn draw(
        &self,
        _data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let plot_area = backend.plot_area();
        let y_scale = scales.get(&Aesthetic::Y);

        let ny = y_scale
            .map(|s| s.map(&Value::Float(self.yintercept)))
            .unwrap_or(0.5);

        let (left, y_px) = coord.transform((0.0, ny), &plot_area);
        let (right, _) = coord.transform((1.0, ny), &plot_area);

        backend.draw_line(
            &[(left, y_px), (right, y_px)],
            &LineStyle {
                color: self.color,
                alpha: self.alpha,
                width: self.width,
                linetype: self.linetype,
            },
        )?;

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![]
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
        "hline"
    }
}

/// Vertical reference line spanning the entire plot height.
pub struct GeomVline {
    pub xintercept: f64,
    pub color: (u8, u8, u8),
    pub width: f64,
    pub linetype: Linetype,
    pub alpha: f64,
}

impl GeomVline {
    pub fn new(xintercept: f64) -> Self {
        GeomVline {
            xintercept,
            color: (0, 0, 0),
            width: 1.0,
            linetype: Linetype::Dashed,
            alpha: 1.0,
        }
    }
}

impl Geom for GeomVline {
    fn draw(
        &self,
        _data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);

        let nx = x_scale
            .map(|s| s.map(&Value::Float(self.xintercept)))
            .unwrap_or(0.5);

        let (x_px, top) = coord.transform((nx, 1.0), &plot_area);
        let (_, bottom) = coord.transform((nx, 0.0), &plot_area);

        backend.draw_line(
            &[(x_px, top), (x_px, bottom)],
            &LineStyle {
                color: self.color,
                alpha: self.alpha,
                width: self.width,
                linetype: self.linetype,
            },
        )?;

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![]
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
        "vline"
    }
}

/// Arbitrary line y = slope*x + intercept spanning the plot.
pub struct GeomAbline {
    pub slope: f64,
    pub intercept: f64,
    pub color: (u8, u8, u8),
    pub width: f64,
    pub linetype: Linetype,
    pub alpha: f64,
}

impl GeomAbline {
    pub fn new(slope: f64, intercept: f64) -> Self {
        GeomAbline {
            slope,
            intercept,
            color: (0, 0, 0),
            width: 1.0,
            linetype: Linetype::Dashed,
            alpha: 1.0,
        }
    }
}

impl Geom for GeomAbline {
    fn draw(
        &self,
        _data: &DataFrame,
        coord: &dyn Coord,
        _scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let plot_area = backend.plot_area();

        // Sample points across the x-range using normalized coords
        let n_pts = 50;
        let points: Vec<(f64, f64)> = (0..=n_pts)
            .filter_map(|i| {
                let nx = i as f64 / n_pts as f64;
                let ny = self.slope * nx + self.intercept;
                if (-0.1..=1.1).contains(&ny) {
                    Some(coord.transform((nx, ny), &plot_area))
                } else {
                    None
                }
            })
            .collect();

        if points.len() >= 2 {
            backend.draw_line(
                &points,
                &LineStyle {
                    color: self.color,
                    alpha: self.alpha,
                    width: self.width,
                    linetype: self.linetype,
                },
            )?;
        }

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![]
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
        "abline"
    }
}
