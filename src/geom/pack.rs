//! Packed bubble chart: `geom_packed_bubble`. Circles with area ∝ `weight`
//! packed without overlap, optionally clustered by `subgroup` (each cluster
//! outlined). Layout by [`StatPack`].

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, FontFace, PointStyle, TextAnchor, TextStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::pack::StatPack;
use crate::stat::Stat;
use crate::theme::Theme;

use super::treemap::{contrast_text, fit_size};
use super::{Geom, GeomParams};

/// Packed bubble chart. Map `weight` (area), optionally `subgroup`
/// (clusters), `fill` and `label`.
pub struct GeomPackedBubble {
    pub sort: bool,
    /// Gap between bubbles as a fraction of the mean radius.
    pub padding: f64,
    /// Fill when `fill` is unmapped.
    pub fill: (u8, u8, u8),
    pub alpha: f64,
    /// Cluster outline colour (`None` hides it).
    pub group_outline: Option<(u8, u8, u8)>,
    pub label_size: f64,
    pub min_label_size: f64,
}

impl Default for GeomPackedBubble {
    fn default() -> Self {
        GeomPackedBubble {
            sort: true,
            padding: 0.08,
            fill: (97, 156, 255),
            alpha: 0.9,
            group_outline: Some((170, 170, 170)),
            label_size: 10.0,
            min_label_size: 6.0,
        }
    }
}

impl Geom for GeomPackedBubble {
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
        let (fill_col, label_col, weight_col) = (
            data.column("fill"),
            data.column("label"),
            data.column("weight"),
        );
        let area = backend.plot_area();
        let px = |x: f64, y: f64| {
            let sx = scales.get(&Aesthetic::X).map(|s| s.map(&Value::Float(x)));
            let sy = scales.get(&Aesthetic::Y).map(|s| s.map(&Value::Float(y)));
            coord.transform((sx.unwrap_or(0.5), sy.unwrap_or(0.5)), &area)
        };
        let radius_px = |x: f64, y: f64, r: f64| {
            let c = px(x, y);
            let (a, b) = (px(x + r, y), px(x, y + r));
            (a.0 - c.0)
                .hypot(a.1 - c.1)
                .min((b.0 - c.0).hypot(b.1 - c.1))
        };
        let family = super::resolve_font_family("", theme);
        let mut labels = Vec::new();

        let mut order: Vec<usize> = (0..data.nrows()).collect();
        // Cluster outlines under the bubbles.
        order.sort_by_key(|&i| part[i].key_str() != "group");
        for i in order {
            let (Some(x), Some(y), Some(r)) = (xc[i].as_f64(), yc[i].as_f64(), rc[i].as_f64())
            else {
                continue;
            };
            let c = px(x, y);
            let rp = radius_px(x, y, r);
            if part[i].key_str() == "group" {
                if let Some(outline) = self.group_outline {
                    super::clear_mark(backend);
                    backend.draw_circle(
                        c,
                        rp,
                        &PointStyle {
                            color: outline,
                            alpha: 1.0,
                            filled: false,
                            ..Default::default()
                        },
                    )?;
                }
                continue;
            }
            let fill = fill_col
                .and_then(|f| scales.map_color(&Aesthetic::Fill, &f[i]))
                .unwrap_or(self.fill);
            let label = label_col
                .map(|l| super::tip_value(&l[i]))
                .filter(|s| !s.is_empty());
            let value = weight_col.map(|w| super::tip_value(&w[i]));
            let tip = match (&label, &value) {
                (Some(l), Some(v)) => format!("{l}: {v}"),
                (Some(l), None) => l.clone(),
                (None, Some(v)) => v.clone(),
                (None, None) => String::new(),
            };
            super::set_mark(
                backend,
                (!tip.is_empty()).then_some(tip),
                label.clone(),
                super::series_key(data, i),
                weight_col.and_then(|w| super::raw_value(&w[i])),
            );
            super::set_row_extras(backend, data, i);
            backend.draw_circle(
                c,
                rp,
                &PointStyle {
                    color: fill,
                    alpha: self.alpha,
                    filled: true,
                    ..Default::default()
                },
            )?;
            if let Some(l) = label {
                let side = rp * std::f64::consts::SQRT_2;
                if let Some(size) = fit_size(&l, side, side, self.label_size, self.min_label_size) {
                    labels.push((l, c, size, contrast_text(fill)));
                }
            }
        }
        super::clear_mark(backend);
        super::clear_row_extras(backend);
        for (text, pos, size, color) in labels {
            backend.draw_text(
                &text,
                pos,
                &TextStyle {
                    color,
                    size,
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
        Vec::new()
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatPack {
            sort: self.sort,
            padding: self.padding,
        })
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "packed_bubble"
    }

    fn consumes_tooltips(&self) -> bool {
        true
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }
}
