//! Event flags on time series: `geom_flags`. Each row (`x`, `label`) is a
//! small flag on a stem, pinned either to the x axis or — with
//! [`GeomFlags::on_series`] — to a series' value at that x (linearly
//! interpolated). Flags at the same x stack upward instead of overlapping.

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

/// Flag glyph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FlagShape {
    /// A pennant box holding the label.
    #[default]
    Flag,
    /// A circle holding the label's first character (full label on hover).
    CirclePin,
}

/// Event flags. Map `x` and `label`, optionally `color`.
pub struct GeomFlags {
    pub shape: FlagShape,
    /// Series the flags sit on: `(x, y)` points, interpolated at each flag.
    pub series: Option<Vec<(f64, f64)>>,
    pub color: (u8, u8, u8),
    pub fill: (u8, u8, u8),
    pub label_size: f64,
    /// Stem length in pixels (each stacked flag adds a box height).
    pub stem: f64,
}

impl Default for GeomFlags {
    fn default() -> Self {
        GeomFlags {
            shape: FlagShape::Flag,
            series: None,
            color: (31, 119, 180),
            fill: (255, 255, 255),
            label_size: 9.0,
            stem: 18.0,
        }
    }
}

impl GeomFlags {
    /// Pin flags to the series given by `x` and `y` values (numbers or
    /// date-times; rows with a missing value are skipped).
    pub fn on_series(mut self, x: &[Value], y: &[Value]) -> Self {
        let mut pts: Vec<(f64, f64)> = x
            .iter()
            .zip(y)
            .filter_map(|(a, b)| Some((a.as_f64()?, b.as_f64()?)))
            .collect();
        pts.sort_by(|a, b| a.0.total_cmp(&b.0));
        self.series = Some(pts);
        self
    }
}

/// Linear interpolation of `pts` (sorted by x) at `x`, clamped at the ends.
pub(crate) fn interpolate_at(pts: &[(f64, f64)], x: f64) -> Option<f64> {
    let (first, last) = (pts.first()?, pts.last()?);
    if x <= first.0 {
        return Some(first.1);
    }
    if x >= last.0 {
        return Some(last.1);
    }
    let k = pts.partition_point(|p| p.0 <= x);
    let (a, b) = (pts[k - 1], pts[k]);
    Some(if b.0 > a.0 {
        a.1 + (b.1 - a.1) * (x - a.0) / (b.0 - a.0)
    } else {
        a.1
    })
}

impl Geom for GeomFlags {
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
        let (label_col, color_col) = (data.column("label"), data.column("color"));
        let area = backend.plot_area();
        let mx = |v: &Value| scales.get(&Aesthetic::X).map(|s| s.map(v)).unwrap_or(0.5);
        let my = |v: f64| {
            scales
                .get(&Aesthetic::Y)
                .map(|s| s.map(&Value::Float(v)))
                .unwrap_or(0.0)
        };
        let family = super::resolve_font_family("", theme);
        let box_h = self.label_size + 7.0;
        let mut order: Vec<usize> = (0..data.nrows()).collect();
        order.sort_by(|&a, &b| mx(&x_col[a]).total_cmp(&mx(&x_col[b])));
        // Anchors already used: (pixel x, flags stacked there).
        let mut stacks: Vec<(f64, usize)> = Vec::new();
        for i in order {
            let nx = mx(&x_col[i]);
            let ny = match (&self.series, x_col[i].as_f64()) {
                (Some(pts), Some(x)) => interpolate_at(pts, x).map(my).unwrap_or(0.0),
                _ => 0.0,
            };
            let anchor = coord.transform((nx, ny), &area);
            if !(anchor.0.is_finite() && anchor.1.is_finite()) {
                continue;
            }
            let level = match stacks.iter_mut().find(|s| (s.0 - anchor.0).abs() < 3.0) {
                Some(s) => {
                    s.1 += 1;
                    s.1 - 1
                }
                None => {
                    stacks.push((anchor.0, 1));
                    0
                }
            };
            let color = color_col
                .and_then(|c| scales.map_color(&Aesthetic::Color, &c[i]))
                .unwrap_or(self.color);
            let label = label_col
                .map(|c| super::tip_value(&c[i]))
                .unwrap_or_default();
            let top = (
                anchor.0,
                anchor.1 - self.stem - level as f64 * (box_h + 3.0),
            );
            super::set_mark(
                backend,
                Some(format!("{}: {label}", super::tip_value(&x_col[i]))),
                Some(super::tip_value(&x_col[i])),
                super::series_key(data, i),
                None,
            );
            super::set_row_extras(backend, data, i);
            backend.draw_line(
                &[anchor, top],
                &LineStyle {
                    color,
                    alpha: 1.0,
                    width: 1.0,
                    linetype: Linetype::Solid,
                    ..Default::default()
                },
            )?;
            let text = match self.shape {
                FlagShape::Flag => label.clone(),
                FlagShape::CirclePin => label.chars().next().map(String::from).unwrap_or_default(),
            };
            let center = match self.shape {
                FlagShape::Flag => {
                    let w = crate::render::text::text_width(&text, self.label_size) + 8.0;
                    backend.draw_rect(
                        (top.0, top.1 - box_h),
                        (top.0 + w, top.1),
                        &RectStyle {
                            fill: Some(self.fill),
                            stroke: Some(color),
                            stroke_width: 1.0,
                            alpha: 1.0,
                            clip: false,
                            corner_radius: 0.0,
                        },
                    )?;
                    (top.0 + w / 2.0, top.1 - box_h / 2.0)
                }
                FlagShape::CirclePin => {
                    let c = (top.0, top.1 - box_h / 2.0);
                    backend.draw_circle(
                        c,
                        box_h / 2.0,
                        &PointStyle {
                            color,
                            alpha: 1.0,
                            filled: true,
                            ..Default::default()
                        },
                    )?;
                    c
                }
            };
            if !text.is_empty() {
                super::clear_mark(backend);
                backend.draw_text(
                    &text,
                    center,
                    &TextStyle {
                        color: if self.shape == FlagShape::CirclePin {
                            super::treemap::contrast_text(color)
                        } else {
                            color
                        },
                        size: self.label_size,
                        anchor: TextAnchor::Middle,
                        angle: 0.0,
                        family: family.clone(),
                        face: FontFace::Bold,
                        bg_colour: None,
                        bg_r: 0.0,
                    },
                )?;
            }
        }
        super::clear_mark(backend);
        super::clear_row_extras(backend);
        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X]
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
        "flags"
    }
    fn consumes_tooltips(&self) -> bool {
        true
    }
    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.color = color;
    }
    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let (true, Some(paper)) = (self.fill == (255, 255, 255), el.paper) {
            self.fill = paper;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolation_and_clamping() {
        let p = [(0.0, 0.0), (10.0, 100.0)];
        assert_eq!(interpolate_at(&p, 5.0), Some(50.0));
        assert_eq!(interpolate_at(&p, -3.0), Some(0.0));
        assert_eq!(interpolate_at(&p, 30.0), Some(100.0));
        assert_eq!(interpolate_at(&[], 1.0), None);
    }
}
