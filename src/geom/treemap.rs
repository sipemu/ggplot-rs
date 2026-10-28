//! Treemap: `geom_treemap`. Rectangles tile the panel with area ∝ `weight`;
//! with a `subgroup` mapping, tiles nest inside group rectangles drawn with
//! a heavier border and a group label. Layout by [`StatTreemap`].

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, FontFace, RectStyle, TextAnchor, TextStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::treemap::StatTreemap;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Treemap. Map `weight` (tile area), optionally `subgroup` (hierarchy),
/// `fill` and `label`.
pub struct GeomTreemap {
    /// Layout width / height; the builder sets the panel aspect to match so
    /// tiles come out square.
    pub aspect: f64,
    /// Inset of a group's tiles inside its rectangle, in layout units
    /// (the layout is 1 unit tall).
    pub group_padding: f64,
    /// Fill when `fill` is unmapped.
    pub fill: (u8, u8, u8),
    /// Tile border colour and width.
    pub border: (u8, u8, u8),
    pub border_width: f64,
    /// Group border width.
    pub group_border_width: f64,
    pub alpha: f64,
    /// Tile label size (shrunk to fit, hidden below `min_label_size`).
    pub label_size: f64,
    pub min_label_size: f64,
    /// Group label size (0 hides group labels).
    pub group_label_size: f64,
}

impl Default for GeomTreemap {
    fn default() -> Self {
        GeomTreemap {
            aspect: 1.6,
            group_padding: 0.006,
            fill: (97, 156, 255),
            border: (255, 255, 255),
            border_width: 1.0,
            group_border_width: 3.0,
            alpha: 1.0,
            label_size: 11.0,
            min_label_size: 6.0,
            group_label_size: 15.0,
        }
    }
}

