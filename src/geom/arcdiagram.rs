//! Arc diagram: `geom_arcdiagram`. Nodes sit on a line, links are arcs
//! between them whose thickness is the link weight; node size is the total
//! flow. Layout by [`StatArcdiagram`].

use std::f64::consts::PI;

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, FontFace, PointStyle, RectStyle, TextAnchor, TextStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::arcdiagram::StatArcdiagram;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Arc diagram. Map `from`, `to` and optionally `weight`; nodes and arcs are
/// coloured through the fill scale by node name (arcs by their source).
pub struct GeomArcdiagram {
    /// Order nodes by descending total flow instead of first appearance.
    pub sort_by_value: bool,
    /// Draw right-to-left links below the line (shows direction).
    pub directed_below: bool,
    /// Node radius range in pixels (by √ of total flow).
    pub node_size: (f64, f64),
    /// Arc width range in pixels (by weight).
    pub link_width: (f64, f64),
    /// Colour used when no fill scale applies.
    pub color: (u8, u8, u8),
    pub link_alpha: f64,
    /// Draw node names under the line.
    pub labels: bool,
    pub label_size: f64,
    pub label_color: (u8, u8, u8),
}

impl Default for GeomArcdiagram {
    fn default() -> Self {
        GeomArcdiagram {
            sort_by_value: false,
            directed_below: false,
            node_size: (3.0, 10.0),
            link_width: (1.0, 8.0),
            color: (97, 156, 255),
            link_alpha: 0.5,
            labels: true,
            label_size: 10.0,
            label_color: (40, 40, 40),
        }
    }
}

/// Outline of a polyline stroked `width` pixels wide: the points offset by
/// ±width/2 along each vertex's averaged normal, as one closed polygon.
pub(crate) fn stroke_band(pts: &[(f64, f64)], width: f64) -> Vec<(f64, f64)> {
    let n = pts.len();
    let h = width / 2.0;
    let normal = |i: usize| {
        let (a, b) = (pts[i.saturating_sub(1)], pts[(i + 1).min(n - 1)]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = dx.hypot(dy);
        if len > 0.0 {
            (-dy / len, dx / len)
        } else {
            (0.0, 0.0)
        }
    };
    let mut band: Vec<(f64, f64)> = (0..n)
        .map(|i| {
            let (nx, ny) = normal(i);
            (pts[i].0 + nx * h, pts[i].1 + ny * h)
        })
        .collect();
    band.extend((0..n).rev().map(|i| {
        let (nx, ny) = normal(i);
        (pts[i].0 - nx * h, pts[i].1 - ny * h)
    }));
    band
}

impl Geom for GeomArcdiagram {
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
        let (part, xc, xe) = (col("part")?, col("x")?, col("xend")?);
        let (node, from, to, value) = (col("node")?, col("from")?, col("to")?, col("value")?);
        let fill_col = data.column("fill");
        let area = backend.plot_area();
        let map = |(x, y): (f64, f64)| {
            let sx = scales.get(&Aesthetic::X).map(|s| s.map(&Value::Float(x)));
            let sy = scales.get(&Aesthetic::Y).map(|s| s.map(&Value::Float(y)));
            coord.transform((sx.unwrap_or(0.5), sy.unwrap_or(0.5)), &area)
        };
        let color_of = |i: usize| {
            fill_col
                .and_then(|c| scales.map_color(&Aesthetic::Fill, &c[i]))
                .unwrap_or(self.color)
        };
        let is = |i: usize, p: &str| part[i].key_str() == p;
        let max_of = |p: &str| {
            (0..data.nrows())
                .filter(|&i| is(i, p))
                .filter_map(|i| value[i].as_f64())
                .fold(0.0, f64::max)
        };
        let (max_w, max_v) = (max_of("link"), max_of("node"));
        let lerp = |(lo, hi): (f64, f64), t: f64| lo + (hi - lo) * t.clamp(0.0, 1.0);

        // Widest arcs first so thin ones stay visible on top.
        let mut links: Vec<usize> = (0..data.nrows()).filter(|&i| is(i, "link")).collect();
        links.sort_by(|&a, &b| {
            let w = |i: usize| value[i].as_f64().unwrap_or(0.0);
            w(b).total_cmp(&w(a))
        });
        for i in links {
            let (Some(x0), Some(x1), Some(w)) = (xc[i].as_f64(), xe[i].as_f64(), value[i].as_f64())
            else {
                continue;
            };
            let (c, r) = ((x0 + x1) / 2.0, (x1 - x0).abs() / 2.0);
            let below = self.directed_below && x1 < x0;
            let steps = ((r * 24.0).ceil() as usize).clamp(12, 180);
            let pts: Vec<(f64, f64)> = (0..=steps)
                .map(|k| {
                    let a = PI * k as f64 / steps as f64;
                    let y = r * a.sin();
                    map((c - r * a.cos(), if below { -y } else { y }))
                })
                .collect();
            let (a, b) = (super::tip_value(&from[i]), super::tip_value(&to[i]));
            super::set_mark(
                backend,
                Some(format!("{a} → {b}: {}", super::tip_value(&value[i]))),
                Some(a.clone()),
                Some(a),
                super::raw_value(&value[i]),
            );
            let t = if max_w > 0.0 { w / max_w } else { 1.0 };
            // A filled band (not a wide translucent stroke, whose segment
            // joints double up the alpha on raster backends).
            backend.draw_polygon(
                &stroke_band(&pts, lerp(self.link_width, t)),
                &RectStyle {
                    fill: Some(color_of(i)),
                    stroke: None,
                    stroke_width: 0.0,
                    alpha: self.link_alpha,
                    clip: true,
                    corner_radius: 0.0,
                },
            )?;
        }

        let mut labels = Vec::new();
        for i in (0..data.nrows()).filter(|&i| is(i, "node")) {
            let (Some(x), Some(v)) = (xc[i].as_f64(), value[i].as_f64()) else {
                continue;
            };
            let p = map((x, 0.0));
            let radius = lerp(
                self.node_size,
                if max_v > 0.0 { (v / max_v).sqrt() } else { 1.0 },
            );
            let name = super::tip_value(&node[i]);
            super::set_mark(
                backend,
                Some(format!("{name}: {}", super::tip_value(&value[i]))),
                Some(name.clone()),
                Some(name.clone()),
                super::raw_value(&value[i]),
            );
            backend.draw_circle(
                p,
                radius,
                &PointStyle {
                    color: color_of(i),
                    alpha: 1.0,
                    filled: true,
                    ..Default::default()
                },
            )?;
            if self.labels {
                labels.push((name, (p.0, p.1 + self.node_size.1 + self.label_size * 0.8)));
            }
        }
        super::clear_mark(backend);
        let family = super::resolve_font_family("", theme);
        for (text, pos) in labels {
            backend.draw_text(
                &text,
                pos,
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
        vec![Aesthetic::From, Aesthetic::To]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatArcdiagram {
            sort_by_value: self.sort_by_value,
            directed_below: self.directed_below,
        })
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "arcdiagram"
    }

    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let Some(ink) = el.ink {
            self.label_color = ink;
        }
    }
}
