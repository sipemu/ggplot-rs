//! Bullet chart (Stephen Few): `geom_bullet`. Per category, qualitative
//! range bands (dark = poor → light = good), a narrow bar for the actual
//! value and a perpendicular tick for the target. Horizontal by default.

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

/// Bullet chart. Map `x` (KPI) and `y` (actual value); name the target
/// column in `target` and give the range bands either as shared
/// breakpoints (`ranges`) or per-row columns (`range_columns`).
pub struct GeomBullet {
    /// Column holding each row's target (`None` draws no target tick).
    pub target: Option<String>,
    /// Shared band upper limits, ascending (used when `range_columns` is
    /// empty).
    pub ranges: Vec<f64>,
    /// Columns holding each row's band upper limits, ascending.
    pub range_columns: Vec<String>,
    /// Band colours from the first (poorest) band outward; recycled.
    pub band_colors: Vec<(u8, u8, u8)>,
    pub bar_color: (u8, u8, u8),
    pub target_color: (u8, u8, u8),
    /// Band, bar and target-tick widths as fractions of the category slot.
    pub band_width: f64,
    pub bar_width: f64,
    pub target_width: f64,
}

impl Default for GeomBullet {
    fn default() -> Self {
        GeomBullet {
            target: None,
            ranges: Vec::new(),
            range_columns: Vec::new(),
            band_colors: vec![(150, 150, 150), (190, 190, 190), (225, 225, 225)],
            bar_color: (40, 40, 40),
            target_color: (200, 30, 30),
            band_width: 0.75,
            bar_width: 0.28,
            target_width: 0.55,
        }
    }
}

impl GeomBullet {
    /// Bullet chart with targets from `target_column`.
    pub fn with_target(mut self, target_column: impl Into<String>) -> Self {
        self.target = Some(target_column.into());
        self
    }

    /// Shared band limits (ascending), e.g. `[50.0, 75.0, 100.0]`.
    pub fn with_ranges(mut self, ranges: impl IntoIterator<Item = f64>) -> Self {
        self.ranges = ranges.into_iter().collect();
        self
    }

    /// Per-row band limits from these columns (ascending).
    pub fn with_range_columns<S: Into<String>>(
        mut self,
        cols: impl IntoIterator<Item = S>,
    ) -> Self {
        self.range_columns = cols.into_iter().map(Into::into).collect();
        self
    }

    fn row_ranges(&self, data: &DataFrame, i: usize) -> Vec<f64> {
        let mut r: Vec<f64> = if self.range_columns.is_empty() {
            self.ranges.clone()
        } else {
            self.range_columns
                .iter()
                .filter_map(|c| data.column(c).and_then(|col| col[i].as_f64()))
                .collect()
        };
        r.retain(|v| v.is_finite());
        r.sort_by(|a, b| a.total_cmp(b));
        r
    }
}

impl Geom for GeomBullet {
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
        let target_col = self.target.as_deref().and_then(|c| data.column(c));
        let area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let my = |v: f64| {
            scales
                .get(&Aesthetic::Y)
                .map(|s| s.map(&Value::Float(v)))
                .unwrap_or(0.0)
        };
        let slot = if x_scale.map(|s| s.is_discrete()).unwrap_or(false) {
            let n = x_scale.map(|s| s.breaks().len()).unwrap_or(1).max(1) as f64;
            1.0 / (n * if coord.discrete_expand() { 1.1 } else { 1.0 })
        } else {
            2.0 * super::continuous_bar_half_width(
                x_col.iter().filter_map(|v| x_scale.map(|s| s.map(v))),
                1.0,
                0.05,
            )
        };
        let rect = |b: &mut dyn DrawBackend, nx: f64, frac: f64, lo: f64, hi: f64, fill| {
            let h = slot * frac / 2.0;
            let (a, c) = (
                coord.transform((nx - h, my(hi)), &area),
                coord.transform((nx + h, my(lo)), &area),
            );
            b.draw_rect(
                (a.0.min(c.0), a.1.min(c.1)),
                (a.0.max(c.0), a.1.max(c.1)),
                &RectStyle {
                    fill: Some(fill),
                    stroke: None,
                    stroke_width: 0.0,
                    alpha: 1.0,
                    clip: true,
                    corner_radius: 0.0,
                },
            )
        };

        for i in 0..data.nrows() {
            let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.5);
            let Some(actual) = y_col[i].as_f64() else {
                continue;
            };
            let x_txt = super::tip_value(&x_col[i]);
            super::clear_mark(backend);
            let mut lo = 0.0;
            for (k, hi) in self.row_ranges(data, i).into_iter().enumerate() {
                let color = self.band_colors[k % self.band_colors.len().max(1)];
                rect(backend, nx, self.band_width, lo, hi, color)?;
                lo = hi;
            }
            let target = target_col.and_then(|c| c[i].as_f64());
            let tip = match target {
                Some(t) => format!(
                    "{x_txt}: {} (target {})",
                    super::tip_value(&y_col[i]),
                    super::tip_value(&Value::Float(t))
                ),
                None => format!("{x_txt}: {}", super::tip_value(&y_col[i])),
            };
            super::set_mark(
                backend,
                Some(tip),
                Some(x_txt),
                super::series_key(data, i),
                super::raw_value(&y_col[i]),
            );
            super::set_row_extras(backend, data, i);
            rect(backend, nx, self.bar_width, 0.0, actual, self.bar_color)?;
            if let Some(t) = target {
                let h = slot * self.target_width / 2.0;
                backend.draw_line(
                    &[
                        coord.transform((nx - h, my(t)), &area),
                        coord.transform((nx + h, my(t)), &area),
                    ],
                    &LineStyle {
                        color: self.target_color,
                        alpha: 1.0,
                        width: 3.0,
                        linetype: Linetype::Solid,
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
        "bullet"
    }
    fn include_zero_baseline(&self) -> bool {
        true
    }
    fn consumes_tooltips(&self) -> bool {
        true
    }
    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.bar_color = color;
    }

    /// The y scale must hold the bands and targets, not just the actuals.
    fn setup_data(&self, data: &mut DataFrame, _warnings: &mut Vec<String>) {
        let n = data.nrows();
        let target_col = self.target.as_deref().and_then(|c| data.column(c));
        let y_col = data.column("y");
        let ymax: Vec<Value> = (0..n)
            .map(|i| {
                let mut m = y_col.and_then(|c| c[i].as_f64()).unwrap_or(f64::NAN);
                if let Some(t) = target_col.and_then(|c| c[i].as_f64()) {
                    m = m.max(t);
                }
                for r in self.row_ranges(data, i) {
                    m = m.max(r);
                }
                Value::Float(m)
            })
            .collect();
        if !data.has_column("ymax") {
            data.add_column("ymax".into(), ymax);
        }
    }
}
