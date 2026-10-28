//! Venn / Euler diagrams: `geom_venn`. Translucent circles with area ∝
//! set size, overlapping by the pairwise intersections; set names and
//! intersection sizes labelled inside their regions. Layout by [`StatVenn`].

use std::f64::consts::TAU;

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{
    DrawBackend, FontFace, LineStyle, Linetype, RectStyle, TextAnchor, TextStyle,
};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::venn::StatVenn;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Venn diagram. Map `sets` (`"A"`, `"A&B"`, …) and `weight` (size).
pub struct GeomVenn {
    /// Fill when no fill scale applies.
    pub fill: (u8, u8, u8),
    pub alpha: f64,
    pub outline_width: f64,
    pub label_size: f64,
    pub label_color: (u8, u8, u8),
}

impl Default for GeomVenn {
    fn default() -> Self {
        GeomVenn {
            fill: (97, 156, 255),
            alpha: 0.3,
            outline_width: 1.5,
            label_size: 11.0,
            label_color: (30, 30, 30),
        }
    }
}

impl Geom for GeomVenn {
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
        let (part, xc, yc, rc) = (col("part")?, col("x")?, col("y")?, col("r")?);
        let (set, text, size) = (col("set")?, col("text")?, col("size")?);
        let fill_col = data.column("fill");
        let area = backend.plot_area();
        let px = |x: f64, y: f64| {
            let sx = scales.get(&Aesthetic::X).map(|s| s.map(&Value::Float(x)));
            let sy = scales.get(&Aesthetic::Y).map(|s| s.map(&Value::Float(y)));
            coord.transform((sx.unwrap_or(0.5), sy.unwrap_or(0.5)), &area)
        };
        for i in (0..data.nrows()).filter(|&i| part[i].key_str() == "circle") {
            let (Some(x), Some(y), Some(r)) = (xc[i].as_f64(), yc[i].as_f64(), rc[i].as_f64())
            else {
                continue;
            };
            let pts: Vec<(f64, f64)> = (0..=96)
                .map(|k| {
                    let a = TAU * k as f64 / 96.0;
                    px(x + r * a.cos(), y + r * a.sin())
                })
                .collect();
            let fill = fill_col
                .and_then(|f| scales.map_color(&Aesthetic::Fill, &f[i]))
                .unwrap_or(self.fill);
            let name = super::tip_value(&set[i]);
            super::set_mark(
                backend,
                Some(format!("{name}: {}", super::tip_value(&size[i]))),
                Some(name.clone()),
                Some(name),
                super::raw_value(&size[i]),
            );
            backend.draw_polygon(
                &pts,
                &RectStyle {
                    fill: Some(fill),
                    stroke: None,
                    stroke_width: 0.0,
                    alpha: self.alpha,
                    clip: true,
                    corner_radius: 0.0,
                },
            )?;
            backend.draw_line(
                &pts,
                &LineStyle {
                    color: fill,
                    alpha: 1.0,
                    width: self.outline_width,
                    linetype: Linetype::Solid,
                    ..Default::default()
                },
            )?;
        }
        super::clear_mark(backend);
        let family = super::resolve_font_family("", theme);
        for i in (0..data.nrows()).filter(|&i| part[i].key_str() == "label") {
            let (Some(x), Some(y)) = (xc[i].as_f64(), yc[i].as_f64()) else {
                continue;
            };
            backend.draw_text(
                &super::tip_value(&text[i]),
                px(x, y),
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
        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Sets]
    }
    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatVenn)
    }
    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }
    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }
    fn name(&self) -> &str {
        "venn"
    }
    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let Some(ink) = el.ink {
            self.label_color = ink;
        }
    }
}
