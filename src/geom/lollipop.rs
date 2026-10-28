//! Lollipop and dumbbell charts: `geom_lollipop` (a stem from the baseline
//! to the value, capped with a dot) and `geom_dumbbell` (a segment between
//! two values per category, with a dot at each end). One layer each; both
//! turn horizontal under `coord_flip`.

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineStyle, Linetype, PointStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Lollipop chart: map `x` and `y` (and optionally `color`).
pub struct GeomLollipop {
    pub color: (u8, u8, u8),
    /// Dot radius in pixels.
    pub point_size: f64,
    pub stem_width: f64,
    /// Value the stems start from.
    pub baseline: f64,
    pub alpha: f64,
}

impl Default for GeomLollipop {
    fn default() -> Self {
        GeomLollipop {
            color: (31, 119, 180),
            point_size: 5.0,
            stem_width: 1.5,
            baseline: 0.0,
            alpha: 1.0,
        }
    }
}

/// Dumbbell chart: map `x` (category) plus `ymin` and `ymax` (the two
/// values); `coord_flip` for the usual horizontal layout.
pub struct GeomDumbbell {
    /// Dot colour at `ymin`.
    pub low_color: (u8, u8, u8),
    /// Dot colour at `ymax`.
    pub high_color: (u8, u8, u8),
    /// Connecting segment colour.
    pub line_color: (u8, u8, u8),
    pub point_size: f64,
    pub line_width: f64,
    pub alpha: f64,
}

impl Default for GeomDumbbell {
    fn default() -> Self {
        GeomDumbbell {
            low_color: (160, 160, 160),
            high_color: (31, 119, 180),
            line_color: (190, 190, 190),
            point_size: 5.0,
            line_width: 3.0,
            alpha: 1.0,
        }
    }
}

fn dot(
    backend: &mut dyn DrawBackend,
    at: (f64, f64),
    r: f64,
    color: (u8, u8, u8),
    alpha: f64,
) -> Result<(), RenderError> {
    backend.draw_circle(
        at,
        r,
        &PointStyle {
            color,
            alpha,
            filled: true,
            ..Default::default()
        },
    )
}

fn stem(
    backend: &mut dyn DrawBackend,
    a: (f64, f64),
    b: (f64, f64),
    color: (u8, u8, u8),
    width: f64,
    alpha: f64,
) -> Result<(), RenderError> {
    backend.draw_line(
        &[a, b],
        &LineStyle {
            color,
            alpha,
            width,
            linetype: Linetype::Solid,
            ..Default::default()
        },
    )
}

impl Geom for GeomLollipop {
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
        let color_col = data.column("color");
        let area = backend.plot_area();
        let xm = super::support::XMapper::new(data, scales);
        let my = |v: &Value| scales.get(&Aesthetic::Y).map(|s| s.map(v)).unwrap_or(0.0);
        let base = my(&Value::Float(self.baseline));
        for i in 0..data.nrows() {
            let (nx, ny) = (xm.map(&x_col[i], i), my(&y_col[i]));
            if !(nx.is_finite() && ny.is_finite()) {
                continue;
            }
            let color = color_col
                .and_then(|c| scales.map_color(&Aesthetic::Color, &c[i]))
                .unwrap_or(self.color);
            let x_txt = super::tip_value(&x_col[i]);
            super::set_mark(
                backend,
                Some(format!("{x_txt}: {}", super::tip_value(&y_col[i]))),
                Some(x_txt),
                super::series_key(data, i),
                super::raw_value(&y_col[i]),
            );
            super::set_mark_formatted(
                backend,
                scales,
                Some(&x_col[i]),
                &Aesthetic::Y,
                Some(&y_col[i]),
            );
            super::set_row_extras(backend, data, i);
            let (top, bottom) = (
                coord.transform((nx, ny), &area),
                coord.transform((nx, base), &area),
            );
            stem(backend, bottom, top, color, self.stem_width, self.alpha)?;
            dot(backend, top, self.point_size, color, self.alpha)?;
        }
        super::clear_mark(backend);
        super::clear_row_extras(backend);
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
        "lollipop"
    }
    fn include_zero_baseline(&self) -> bool {
        self.baseline == 0.0
    }
    fn consumes_tooltips(&self) -> bool {
        true
    }
    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.color = color;
    }
}

impl Geom for GeomDumbbell {
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
        let (x_col, lo_col, hi_col) = (col("x")?, col("ymin")?, col("ymax")?);
        let area = backend.plot_area();
        let xm = super::support::XMapper::new(data, scales);
        let my = |v: &Value| scales.get(&Aesthetic::Y).map(|s| s.map(v)).unwrap_or(0.0);
        for i in 0..data.nrows() {
            let (nx, lo, hi) = (xm.map(&x_col[i], i), my(&lo_col[i]), my(&hi_col[i]));
            if ![nx, lo, hi].iter().all(|v| v.is_finite()) {
                continue;
            }
            let x_txt = super::tip_value(&x_col[i]);
            let (l, h) = (super::tip_value(&lo_col[i]), super::tip_value(&hi_col[i]));
            super::set_mark(
                backend,
                Some(format!("{x_txt}: {l} → {h}")),
                Some(x_txt),
                super::series_key(data, i),
                super::raw_value(&hi_col[i]),
            );
            super::set_row_extras(backend, data, i);
            let (a, b) = (
                coord.transform((nx, lo), &area),
                coord.transform((nx, hi), &area),
            );
            stem(backend, a, b, self.line_color, self.line_width, self.alpha)?;
            dot(backend, a, self.point_size, self.low_color, self.alpha)?;
            dot(backend, b, self.point_size, self.high_color, self.alpha)?;
        }
        super::clear_mark(backend);
        super::clear_row_extras(backend);
        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Ymin, Aesthetic::Ymax]
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
        "dumbbell"
    }
    fn consumes_tooltips(&self) -> bool {
        true
    }
    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.high_color = color;
    }
}
