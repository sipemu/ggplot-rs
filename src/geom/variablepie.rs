//! Variable-radius pie: `geom_variable_pie`. Each wedge's angle encodes
//! `y` (its share of the total) and its outer radius encodes `radius`
//! (linearly between `min_radius` and the full radius — Highcharts'
//! `variablepie`). Wedges run clockwise from 12 o'clock in data order;
//! drawn directly in the panel, so the pie stays round.

use std::f64::consts::{FRAC_PI_2, TAU};

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, FontFace, RectStyle, TextAnchor, TextStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Variable-radius pie. Map `y` (angle value), `radius` and `fill`
/// (category); optionally `label`.
pub struct GeomVariablePie {
    /// Inner radius as a fraction of the full radius (0 = pie, > 0 = donut).
    pub inner: f64,
    /// Outer radius of the smallest `radius` value, as a fraction of the full
    /// radius.
    pub min_radius: f64,
    /// Fill when `fill` is unmapped.
    pub fill: (u8, u8, u8),
    pub border: (u8, u8, u8),
    /// Label wedges (the `label`, else `fill` value) outside their rim.
    pub labels: bool,
    pub label_size: f64,
    pub label_color: (u8, u8, u8),
}

impl Default for GeomVariablePie {
    fn default() -> Self {
        GeomVariablePie {
            inner: 0.0,
            min_radius: 0.35,
            fill: (97, 156, 255),
            border: (255, 255, 255),
            labels: true,
            label_size: 10.0,
            label_color: (40, 40, 40),
        }
    }
}

/// Wedge outer radius fraction for `v` in `[lo, hi]`, linear onto
/// `[min_r, 1]` (all equal → 1).
pub(crate) fn radius_fraction(v: f64, lo: f64, hi: f64, min_r: f64) -> f64 {
    if hi > lo {
        min_r + (1.0 - min_r) * ((v - lo) / (hi - lo)).clamp(0.0, 1.0)
    } else {
        1.0
    }
}

impl Geom for GeomVariablePie {
    fn draw(
        &self,
        data: &DataFrame,
        _coord: &dyn Coord,
        scales: &ScaleSet,
        theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let y_col = data
            .column("y")
            .ok_or(RenderError::MissingAesthetic("y".into()))?;
        let r_col = data
            .column("radius")
            .ok_or(RenderError::MissingAesthetic("radius".into()))?;
        let (fill_col, label_col) = (data.column("fill"), data.column("label"));
        let rows: Vec<(usize, f64, f64)> = (0..data.nrows())
            .filter_map(|i| {
                let (y, r) = (y_col[i].as_f64()?, r_col[i].as_f64()?);
                (y.is_finite() && y > 0.0 && r.is_finite()).then_some((i, y, r))
            })
            .collect();
        let total: f64 = rows.iter().map(|r| r.1).sum();
        if total <= 0.0 {
            return Ok(());
        }
        let (lo, hi) = rows
            .iter()
            .fold((f64::MAX, f64::MIN), |(a, b), r| (a.min(r.2), b.max(r.2)));
        let area = backend.plot_area();
        let c = (area.x + area.width / 2.0, area.y + area.height / 2.0);
        let room = if self.labels { 0.8 } else { 0.95 };
        let full = room * area.width.min(area.height) / 2.0;
        let pt = |a: f64, r: f64| (c.0 + r * a.cos(), c.1 + r * a.sin());
        let family = super::resolve_font_family("", theme);
        let mut labels = Vec::new();
        let mut start = -FRAC_PI_2;
        for (i, y, rv) in rows {
            let sweep = TAU * y / total;
            let outer = full * radius_fraction(rv, lo, hi, self.min_radius);
            let inner = full * self.inner.clamp(0.0, 0.95);
            let steps = ((sweep / 0.04).ceil() as usize).max(2);
            let mut poly: Vec<(f64, f64)> = (0..=steps)
                .map(|k| pt(start + sweep * k as f64 / steps as f64, outer))
                .collect();
            if inner > 0.0 {
                poly.extend(
                    (0..=steps)
                        .rev()
                        .map(|k| pt(start + sweep * k as f64 / steps as f64, inner)),
                );
            } else {
                poly.push(c);
            }
            let fill = fill_col
                .and_then(|f| scales.map_color(&Aesthetic::Fill, &f[i]))
                .unwrap_or(self.fill);
            let name = label_col
                .or(fill_col)
                .map(|l| super::tip_value(&l[i]))
                .unwrap_or_default();
            let tip = format!(
                "{name}: {} (radius {})",
                super::tip_value(&y_col[i]),
                super::tip_value(&r_col[i])
            );
            super::set_mark(
                backend,
                Some(tip),
                Some(name.clone()),
                super::series_key(data, i),
                super::raw_value(&y_col[i]),
            );
            super::set_row_extras(backend, data, i);
            backend.draw_polygon(
                &poly,
                &RectStyle {
                    fill: Some(fill),
                    stroke: Some(self.border),
                    stroke_width: 1.0,
                    alpha: 1.0,
                    clip: false,
                    corner_radius: 0.0,
                },
            )?;
            if self.labels && !name.is_empty() {
                let mid = start + sweep / 2.0;
                let p = pt(mid, outer + 8.0);
                let anchor = if mid.cos().abs() < 0.15 {
                    TextAnchor::Middle
                } else if mid.cos() > 0.0 {
                    TextAnchor::Start
                } else {
                    TextAnchor::End
                };
                labels.push((name, p, anchor));
            }
            start += sweep;
        }
        super::clear_mark(backend);
        super::clear_row_extras(backend);
        for (text, pos, anchor) in labels {
            backend.draw_text(
                &text,
                pos,
                &TextStyle {
                    color: self.label_color,
                    size: self.label_size,
                    anchor,
                    angle: 0.0,
                    family: family.clone(),
                    face: FontFace::Plain,
                    bg_colour: None,
                    bg_r: 0.0,
                },
            )?;
        }
        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Y, Aesthetic::Radius]
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
        "variable_pie"
    }
    fn consumes_tooltips(&self) -> bool {
        true
    }
    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let Some(ink) = el.ink {
            self.label_color = ink;
        }
        if let (true, Some(paper)) = (self.border == (255, 255, 255), el.paper) {
            self.border = paper;
        }
    }
    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radius_maps_linearly() {
        assert_eq!(radius_fraction(10.0, 10.0, 20.0, 0.4), 0.4);
        assert_eq!(radius_fraction(20.0, 10.0, 20.0, 0.4), 1.0);
        assert!((radius_fraction(15.0, 10.0, 20.0, 0.4) - 0.7).abs() < 1e-12);
        assert_eq!(radius_fraction(5.0, 5.0, 5.0, 0.4), 1.0);
    }
}
