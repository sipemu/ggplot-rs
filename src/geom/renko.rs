//! Renko and point-and-figure charts: `geom_renko` (bricks) and
//! `geom_point_figure` (columns of X and O). Map `y` to the price in time
//! order; the stats ([`StatRenko`], [`StatPointFigure`]) replace time with
//! brick / column numbers.

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineStyle, Linetype, PointStyle, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::renko::{StatPointFigure, StatRenko};
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Renko (`point_figure: false`) or point-and-figure chart.
pub struct GeomRenko {
    pub box_size: f64,
    /// Point-and-figure boxes needed to reverse a column.
    pub reversal: usize,
    pub point_figure: bool,
    pub up: (u8, u8, u8),
    pub down: (u8, u8, u8),
}

impl GeomRenko {
    pub fn renko(box_size: f64) -> Self {
        GeomRenko {
            box_size,
            reversal: 3,
            point_figure: false,
            up: (0x0c, 0xa6, 0x78),
            down: (0xe0, 0x31, 0x31),
        }
    }

    pub fn point_figure(box_size: f64, reversal: usize) -> Self {
        GeomRenko {
            reversal,
            point_figure: true,
            ..Self::renko(box_size)
        }
    }
}

impl Geom for GeomRenko {
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
        let (xc, y0c, y1c) = (col("x")?, col("ymin")?, col("ymax")?);
        let kind = if self.point_figure {
            col("mark")?
        } else {
            col("direction")?
        };
        let area = backend.plot_area();
        let m = |aes: Aesthetic, v: &Value| scales.get(&aes).map(|s| s.map(v)).unwrap_or(0.0);
        let half = scales
            .get(&Aesthetic::X)
            .map(|s| {
                let d = s.map(&Value::Float(2.0)) - s.map(&Value::Float(1.0));
                0.42 * d.abs()
            })
            .unwrap_or(0.01);
        for i in 0..data.nrows() {
            let nx = m(Aesthetic::X, &xc[i]);
            let up = matches!(kind[i].key_str().as_ref(), "up" | "X");
            let color = if up { self.up } else { self.down };
            let a = coord.transform((nx - half, m(Aesthetic::Y, &y1c[i])), &area);
            let b = coord.transform((nx + half, m(Aesthetic::Y, &y0c[i])), &area);
            let (l, t, r, btm) = (a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1));
            super::set_mark(
                backend,
                Some(format!(
                    "{} {}–{}",
                    kind[i].key_str(),
                    super::tip_value(&y0c[i]),
                    super::tip_value(&y1c[i])
                )),
                Some(super::tip_value(&xc[i])),
                Some(kind[i].key_str().into_owned()),
                super::raw_value(&y1c[i]),
            );
            if !self.point_figure {
                backend.draw_rect(
                    (l, t),
                    (r, btm),
                    &RectStyle {
                        fill: Some(color),
                        stroke: Some((255, 255, 255)),
                        stroke_width: 0.5,
                        alpha: 1.0,
                        clip: true,
                        corner_radius: 0.0,
                    },
                )?;
            } else if up {
                let s = LineStyle {
                    color,
                    alpha: 1.0,
                    width: 1.5,
                    linetype: Linetype::Solid,
                    ..Default::default()
                };
                let inset = 0.15 * (r - l).min(btm - t);
                backend.draw_line(&[(l + inset, t + inset), (r - inset, btm - inset)], &s)?;
                backend.draw_line(&[(l + inset, btm - inset), (r - inset, t + inset)], &s)?;
            } else {
                backend.draw_circle(
                    ((l + r) / 2.0, (t + btm) / 2.0),
                    0.38 * (r - l).min(btm - t),
                    &PointStyle {
                        color,
                        alpha: 1.0,
                        filled: false,
                        ..Default::default()
                    },
                )?;
            }
        }
        super::clear_mark(backend);
        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Y]
    }
    fn default_stat(&self) -> Box<dyn Stat> {
        if self.point_figure {
            Box::new(StatPointFigure {
                box_size: self.box_size,
                reversal: self.reversal,
            })
        } else {
            Box::new(StatRenko {
                box_size: self.box_size,
            })
        }
    }
    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }
    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }
    fn name(&self) -> &str {
        if self.point_figure {
            "point_figure"
        } else {
            "renko"
        }
    }
}
