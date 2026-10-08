//! Financial OHLC charts: `geom_candlestick` (body + wick) and `geom_ohlc`
//! (bar with open/close ticks). Map `x` plus the `open`/`high`/`low`/`close`
//! aesthetics; the y scale trains on the high–low range.

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

/// Candlestick chart: a wick from low to high and a body from open to close,
/// coloured `up` when close ≥ open and `down` otherwise.
pub struct GeomCandlestick {
    /// Colour of rising periods (close ≥ open).
    pub up: (u8, u8, u8),
    /// Colour of falling periods (close < open).
    pub down: (u8, u8, u8),
    /// Body width as a fraction of the x spacing (category slot / resolution).
    pub width: f64,
    pub line_width: f64,
    pub alpha: f64,
}

impl Default for GeomCandlestick {
    fn default() -> Self {
        GeomCandlestick {
            up: (0x0c, 0xa6, 0x78),
            down: (0xe0, 0x31, 0x31),
            width: 0.6,
            line_width: 1.0,
            alpha: 1.0,
        }
    }
}

/// OHLC bar chart: a vertical high–low bar with a left tick at the open and a
/// right tick at the close, coloured `up`/`down` like [`GeomCandlestick`].
pub struct GeomOhlc {
    pub up: (u8, u8, u8),
    pub down: (u8, u8, u8),
    /// Tick span as a fraction of the x spacing.
    pub width: f64,
    pub line_width: f64,
    pub alpha: f64,
}

impl Default for GeomOhlc {
    fn default() -> Self {
        GeomOhlc {
            up: (0x0c, 0xa6, 0x78),
            down: (0xe0, 0x31, 0x31),
            width: 0.6,
            line_width: 1.4,
            alpha: 1.0,
        }
    }
}

fn ohlc_required() -> Vec<Aesthetic> {
    vec![
        Aesthetic::X,
        Aesthetic::Open,
        Aesthetic::High,
        Aesthetic::Low,
        Aesthetic::Close,
    ]
}

/// The y scale trains on `[low, high]`: expose them as `ymin`/`ymax`.
fn ohlc_setup(data: &mut DataFrame) {
    for (src, dst) in [("low", "ymin"), ("high", "ymax")] {
        if !data.has_column(dst) {
            if let Some(c) = data.column(src) {
                let c = c.to_vec();
                data.add_column(dst.to_string(), c);
            }
        }
    }
}

/// One period, mapped to normalized panel coordinates.
struct Period {
    nx: f64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    up: bool,
}

/// Shared walk over the rows: maps x/ohlc, sets hover metadata, and calls
/// `draw(period, half_width, backend)` for each complete row.
fn for_each_period(
    data: &DataFrame,
    scales: &ScaleSet,
    width: f64,
    backend: &mut dyn DrawBackend,
    mut draw: impl FnMut(&Period, f64, &mut dyn DrawBackend) -> Result<(), RenderError>,
) -> Result<(), RenderError> {
    let col = |n: &str| {
        data.column(n)
            .ok_or_else(|| RenderError::MissingAesthetic(n.into()))
    };
    let (x_col, o_col, h_col, l_col, c_col) = (
        col("x")?,
        col("open")?,
        col("high")?,
        col("low")?,
        col("close")?,
    );
    let x_scale = scales.get(&Aesthetic::X);
    let y_scale = scales.get(&Aesthetic::Y);
    let my = |v: f64| y_scale.map(|s| s.map(&Value::Float(v))).unwrap_or(0.0);
    let half = if x_scale.map(|s| s.is_discrete()).unwrap_or(false) {
        let n = x_scale.map(|s| s.breaks().len()).unwrap_or(1).max(1) as f64;
        width / n / 2.0
    } else {
        super::continuous_bar_half_width(
            x_col.iter().filter_map(|v| x_scale.map(|s| s.map(v))),
            width,
            0.01,
        )
    };
    for i in 0..data.nrows() {
        let (Some(o), Some(h), Some(l), Some(c)) = (
            o_col[i].as_f64(),
            h_col[i].as_f64(),
            l_col[i].as_f64(),
            c_col[i].as_f64(),
        ) else {
            continue;
        };
        let p = Period {
            nx: x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.5),
            open: my(o),
            high: my(h),
            low: my(l),
            close: my(c),
            up: c >= o,
        };
        let x = super::tip_value(&x_col[i]);
        let f = |v: &Value| super::tip_value(v);
        let tip = format!(
            "{x} — O {} H {} L {} C {}",
            f(&o_col[i]),
            f(&h_col[i]),
            f(&l_col[i]),
            f(&c_col[i])
        );
        let series =
            super::series_key(data, i).or_else(|| Some(if p.up { "up" } else { "down" }.into()));
        super::set_mark(
            backend,
            Some(tip),
            Some(x),
            series,
            super::raw_value(&c_col[i]),
        );
        draw(&p, half, backend)?;
    }
    super::clear_mark(backend);
    Ok(())
}

impl Geom for GeomCandlestick {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let area = backend.plot_area();
        for_each_period(data, scales, self.width, backend, |p, half, b| {
            let color = if p.up { self.up } else { self.down };
            let wick = [
                coord.transform((p.nx, p.low), &area),
                coord.transform((p.nx, p.high), &area),
            ];
            b.draw_line(
                &wick,
                &LineStyle {
                    color,
                    alpha: self.alpha,
                    width: self.line_width,
                    linetype: Linetype::Solid,
                },
            )?;
            let (x0, y0) = coord.transform((p.nx - half, p.open.max(p.close)), &area);
            let (x1, y1) = coord.transform((p.nx + half, p.open.min(p.close)), &area);
            // A doji (open == close) still shows a 1px body.
            let (top, bottom) = (y0.min(y1), y0.max(y1).max(y0.min(y1) + 1.0));
            b.draw_rect(
                (x0.min(x1), top),
                (x0.max(x1), bottom),
                &RectStyle {
                    fill: Some(color),
                    stroke: Some(color),
                    stroke_width: self.line_width * 0.5,
                    alpha: self.alpha,
                    clip: true,
                },
            )
        })
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        ohlc_required()
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
        "candlestick"
    }
    fn setup_data(&self, data: &mut DataFrame) {
        ohlc_setup(data);
    }
}

impl Geom for GeomOhlc {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let area = backend.plot_area();
        for_each_period(data, scales, self.width, backend, |p, half, b| {
            let style = LineStyle {
                color: if p.up { self.up } else { self.down },
                alpha: self.alpha,
                width: self.line_width,
                linetype: Linetype::Solid,
            };
            let t = |x: f64, y: f64| coord.transform((x, y), &area);
            b.draw_line(&[t(p.nx, p.low), t(p.nx, p.high)], &style)?;
            b.draw_line(&[t(p.nx - half, p.open), t(p.nx, p.open)], &style)?;
            b.draw_line(&[t(p.nx, p.close), t(p.nx + half, p.close)], &style)
        })
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        ohlc_required()
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
        "ohlc"
    }
    fn setup_data(&self, data: &mut DataFrame) {
        ohlc_setup(data);
    }
}
