//! Waterfall (bridge) chart: `geom_waterfall`. Each step is a bar floating
//! from the previous running total; total bars rise from 0. Bars are coloured
//! through the fill scale on the stat's `direction` column (Increase /
//! Decrease / Total), and thin connectors join each bar's end to the next bar.

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineStyle, Linetype, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::waterfall::{StatWaterfall, WATERFALL_DECREASE, WATERFALL_TOTAL};
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Waterfall chart. Map `x` (the steps, in data order) and `y` (each step's
/// change). List subtotal steps in `totals` (their `y` is ignored) and set
/// `final_total` to append a closing total bar.
pub struct GeomWaterfall {
    /// Fill of rising steps (also the default `Increase` legend key).
    pub up: (u8, u8, u8),
    /// Fill of falling steps.
    pub down: (u8, u8, u8),
    /// Fill of total bars.
    pub total: (u8, u8, u8),
    /// Bar outline colour.
    pub color: (u8, u8, u8),
    /// x levels drawn as totals (0 → running total).
    pub totals: Vec<String>,
    /// Append a closing total bar with this x label.
    pub final_total: Option<String>,
    /// Draw dashed connectors from each bar's end to the next bar.
    pub connectors: bool,
    pub connector_color: (u8, u8, u8),
    /// Bar width as a fraction of the category slot.
    pub width: f64,
    pub alpha: f64,
}

impl Default for GeomWaterfall {
    fn default() -> Self {
        GeomWaterfall {
            up: (0x0c, 0xa6, 0x78),
            down: (0xe0, 0x31, 0x31),
            total: (0x4c, 0x6e, 0xa8),
            color: (50, 50, 50),
            totals: Vec::new(),
            final_total: None,
            connectors: true,
            connector_color: (110, 110, 110),
            width: 0.7,
            alpha: 1.0,
        }
    }
}

impl GeomWaterfall {
    /// Mark these x levels as subtotal bars.
    pub fn with_totals<S: Into<String>>(mut self, levels: impl IntoIterator<Item = S>) -> Self {
        self.totals = levels.into_iter().map(Into::into).collect();
        self
    }

    /// Append a closing total bar labelled `label`.
    pub fn with_final_total(mut self, label: impl Into<String>) -> Self {
        self.final_total = Some(label.into());
        self
    }

    /// Whether any total bar will be drawn (so the legend needs a Total key).
    pub(crate) fn has_totals(&self) -> bool {
        !self.totals.is_empty() || self.final_total.is_some()
    }

    fn direction_color(&self, dir: &Value) -> (u8, u8, u8) {
        match dir.key_str().as_ref() {
            WATERFALL_TOTAL => self.total,
            WATERFALL_DECREASE => self.down,
            _ => self.up,
        }
    }
}

impl Geom for GeomWaterfall {
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
        let (x_col, y_col, ymin_col, ymax_col) = (col("x")?, col("y")?, col("ymin")?, col("ymax")?);
        let delta_col = data.column("delta");
        let dir_col = data.column("direction");
        let fill_col = data.column("fill");
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

        // (nx, running total after the step) of the previous bar, for connectors.
        let mut prev: Option<(f64, f64)> = None;
        for i in 0..data.nrows() {
            let (Some(lo), Some(hi), Some(end)) = (
                ymin_col[i].as_f64(),
                ymax_col[i].as_f64(),
                y_col[i].as_f64(),
            ) else {
                continue;
            };
            let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.5);
            let (nlo, nhi, nend) = (my(lo), my(hi), my(end));

            if self.connectors {
                if let Some((px, pend)) = prev {
                    super::clear_mark(backend);
                    let a = coord.transform((px + half, pend), &area);
                    let b = coord.transform((nx - half, pend), &area);
                    backend.draw_line(
                        &[a, b],
                        &LineStyle {
                            color: self.connector_color,
                            alpha: self.alpha,
                            width: 0.8,
                            linetype: Linetype::Dashed,
                            ..Default::default()
                        },
                    )?;
                }
            }
            prev = Some((nx, nend));

            let dir = dir_col.map(|c| c[i].clone()).unwrap_or(Value::Na);
            let fill = fill_col
                .and_then(|c| scales.map_color(&Aesthetic::Fill, &c[i]))
                .unwrap_or_else(|| self.direction_color(&dir));
            let delta = delta_col.map(|c| &c[i]).unwrap_or(&y_col[i]);
            let x_txt = super::tip_value(&x_col[i]);
            let tip = if dir.key_str() == WATERFALL_TOTAL {
                format!("{x_txt}: {}", super::tip_value(delta))
            } else {
                let d = delta.as_f64().unwrap_or(0.0);
                let sign = if d >= 0.0 { "+" } else { "" };
                format!(
                    "{x_txt}: {sign}{} (total {})",
                    super::tip_value(delta),
                    super::tip_value(&y_col[i])
                )
            };
            super::set_mark(
                backend,
                Some(tip),
                Some(x_txt),
                super::raw_value(&dir).or_else(|| super::series_key(data, i)),
                super::raw_value(delta),
            );
            super::set_mark_formatted(backend, scales, Some(&x_col[i]), &Aesthetic::Y, Some(delta));
            super::set_row_extras(backend, data, i);

            let (x0, y0) = coord.transform((nx - half, nhi), &area);
            let (x1, y1) = coord.transform((nx + half, nlo), &area);
            backend.draw_rect(
                (x0.min(x1), y0.min(y1)),
                (x0.max(x1), y0.max(y1)),
                &RectStyle {
                    fill: Some(fill),
                    stroke: Some(self.color),
                    stroke_width: 0.5,
                    alpha: self.alpha,
                    clip: true,
                    corner_radius: 0.0,
                },
            )?;
        }
        super::clear_mark(backend);
        super::clear_row_extras(backend);
        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatWaterfall {
            totals: self.totals.clone(),
            final_total: self.final_total.clone(),
        })
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "waterfall"
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
}
