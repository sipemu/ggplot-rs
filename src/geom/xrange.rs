//! X-range / Gantt chart: `geom_xrange`. One horizontal bar per row from
//! `xmin` to `xmax` on a discrete `y` (task) axis, with an optional
//! progress fill, milestones (zero-length tasks as diamonds) and dependency
//! arrows from a predecessor's end to a task's start.

use std::collections::HashMap;

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

use super::arrow::{arrow, draw_heads, heads, ArrowType};
use super::{Geom, GeomParams};

/// X-range / Gantt chart. Map `y` (task), `xmin` (start), `xmax` (end) and
/// optionally `fill`; name progress and dependency columns on the geom.
pub struct GeomXrange {
    /// Column with each task's completed fraction (0–1).
    pub progress: Option<String>,
    /// Column naming each task's predecessor (its `y` value); an arrow runs
    /// from the predecessor's end to this task's start.
    pub depends_on: Option<String>,
    /// Bar fill when `fill` is unmapped.
    pub fill: (u8, u8, u8),
    /// Bar height as a fraction of the task slot.
    pub height: f64,
    pub alpha: f64,
    pub arrow_color: (u8, u8, u8),
}

impl Default for GeomXrange {
    fn default() -> Self {
        GeomXrange {
            progress: None,
            depends_on: None,
            fill: (97, 156, 255),
            height: 0.6,
            alpha: 1.0,
            arrow_color: (90, 90, 90),
        }
    }
}

impl GeomXrange {
    /// Shade each bar's completed share from `column` (0–1).
    pub fn with_progress(mut self, column: impl Into<String>) -> Self {
        self.progress = Some(column.into());
        self
    }

    /// Draw dependency arrows from the predecessor named in `column`.
    pub fn with_dependencies(mut self, column: impl Into<String>) -> Self {
        self.depends_on = Some(column.into());
        self
    }
}

/// A pixel position.
type Px = (f64, f64);

fn darken(c: (u8, u8, u8), f: f64) -> (u8, u8, u8) {
    let d = |v: u8| (v as f64 * f) as u8;
    (d(c.0), d(c.1), d(c.2))
}

impl Geom for GeomXrange {
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
        let (y_col, x0c, x1c) = (col("y")?, col("xmin")?, col("xmax")?);
        let fill_col = data.column("fill");
        let progress = self.progress.as_deref().and_then(|c| data.column(c));
        let deps = self.depends_on.as_deref().and_then(|c| data.column(c));
        let area = backend.plot_area();
        let y_scale = scales.get(&Aesthetic::Y);
        let mx = |v: &Value| scales.get(&Aesthetic::X).map(|s| s.map(v)).unwrap_or(0.0);
        let my = |v: &Value| y_scale.map(|s| s.map(v)).unwrap_or(0.5);
        let n_levels = y_scale.map(|s| s.breaks().len()).unwrap_or(1).max(1) as f64;
        let half = self.height / (n_levels * if coord.discrete_expand() { 1.1 } else { 1.0 }) / 2.0;

        // Pixel anchors per task for dependency arrows: (start, end, centre y).
        let mut anchors: HashMap<String, (Px, Px)> = HashMap::new();
        for i in 0..data.nrows() {
            let (ny, nx0, nx1) = (my(&y_col[i]), mx(&x0c[i]), mx(&x1c[i]));
            if ![ny, nx0, nx1].iter().all(|v| v.is_finite()) {
                continue;
            }
            let task = super::tip_value(&y_col[i]);
            let fill = fill_col
                .and_then(|c| scales.map_color(&Aesthetic::Fill, &c[i]))
                .unwrap_or(self.fill);
            let p = progress
                .and_then(|c| c[i].as_f64())
                .map(|p| p.clamp(0.0, 1.0));
            let (s, e) = (super::tip_value(&x0c[i]), super::tip_value(&x1c[i]));
            let milestone = (nx1 - nx0).abs() < 1e-12;
            let tip = match (milestone, p) {
                (true, _) => format!("{task}: {s}"),
                (false, Some(p)) => format!("{task}: {s} – {e} ({}% done)", (p * 100.0).round()),
                (false, None) => format!("{task}: {s} – {e}"),
            };
            super::set_mark(
                backend,
                Some(tip),
                Some(task.clone()),
                super::series_key(data, i),
                None,
            );
            super::set_row_extras(backend, data, i);
            let style = |fill| RectStyle {
                fill: Some(fill),
                stroke: None,
                stroke_width: 0.0,
                alpha: self.alpha,
                clip: true,
                corner_radius: 3.0,
            };
            let start = coord.transform((nx0, ny), &area);
            let end = coord.transform((nx1, ny), &area);
            if milestone {
                let r =
                    (0.75 * (coord.transform((nx0, ny + half), &area).1 - start.1).abs()).max(4.0);
                backend.draw_polygon(
                    &[
                        (start.0, start.1 - r),
                        (start.0 + r, start.1),
                        (start.0, start.1 + r),
                        (start.0 - r, start.1),
                    ],
                    &style(darken(fill, 0.8)),
                )?;
                anchors.insert(task, ((start.0 - r, start.1), (start.0 + r, start.1)));
                continue;
            }
            let a = coord.transform((nx0, ny + half), &area);
            let b = coord.transform((nx1, ny - half), &area);
            let (l, t, r, btm) = (a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1));
            backend.draw_rect((l, t), (r, btm), &style(fill))?;
            if let Some(p) = p {
                super::clear_mark(backend);
                let w = (r - l) * p;
                if w > 0.0 {
                    let inset = (btm - t) * 0.25;
                    backend.draw_rect(
                        (l, t + inset),
                        (l + w, btm - inset),
                        &style(darken(fill, 0.6)),
                    )?;
                }
            }
            anchors.insert(task, (start, end));
        }
        super::clear_row_extras(backend);

        if let Some(deps) = deps {
            let head = arrow().kind(ArrowType::Closed).length(6.0);
            let line = LineStyle {
                color: self.arrow_color,
                alpha: 1.0,
                width: 1.0,
                linetype: Linetype::Solid,
                ..Default::default()
            };
            for i in 0..data.nrows() {
                if deps[i].is_na() {
                    continue;
                }
                let task = super::tip_value(&y_col[i]);
                let pred = super::tip_value(&deps[i]);
                let (Some(&(_, from)), Some(&(to, _))) = (anchors.get(&pred), anchors.get(&task))
                else {
                    continue;
                };
                // Elbow: out of the predecessor's end, down/up to the task's
                // row, into its start.
                let bend = from.0 + 8.0;
                let path = if bend <= to.0 - 6.0 {
                    vec![from, (bend, from.1), (bend, to.1), to]
                } else {
                    let mid = (from.1 + to.1) / 2.0;
                    vec![
                        from,
                        (bend, from.1),
                        (bend, mid),
                        (to.0 - 8.0, mid),
                        (to.0 - 8.0, to.1),
                        to,
                    ]
                };
                super::set_mark(backend, Some(format!("{pred} → {task}")), None, None, None);
                backend.draw_line(&path, &line)?;
                let hs = heads(&path, &head, 1.0);
                draw_heads(backend, &hs, &head, self.arrow_color, 1.0, 1.0)?;
                backend.set_mark_part(None);
            }
        }
        super::clear_mark(backend);
        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Y, Aesthetic::Xmin, Aesthetic::Xmax]
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
        "xrange"
    }
    fn consumes_tooltips(&self) -> bool {
        true
    }
    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }
}
