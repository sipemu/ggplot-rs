//! Timeline: `geom_timeline`. Events along an x (usually date) axis on a
//! central baseline; each is a dot with a stem to a boxed label. Labels are
//! stacked on alternating levels above and below the line so they never
//! overlap (greedy, at draw time in pixels); a span-end column turns an
//! event into a span drawn as a band on the baseline.

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{
    DrawBackend, FontFace, LineStyle, Linetype, PointStyle, RectStyle, TextAnchor, TextStyle,
};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Timeline. Map `x` (event time), `label` and optionally `color`; name a
/// span-end column in `end` (rows where it is missing are point events —
/// unlike a mapped `xend`, a missing end doesn't drop the row).
pub struct GeomTimeline {
    /// Column holding each event's span end (`None`: point events only).
    pub end: Option<String>,
    pub color: (u8, u8, u8),
    pub line_color: (u8, u8, u8),
    pub label_size: f64,
    pub label_color: (u8, u8, u8),
    /// Label box fill.
    pub box_fill: (u8, u8, u8),
    /// Gap kept between neighbouring labels on one level, px.
    pub spacing: f64,
}

impl Default for GeomTimeline {
    fn default() -> Self {
        GeomTimeline {
            end: None,
            color: (31, 119, 180),
            line_color: (120, 120, 120),
            label_size: 9.5,
            label_color: (30, 30, 30),
            box_fill: (255, 255, 255),
            spacing: 6.0,
        }
    }
}

impl GeomTimeline {
    /// Read span ends from `column`.
    pub fn with_end(mut self, column: impl Into<String>) -> Self {
        self.end = Some(column.into());
        self
    }
}

/// Level order: 1, −1, 2, −2, … (above, below, further above, …).
fn level(k: usize) -> i32 {
    let n = (k / 2 + 1) as i32;
    if k.is_multiple_of(2) {
        n
    } else {
        -n
    }
}

/// Greedy level for a label spanning `[l, r]` given each level's occupied
/// intervals; `None` when every level up to `max_levels` per side collides.
pub(crate) fn pick_level(
    occupied: &mut Vec<(i32, f64, f64)>,
    l: f64,
    r: f64,
    max_levels: usize,
) -> Option<i32> {
    (0..2 * max_levels.max(1)).map(level).find(|&lv| {
        let free = !occupied.iter().any(|&(o, a, b)| o == lv && l < b && a < r);
        if free {
            occupied.push((lv, l, r));
        }
        free
    })
}

