//! Unit charts: `geom_waffle` (a square grid of units) and
//! `geom_parliament` (seats on concentric half-circle arcs). Each unit is
//! one item of its `fill` category. Layout by [`StatWaffle`].

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, PointStyle, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::waffle::{StatWaffle, UnitLayout};
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Waffle / parliament chart. Map `fill` (category) and `weight` (units
/// per row; 1 when unmapped).
pub struct GeomWaffle {
    pub layout: UnitLayout,
    /// Gap between units as a fraction of the unit size.
    pub gap: f64,
    /// Corner radius of waffle squares as a fraction of their size.
    pub roundness: f64,
    /// Fill when `fill` is unmapped.
    pub fill: (u8, u8, u8),
    pub alpha: f64,
}

impl Default for GeomWaffle {
    fn default() -> Self {
        GeomWaffle {
            layout: UnitLayout::Waffle { rows: 10 },
            gap: 0.12,
            roundness: 0.0,
            fill: (97, 156, 255),
            alpha: 1.0,
        }
    }
}

impl GeomWaffle {
    /// A waffle `rows` units high.
    pub fn waffle(rows: usize) -> Self {
        GeomWaffle {
            layout: UnitLayout::Waffle { rows },
            ..Default::default()
        }
    }

    /// A parliament (hemicycle); `rows = 0` picks the number of arcs.
    pub fn parliament(rows: usize) -> Self {
        GeomWaffle {
            layout: UnitLayout::Parliament { rows, inner: 0.4 },
            ..Default::default()
        }
    }
}

impl Geom for GeomWaffle {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let col = |n: &str| {
            data.column(n)
                .ok_or_else(|| RenderError::MissingAesthetic(n.into()))
        };
        let (xc, yc, uc) = (col("x")?, col("y")?, col("unit")?);
        let fill_col = data.column("fill");
        let area = backend.plot_area();
        let px = |x: f64, y: f64| {
            let sx = scales.get(&Aesthetic::X).map(|s| s.map(&Value::Float(x)));
            let sy = scales.get(&Aesthetic::Y).map(|s| s.map(&Value::Float(y)));
            coord.transform((sx.unwrap_or(0.5), sy.unwrap_or(0.5)), &area)
        };
        // Pixel size of one unit radius: the smaller axis scale keeps squares
        // square and seats round whatever the panel aspect.
        let unit_px = |x: f64, y: f64, u: f64| {
            let c = px(x, y);
            let (a, b) = (px(x + u, y), px(x, y + u));
            ((a.0 - c.0).hypot(a.1 - c.1)).min((b.0 - c.0).hypot(b.1 - c.1))
        };
        let square = matches!(self.layout, UnitLayout::Waffle { .. });
        for i in 0..data.nrows() {
            let (Some(x), Some(y), Some(u)) = (xc[i].as_f64(), yc[i].as_f64(), uc[i].as_f64())
            else {
                continue;
            };
            let c = px(x, y);
            let r = unit_px(x, y, u) * (1.0 - self.gap);
            let fill = fill_col
                .and_then(|f| scales.map_color(&Aesthetic::Fill, &f[i]))
                .unwrap_or(self.fill);
            let series = super::series_key(data, i);
            super::set_mark(backend, series.clone(), None, series, None);
            super::set_row_extras(backend, data, i);
            if square {
                backend.draw_rect(
                    (c.0 - r, c.1 - r),
                    (c.0 + r, c.1 + r),
                    &RectStyle {
                        fill: Some(fill),
                        stroke: None,
                        stroke_width: 0.0,
                        alpha: self.alpha,
                        clip: true,
                        corner_radius: 2.0 * r * self.roundness,
                    },
                )?;
            } else {
                backend.draw_circle(
                    c,
                    r,
                    &PointStyle {
                        color: fill,
                        alpha: self.alpha,
                        filled: true,
                        ..Default::default()
                    },
                )?;
            }
        }
        super::clear_mark(backend);
        super::clear_row_extras(backend);
        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        Vec::new()
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatWaffle {
            layout: self.layout,
        })
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "waffle"
    }

    fn consumes_tooltips(&self) -> bool {
        true
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }
}
