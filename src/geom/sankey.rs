//! Sankey diagram: `geom_sankey`. Nodes are bars stacked in columns; links
//! are ribbons whose width is the flow, drawn as smooth S-curves between the
//! source's and target's bands. Layout by [`StatSankey`].

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, FontFace, RectStyle, TextAnchor, TextStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::sankey::StatSankey;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Sankey diagram. Map `from`, `to` and (optionally) `weight`; nodes are
/// coloured through the fill scale by node name, links by their source.
pub struct GeomSankey {
    /// Node width in column units (columns are 1 apart).
    pub node_width: f64,
    /// Gap between nodes as a fraction of the tallest column's flow.
    pub node_padding: f64,
    /// Move sink nodes to the last column.
    pub justify: bool,
    /// Fill used when no fill scale applies.
    pub fill: (u8, u8, u8),
    /// Node outline (`None` for none).
    pub color: Option<(u8, u8, u8)>,
    pub node_alpha: f64,
    pub link_alpha: f64,
    /// Draw node names beside the nodes.
    pub labels: bool,
    pub label_size: f64,
    pub label_color: (u8, u8, u8),
    /// Append the node's flow to its label (`"Gas (55)"`).
    pub label_values: bool,
    /// Label halo colour (`None` for no halo).
    pub label_halo: Option<(u8, u8, u8)>,
}

impl Default for GeomSankey {
    fn default() -> Self {
        GeomSankey {
            node_width: 0.12,
            node_padding: 0.08,
            justify: false,
            fill: (97, 156, 255),
            color: None,
            node_alpha: 1.0,
            link_alpha: 0.4,
            labels: true,
            label_size: 10.0,
            label_color: (40, 40, 40),
            label_values: false,
            label_halo: Some((255, 255, 255)),
        }
    }
}

/// Samples per link edge: enough for a smooth S-curve at poster sizes.
const LINK_STEPS: usize = 32;

impl Geom for GeomSankey {
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
        let (part, xmin, xmax, ymin, ymax) = (
            col("part")?,
            col("xmin")?,
            col("xmax")?,
            col("ymin")?,
            col("ymax")?,
        );
        let (emin, emax) = (col("yend_min")?, col("yend_max")?);
        let (node, from, to, value) = (col("node")?, col("from")?, col("to")?, col("value")?);
        let column = data.column("column");
        let fill_col = data.column("fill");
        let area = backend.plot_area();
        let mx = |v: f64| {
            scales
                .get(&Aesthetic::X)
                .map(|s| s.map(&Value::Float(v)))
                .unwrap_or(0.0)
        };
        let my = |v: f64| {
            scales
                .get(&Aesthetic::Y)
                .map(|s| s.map(&Value::Float(v)))
                .unwrap_or(0.0)
        };
        let f = |c: &[Value], i: usize| c[i].as_f64().unwrap_or(f64::NAN);
        let fill_of = |i: usize| {
            fill_col
                .and_then(|c| scales.map_color(&Aesthetic::Fill, &c[i]))
                .unwrap_or(self.fill)
        };
        let is = |i: usize, p: &str| part[i].key_str() == p;

        // Links first, under the nodes.
        for i in (0..data.nrows()).filter(|&i| is(i, "link")) {
            let (x0, x1) = (f(xmin, i), f(xmax, i));
            let (s0, s1, t0, t1) = (f(ymin, i), f(ymax, i), f(emin, i), f(emax, i));
            if ![x0, x1, s0, s1, t0, t1].iter().all(|v| v.is_finite()) {
                continue;
            }
            // Smoothstep in y = a cubic Bézier with both controls at mid-x.
            let ease = |t: f64| t * t * (3.0 - 2.0 * t);
            let mut pts = Vec::with_capacity(2 * (LINK_STEPS + 1));
            for k in 0..=LINK_STEPS {
                let t = k as f64 / LINK_STEPS as f64;
                let y = s1 + (t1 - s1) * ease(t);
                pts.push(coord.transform((mx(x0 + (x1 - x0) * t), my(y)), &area));
            }
            for k in (0..=LINK_STEPS).rev() {
                let t = k as f64 / LINK_STEPS as f64;
                let y = s0 + (t0 - s0) * ease(t);
                pts.push(coord.transform((mx(x0 + (x1 - x0) * t), my(y)), &area));
            }
            let (a, b) = (super::tip_value(&from[i]), super::tip_value(&to[i]));
            super::set_mark(
                backend,
                Some(format!("{a} → {b}: {}", super::tip_value(&value[i]))),
                Some(a.clone()),
                Some(a),
                super::raw_value(&value[i]),
            );
            backend.draw_polygon(
                &pts,
                &RectStyle {
                    fill: Some(fill_of(i)),
                    stroke: None,
                    stroke_width: 0.0,
                    alpha: self.link_alpha,
                    clip: true,
                    corner_radius: 0.0,
                },
            )?;
        }

        let last_column = column
            .map(|c| c.iter().filter_map(|v| v.as_f64()).fold(0.0, f64::max))
            .unwrap_or(0.0);
        let mut labels = Vec::new();
        for i in (0..data.nrows()).filter(|&i| is(i, "node")) {
            let (x0, x1, y0, y1) = (f(xmin, i), f(xmax, i), f(ymin, i), f(ymax, i));
            let name = super::tip_value(&node[i]);
            super::set_mark(
                backend,
                Some(format!("{name}: {}", super::tip_value(&value[i]))),
                Some(name.clone()),
                Some(name.clone()),
                super::raw_value(&value[i]),
            );
            let (px0, py0) = coord.transform((mx(x0), my(y1)), &area);
            let (px1, py1) = coord.transform((mx(x1), my(y0)), &area);
            backend.draw_rect(
                (px0.min(px1), py0.min(py1)),
                (px0.max(px1), py0.max(py1)),
                &RectStyle {
                    fill: Some(fill_of(i)),
                    stroke: self.color,
                    stroke_width: if self.color.is_some() { 0.5 } else { 0.0 },
                    alpha: self.node_alpha,
                    clip: true,
                    corner_radius: 0.0,
                },
            )?;
            if self.labels {
                // Last column: label to the left; elsewhere to the right.
                let at_end =
                    column.and_then(|c| c[i].as_f64()) == Some(last_column) && last_column > 0.0;
                let ymid = my((y0 + y1) / 2.0);
                let (pos, anchor) = if at_end {
                    (coord.transform((mx(x0), ymid), &area), TextAnchor::End)
                } else {
                    (coord.transform((mx(x1), ymid), &area), TextAnchor::Start)
                };
                let dx = if at_end { -4.0 } else { 4.0 };
                let text = if self.label_values {
                    format!("{name} ({})", super::tip_value(&value[i]))
                } else {
                    name
                };
                labels.push((text, (pos.0 + dx, pos.1), anchor));
            }
        }
        super::clear_mark(backend);
        let family = super::resolve_font_family("", theme);
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
                    bg_colour: self.label_halo,
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
        Box::new(StatSankey {
            node_width: self.node_width,
            node_padding: self.node_padding,
            justify: self.justify,
            ..Default::default()
        })
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "sankey"
    }

    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let Some(ink) = el.ink {
            self.label_color = ink;
        }
        if let (Some(paper), Some(_)) = (el.paper, self.label_halo) {
            self.label_halo = Some(paper);
        }
    }
}