impl Geom for GeomTimeline {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let x_col = data
            .column("x")
            .ok_or(RenderError::MissingAesthetic("x".into()))?;
        let label = data.column("label");
        let xend = data.column("xmax");
        let color_col = data.column("color");
        let area = backend.plot_area();
        let mx = |v: &Value| scales.get(&Aesthetic::X).map(|s| s.map(v)).unwrap_or(0.5);
        let px = |nx: f64| coord.transform((nx, 0.5), &area).0;
        let base = area.y + area.height / 2.0;
        backend.draw_line(
            &[(area.x, base), (area.x + area.width, base)],
            &LineStyle {
                color: self.line_color,
                alpha: 1.0,
                width: 1.5,
                linetype: Linetype::Solid,
                ..Default::default()
            },
        )?;
        let box_h = self.label_size + 8.0;
        let step = box_h + 6.0;
        let max_levels = (((area.height / 2.0) - 12.0) / step).floor().max(1.0) as usize;
        let mut order: Vec<usize> = (0..data.nrows()).collect();
        order.sort_by(|&a, &b| mx(&x_col[a]).total_cmp(&mx(&x_col[b])));
        let mut occupied = Vec::new();
        let family = super::resolve_font_family("", theme);
        for i in order {
            let x = px(mx(&x_col[i]));
            if !x.is_finite() {
                continue;
            }
            let color = color_col
                .and_then(|c| scales.map_color(&Aesthetic::Color, &c[i]))
                .unwrap_or(self.color);
            let text = label.map(|c| super::tip_value(&c[i])).unwrap_or_default();
            let when = super::tip_value(&x_col[i]);
            let tip = match xend.map(|c| &c[i]).filter(|v| !v.is_na()) {
                Some(e) => format!("{when} – {}: {text}", super::tip_value(e)),
                None => format!("{when}: {text}"),
            };
            super::set_mark(
                backend,
                Some(tip),
                Some(when),
                super::series_key(data, i),
                None,
            );
            super::set_row_extras(backend, data, i);
            // Span band on the baseline.
            if let Some(xe) = xend.and_then(|c| (!c[i].is_na()).then(|| px(mx(&c[i])))) {
                if xe.is_finite() {
                    backend.draw_rect(
                        (x.min(xe), base - 4.0),
                        (x.max(xe), base + 4.0),
                        &RectStyle {
                            fill: Some(color),
                            stroke: None,
                            stroke_width: 0.0,
                            alpha: 0.5,
                            clip: true,
                            corner_radius: 4.0,
                        },
                    )?;
                }
            }
            backend.draw_circle(
                (x, base),
                4.5,
                &PointStyle {
                    color,
                    alpha: 1.0,
                    filled: true,
                    ..Default::default()
                },
            )?;
            if text.is_empty() {
                continue;
            }
            let w = crate::render::text::text_width(&text, self.label_size) + 10.0;
            // Keep the box inside the panel horizontally.
            let cx = x.clamp(area.x + w / 2.0, area.x + area.width - w / 2.0);
            let Some(lv) = pick_level(
                &mut occupied,
                cx - w / 2.0 - self.spacing / 2.0,
                cx + w / 2.0 + self.spacing / 2.0,
                max_levels,
            ) else {
                continue;
            };
            let cy = base - lv as f64 * step;
            let edge = if lv > 0 {
                cy + box_h / 2.0
            } else {
                cy - box_h / 2.0
            };
            super::clear_mark(backend);
            backend.draw_line(
                &[(x, base), (x, edge)],
                &LineStyle {
                    color,
                    alpha: 0.8,
                    width: 1.0,
                    linetype: Linetype::Solid,
                    ..Default::default()
                },
            )?;
            backend.draw_rect(
                (cx - w / 2.0, cy - box_h / 2.0),
                (cx + w / 2.0, cy + box_h / 2.0),
                &RectStyle {
                    fill: Some(self.box_fill),
                    stroke: Some(color),
                    stroke_width: 1.0,
                    alpha: 1.0,
                    clip: false,
                    corner_radius: 3.0,
                },
            )?;
            backend.draw_text(
                &text,
                (cx, cy),
                &TextStyle {
                    color: self.label_color,
                    size: self.label_size,
                    anchor: TextAnchor::Middle,
                    angle: 0.0,
                    family: family.clone(),
                    face: FontFace::Plain,
                    bg_colour: None,
                    bg_r: 0.0,
                },
            )?;
        }
        super::clear_mark(backend);
        super::clear_row_extras(backend);
        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X]
    }

    /// Span ends become `xmax` (after missing-position rows were dropped, so
    /// a point event's empty end keeps its row) and train the x scale.
    fn setup_data(&self, data: &mut DataFrame, _warnings: &mut Vec<String>) {
        // A (hidden) y scale must exist for the panel's axes to be drawn.
        let n = data.nrows();
        for (c, v) in [("ymin", -1.0), ("ymax", 1.0)] {
            if !data.has_column(c) {
                data.add_column(c.into(), vec![Value::Float(v); n]);
            }
        }
        if let Some(end) = self.end.as_deref().and_then(|c| data.column(c)) {
            let end = end.to_vec();
            if !data.has_column("xmax") {
                data.add_column("xmax".into(), end);
            }
        }
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
        "timeline"
    }
    fn consumes_tooltips(&self) -> bool {
        true
    }
    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let Some(ink) = el.ink {
            self.label_color = ink;
        }
        if let (true, Some(paper)) = (self.box_fill == (255, 255, 255), el.paper) {
            self.box_fill = paper;
        }
    }
    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.color = color;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_alternate_and_avoid_overlap() {
        assert_eq!((0..4).map(level).collect::<Vec<_>>(), vec![1, -1, 2, -2]);
        let mut occ = Vec::new();
        assert_eq!(pick_level(&mut occ, 0.0, 10.0, 3), Some(1));
        assert_eq!(pick_level(&mut occ, 5.0, 15.0, 3), Some(-1));
        assert_eq!(pick_level(&mut occ, 8.0, 12.0, 3), Some(2));
        assert_eq!(pick_level(&mut occ, 20.0, 30.0, 3), Some(1), "free again");
        assert_eq!(
            pick_level(&mut occ, 0.0, 30.0, 1),
            None,
            "only ±1 allowed, both busy"
        );
    }
}