/// Black or white, whichever reads better on `bg` (WCAG relative luminance).
pub(crate) fn contrast_text(bg: (u8, u8, u8)) -> (u8, u8, u8) {
    let lin = |c: u8| {
        let c = c as f64 / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let l = 0.2126 * lin(bg.0) + 0.7152 * lin(bg.1) + 0.0722 * lin(bg.2);
    if l > 0.4 {
        (30, 30, 30)
    } else {
        (255, 255, 255)
    }
}

/// Largest font size ≤ `max` at which `text` fits a `w × h` pixel box
/// (approximate glyph width 0.68 em, erring wide), or `None` below `min`.
pub(crate) fn fit_size(text: &str, w: f64, h: f64, max: f64, min: f64) -> Option<f64> {
    let chars = text.chars().count().max(1) as f64;
    let size = max.min((w - 6.0) / (chars * 0.68)).min(h - 4.0);
    (size >= min).then_some(size)
}

impl Geom for GeomTreemap {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let col = |n: &str| {
            data.column(n)
                .ok_or_else(|| RenderError::MissingAesthetic(n.into()))
        };
        let (part, x0c, x1c, y0c, y1c, area_c) = (
            col("part")?,
            col("xmin")?,
            col("xmax")?,
            col("ymin")?,
            col("ymax")?,
            col("area")?,
        );
        let fill_col = data.column("fill");
        let label_col = data.column("label");
        let sub_col = data.column("subgroup");
        let area = backend.plot_area();
        let px = |i: usize| {
            let m = |aes: Aesthetic, v: &Value| scales.get(&aes).map(|s| s.map(v)).unwrap_or(0.0);
            let (a, b) = (
                coord.transform((m(Aesthetic::X, &x0c[i]), m(Aesthetic::Y, &y1c[i])), &area),
                coord.transform((m(Aesthetic::X, &x1c[i]), m(Aesthetic::Y, &y0c[i])), &area),
            );
            ((a.0.min(b.0), a.1.min(b.1)), (a.0.max(b.0), a.1.max(b.1)))
        };
        let family = super::resolve_font_family("", theme);
        let mut tile_labels = Vec::new();
        for i in (0..data.nrows()).filter(|&i| part[i].key_str() == "tile") {
            let ((l, t), (r, b)) = px(i);
            let fill = fill_col
                .and_then(|c| scales.map_color(&Aesthetic::Fill, &c[i]))
                .unwrap_or(self.fill);
            let label = label_col
                .map(|c| super::tip_value(&c[i]))
                .filter(|s| !s.is_empty());
            let name = label
                .clone()
                .or_else(|| sub_col.map(|c| super::tip_value(&c[i])))
                .unwrap_or_default();
            let tip = if name.is_empty() {
                super::tip_value(&area_c[i])
            } else {
                format!("{name}: {}", super::tip_value(&area_c[i]))
            };
            super::set_mark(
                backend,
                Some(tip),
                label.clone(),
                super::series_key(data, i),
                super::raw_value(&area_c[i]),
            );
            super::set_row_extras(backend, data, i);
            backend.draw_rect(
                (l, t),
                (r, b),
                &RectStyle {
                    fill: Some(fill),
                    stroke: Some(self.border),
                    stroke_width: self.border_width,
                    alpha: self.alpha,
                    clip: true,
                    corner_radius: 0.0,
                },
            )?;
            if let Some(label) = label {
                if let Some(size) =
                    fit_size(&label, r - l, b - t, self.label_size, self.min_label_size)
                {
                    let pos = (l + 4.0, t + 3.0 + size / 2.0);
                    tile_labels.push((label, pos, size, contrast_text(fill)));
                }
            }
        }
        super::clear_row_extras(backend);
        super::clear_mark(backend);

        let groups: Vec<usize> = (0..data.nrows())
            .filter(|&i| part[i].key_str() == "group")
            .collect();
        for &i in &groups {
            let ((l, t), (r, b)) = px(i);
            backend.draw_rect(
                (l, t),
                (r, b),
                &RectStyle {
                    fill: None,
                    stroke: Some(self.border),
                    stroke_width: self.group_border_width,
                    alpha: 1.0,
                    clip: true,
                    corner_radius: 0.0,
                },
            )?;
        }
        for (label, pos, size, color) in tile_labels {
            backend.draw_text(
                &label,
                pos,
                &TextStyle {
                    color,
                    size,
                    anchor: TextAnchor::Start,
                    angle: 0.0,
                    family: family.clone(),
                    face: FontFace::Plain,
                    bg_colour: None,
                    bg_r: 0.0,
                },
            )?;
        }
        if self.group_label_size > 0.0 {
            for &i in &groups {
                let Some(name) = sub_col.map(|c| super::tip_value(&c[i])) else {
                    continue;
                };
                let ((l, t), (r, b)) = px(i);
                if let Some(size) = fit_size(&name, r - l, b - t, self.group_label_size, 8.0) {
                    let center = ((l + r) / 2.0, (t + b) / 2.0);
                    backend.draw_text(
                        &name,
                        center,
                        &TextStyle {
                            color: (255, 255, 255),
                            size,
                            anchor: TextAnchor::Middle,
                            angle: 0.0,
                            family: family.clone(),
                            face: FontFace::Bold,
                            bg_colour: Some((40, 40, 40)),
                            bg_r: 2.0,
                        },
                    )?;
                }
            }
        }
        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Weight]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatTreemap {
            aspect: self.aspect,
            group_padding: self.group_padding,
        })
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "treemap"
    }

    fn consumes_tooltips(&self) -> bool {
        true
    }

    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
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
    fn label_fitting_and_contrast() {
        assert_eq!(fit_size("abc", 200.0, 50.0, 11.0, 6.0), Some(11.0));
        assert!(fit_size("a long label", 30.0, 50.0, 11.0, 6.0).is_none());
        assert_eq!(contrast_text((255, 255, 200)), (30, 30, 30));
        assert_eq!(contrast_text((20, 40, 90)), (255, 255, 255));
    }
}
