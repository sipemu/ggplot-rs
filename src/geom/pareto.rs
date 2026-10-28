//! Pareto chart: `geom_pareto`. Categories as bars sorted by value, plus the
//! cumulative total as a line with points, read against a 0–100 % secondary
//! axis ([`SecAxis::percent_of_max`](crate::scale::sec_axis::SecAxis::percent_of_max)).

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineStyle, Linetype, PointStyle, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::pareto::StatPareto;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Pareto chart. Map `x` (categories) and `y` (values; repeated categories
/// are summed). Bars are sorted descending unless `sort` is false.
pub struct GeomPareto {
    /// Bar fill.
    pub fill: (u8, u8, u8),
    /// Bar outline.
    pub color: (u8, u8, u8),
    /// Cumulative line and point colour.
    pub line_color: (u8, u8, u8),
    pub line_width: f64,
    /// Point radius in pixels (0 hides the points).
    pub point_size: f64,
    /// Bar width as a fraction of the category slot.
    pub width: f64,
    pub alpha: f64,
    /// Sort categories by descending value.
    pub sort: bool,
    /// Draw a dashed reference line at this cumulative share (e.g. `0.8`
    /// for the 80/20 rule).
    pub threshold: Option<f64>,
}

impl Default for GeomPareto {
    fn default() -> Self {
        GeomPareto {
            fill: (97, 156, 255),
            color: (50, 50, 50),
            line_color: (0xe8, 0x59, 0x0c),
            line_width: 1.5,
            point_size: 3.0,
            width: 0.8,
            alpha: 1.0,
            sort: true,
            threshold: None,
        }
    }
}

impl GeomPareto {
    /// Draw a dashed reference line at cumulative share `share` (0–1).
    pub fn with_threshold(mut self, share: f64) -> Self {
        self.threshold = Some(share);
        self
    }
}

impl Geom for GeomPareto {
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
        let (x_col, y_col, cum_col) = (col("x")?, col("y")?, col("cum")?);
        let pct_col = data.column("cum_pct");
        let area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);
        let my = |v: f64| y_scale.map(|s| s.map(&Value::Float(v))).unwrap_or(0.0);
        let half = if x_scale.map(|s| s.is_discrete()).unwrap_or(false) {
            let n = x_scale.map(|s| s.breaks().len()).unwrap_or(1).max(1) as f64;
            let pad = if coord.discrete_expand() { 1.1 } else { 1.0 };
            self.width / (n * pad) / 2.0
        } else {
            super::continuous_bar_half_width(
                x_col.iter().filter_map(|v| x_scale.map(|s| s.map(v))),
                self.width,
                0.02,
            )
        };
        let pct_txt = |i: usize| {
            pct_col
                .and_then(|c| c[i].as_f64())
                .map(|p| {
                    format!(
                        "{}%",
                        super::tip_value(&Value::Float((p * 10.0).round() / 10.0))
                    )
                })
                .unwrap_or_default()
        };

        let mut line = Vec::with_capacity(data.nrows());
        for i in 0..data.nrows() {
            let (Some(v), Some(cum)) = (y_col[i].as_f64(), cum_col[i].as_f64()) else {
                continue;
            };
            let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.5);
            let x_txt = super::tip_value(&x_col[i]);
            super::set_mark(
                backend,
                Some(format!(
                    "{x_txt}: {} (cumulative {})",
                    super::tip_value(&y_col[i]),
                    pct_txt(i)
                )),
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
            let (x0, y0) = coord.transform((nx - half, my(v)), &area);
            let (x1, y1) = coord.transform((nx + half, my(0.0)), &area);
            backend.draw_rect(
                (x0.min(x1), y0.min(y1)),
                (x0.max(x1), y0.max(y1)),
                &RectStyle {
                    fill: Some(self.fill),
                    stroke: Some(self.color),
                    stroke_width: 0.5,
                    alpha: self.alpha,
                    clip: true,
                    corner_radius: 0.0,
                },
            )?;
            line.push((i, nx, my(cum)));
        }
        super::clear_row_extras(backend);
        line.sort_by(|a, b| a.1.total_cmp(&b.1));

        if let (Some(share), Some(total)) = (
            self.threshold,
            cum_col.iter().filter_map(|v| v.as_f64()).reduce(f64::max),
        ) {
            super::clear_mark(backend);
            let ny = my(share * total);
            backend.draw_line(
                &[
                    coord.transform((0.0, ny), &area),
                    coord.transform((1.0, ny), &area),
                ],
                &LineStyle {
                    color: self.line_color,
                    alpha: 0.7 * self.alpha,
                    width: 1.0,
                    linetype: Linetype::Dashed,
                    ..Default::default()
                },
            )?;
        }

        let pts: Vec<(f64, f64)> = line
            .iter()
            .map(|&(_, nx, ny)| coord.transform((nx, ny), &area))
            .collect();
        if pts.len() > 1 {
            super::set_mark(backend, None, None, Some("cumulative".into()), None);
            backend.draw_line(
                &pts,
                &LineStyle {
                    color: self.line_color,
                    alpha: self.alpha,
                    width: self.line_width,
                    linetype: Linetype::Solid,
                    ..Default::default()
                },
            )?;
        }
        if self.point_size > 0.0 {
            for (&(i, _, _), &p) in line.iter().zip(&pts) {
                let x_txt = super::tip_value(&x_col[i]);
                super::set_mark(
                    backend,
                    Some(format!("{x_txt}: cumulative {}", pct_txt(i))),
                    Some(x_txt),
                    Some("cumulative".into()),
                    pct_col.and_then(|c| super::raw_value(&c[i])),
                );
                backend.draw_circle(
                    p,
                    self.point_size,
                    &PointStyle {
                        color: self.line_color,
                        alpha: self.alpha,
                        filled: true,
                        ..Default::default()
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
        Box::new(StatPareto { sort: self.sort })
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "pareto"
    }

    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let (true, Some(ink)) = (self.color == (50, 50, 50), el.ink) {
            self.color = ink;
        }
    }

    fn consumes_tooltips(&self) -> bool {
        true
    }

    fn include_zero_baseline(&self) -> bool {
        true
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }
}
