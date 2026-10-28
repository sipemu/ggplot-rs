//! Network graph: `geom_network`. Nodes from an edge list laid out by
//! [`StatNetwork`] (force-directed by default); edges as lines whose width
//! follows the weight (with arrowheads when directed), nodes sized by
//! degree, optional labels.

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{
    DrawBackend, FontFace, LineStyle, Linetype, PointStyle, TextAnchor, TextStyle,
};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::network::{NetworkLayout, StatNetwork};
use crate::stat::Stat;
use crate::theme::Theme;

use super::arrow::{arrow, draw_heads, heads, ArrowType};
use super::{Geom, GeomParams};

/// Network graph. Map `from`, `to` and optionally `weight`.
pub struct GeomNetwork {
    pub layout: NetworkLayout,
    /// Draw arrowheads at the target end of each edge.
    pub directed: bool,
    /// Colour nodes by connected component (through the fill scale).
    pub color_components: bool,
    /// Node fill when not coloured by component.
    pub fill: (u8, u8, u8),
    /// Node radius range in pixels (by √degree).
    pub node_size: (f64, f64),
    pub edge_color: (u8, u8, u8),
    /// Edge width range in pixels (by weight).
    pub edge_width: (f64, f64),
    pub edge_alpha: f64,
    pub labels: bool,
    pub label_size: f64,
    pub label_color: (u8, u8, u8),
}

impl Default for GeomNetwork {
    fn default() -> Self {
        GeomNetwork {
            layout: NetworkLayout::default(),
            directed: false,
            color_components: false,
            fill: (31, 119, 180),
            node_size: (4.0, 11.0),
            edge_color: (150, 150, 150),
            edge_width: (0.8, 4.0),
            edge_alpha: 0.7,
            labels: true,
            label_size: 9.0,
            label_color: (40, 40, 40),
        }
    }
}

impl Geom for GeomNetwork {
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
        let (part, xc, yc, xe, ye) = (
            col("part")?,
            col("x")?,
            col("y")?,
            col("xend")?,
            col("yend")?,
        );
        let (node, from, to, value, degree) = (
            col("node")?,
            col("from")?,
            col("to")?,
            col("value")?,
            col("degree")?,
        );
        let fill_col = data.column("fill");
        let area = backend.plot_area();
        let px = |x: f64, y: f64| {
            let sx = scales.get(&Aesthetic::X).map(|s| s.map(&Value::Float(x)));
            let sy = scales.get(&Aesthetic::Y).map(|s| s.map(&Value::Float(y)));
            coord.transform((sx.unwrap_or(0.5), sy.unwrap_or(0.5)), &area)
        };
        let is = |i: usize, p: &str| part[i].key_str() == p;
        let max_of = |p: &str, c: &[Value]| {
            (0..data.nrows())
                .filter(|&i| is(i, p))
                .filter_map(|i| c[i].as_f64())
                .fold(0.0, f64::max)
        };
        let (max_w, max_d) = (max_of("edge", value), max_of("node", degree));
        let lerp = |(lo, hi): (f64, f64), t: f64| lo + (hi - lo) * t.clamp(0.0, 1.0);
        let radius = |d: f64| {
            lerp(
                self.node_size,
                if max_d > 0.0 { (d / max_d).sqrt() } else { 0.0 },
            )
        };
        // Target node radius per name, to stop arrows at the node's rim.
        let node_radius: std::collections::HashMap<String, f64> = (0..data.nrows())
            .filter(|&i| is(i, "node"))
            .map(|i| {
                (
                    node[i].key_str().into_owned(),
                    radius(degree[i].as_f64().unwrap_or(0.0)),
                )
            })
            .collect();
        let head = arrow().kind(ArrowType::Closed).length(8.0);

        for i in (0..data.nrows()).filter(|&i| is(i, "edge")) {
            let (Some(x0), Some(y0), Some(x1), Some(y1)) = (
                xc[i].as_f64(),
                yc[i].as_f64(),
                xe[i].as_f64(),
                ye[i].as_f64(),
            ) else {
                continue;
            };
            let (a, mut b) = (px(x0, y0), px(x1, y1));
            let w = lerp(
                self.edge_width,
                value[i]
                    .as_f64()
                    .map(|v| if max_w > 0.0 { v / max_w } else { 1.0 })
                    .unwrap_or(0.0),
            );
            if self.directed {
                let r = node_radius
                    .get(to[i].key_str().as_ref())
                    .copied()
                    .unwrap_or(0.0)
                    + 1.0;
                let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                let len = dx.hypot(dy);
                if len > r {
                    b = (b.0 - dx / len * r, b.1 - dy / len * r);
                }
            }
            let (f, t) = (super::tip_value(&from[i]), super::tip_value(&to[i]));
            let sep = if self.directed { "→" } else { "—" };
            super::set_mark(
                backend,
                Some(format!("{f} {sep} {t}: {}", super::tip_value(&value[i]))),
                Some(f.clone()),
                Some(f),
                super::raw_value(&value[i]),
            );
            backend.draw_line(
                &[a, b],
                &LineStyle {
                    color: self.edge_color,
                    alpha: self.edge_alpha,
                    width: w,
                    linetype: Linetype::Solid,
                    ..Default::default()
                },
            )?;
            if self.directed {
                let hs = heads(&[a, b], &head, w);
                draw_heads(backend, &hs, &head, self.edge_color, self.edge_alpha, w)?;
                backend.set_mark_part(None);
            }
        }

        let mut labels = Vec::new();
        for i in (0..data.nrows()).filter(|&i| is(i, "node")) {
            let (Some(x), Some(y)) = (xc[i].as_f64(), yc[i].as_f64()) else {
                continue;
            };
            let c = px(x, y);
            let r = radius(degree[i].as_f64().unwrap_or(0.0));
            let fill = fill_col
                .and_then(|f| scales.map_color(&Aesthetic::Fill, &f[i]))
                .unwrap_or(self.fill);
            let name = super::tip_value(&node[i]);
            super::set_mark(
                backend,
                Some(format!("{name} (degree {})", super::tip_value(&degree[i]))),
                Some(name.clone()),
                Some(name.clone()),
                super::raw_value(&degree[i]),
            );
            backend.draw_circle(
                c,
                r,
                &PointStyle {
                    color: fill,
                    alpha: 1.0,
                    filled: true,
                    ..Default::default()
                },
            )?;
            if self.labels {
                labels.push((name, (c.0 + r + 3.0, c.1)));
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
                    anchor: TextAnchor::Start,
                    angle: 0.0,
                    family: family.clone(),
                    face: FontFace::Plain,
                    bg_colour: Some((255, 255, 255)),
                    bg_r: 1.5,
                },
            )?;
        }
        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::From, Aesthetic::To]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatNetwork {
            layout: self.layout,
        })
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "network"
    }

    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let Some(ink) = el.ink {
            self.label_color = ink;
        }
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }
}
