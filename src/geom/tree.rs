//! Tree diagrams: `geom_tree` (tidy tree), org charts (boxed nodes) and
//! `geom_dendrogram`. Layout by [`StatTree`]; links drawn as elbows, steps,
//! diagonals or straight lines.

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
use crate::stat::tree::{StatTree, TreeOrientation};
use crate::stat::Stat;
use crate::theme::Theme;

use super::treemap::contrast_text;
use super::{Geom, GeomParams};

/// How a link joins parent and child.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TreeLink {
    /// Orthogonal with the turn halfway between the levels (org charts).
    #[default]
    Elbow,
    /// Orthogonal, turning at the parent's level (dendrograms).
    Step,
    /// A smooth S-curve (d3's `linkVertical` / `linkHorizontal`).
    Diagonal,
    /// A straight line.
    Straight,
}

/// How nodes are drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TreeNode {
    /// A dot, labelled beside it.
    #[default]
    Point,
    /// A rounded box with the name inside (org chart).
    Box,
    /// Nothing (dendrograms: only leaves are labelled).
    None,
}

/// Tree diagram over parent → child links (`from` → `to`).
pub struct GeomTree {
    pub orientation: TreeOrientation,
    pub align_leaves: bool,
    /// Column holding each link's parent height (dendrograms).
    pub height: Option<String>,
    pub link: TreeLink,
    pub node: TreeNode,
    /// Node fill when `fill` is unmapped.
    pub fill: (u8, u8, u8),
    pub link_color: (u8, u8, u8),
    pub link_width: f64,
    pub labels: bool,
    /// Label only the leaves.
    pub leaf_labels_only: bool,
    pub label_size: f64,
    pub label_color: (u8, u8, u8),
}

impl Default for GeomTree {
    fn default() -> Self {
        GeomTree {
            orientation: TreeOrientation::TopDown,
            align_leaves: false,
            height: None,
            link: TreeLink::Diagonal,
            node: TreeNode::Point,
            fill: (31, 119, 180),
            link_color: (150, 150, 150),
            link_width: 1.2,
            labels: true,
            leaf_labels_only: false,
            label_size: 9.0,
            label_color: (40, 40, 40),
        }
    }
}

impl GeomTree {
    /// An org chart: boxed nodes joined by elbows, top-down.
    pub fn org_chart() -> Self {
        GeomTree {
            link: TreeLink::Elbow,
            node: TreeNode::Box,
            ..Default::default()
        }
    }

    /// A dendrogram from [`dendrogram_links`](crate::stat::tree::dendrogram_links)
    /// output: step links at merge heights, leaves labelled.
    pub fn dendrogram() -> Self {
        GeomTree {
            height: Some("height".into()),
            link: TreeLink::Step,
            node: TreeNode::None,
            leaf_labels_only: true,
            link_color: (60, 60, 60),
            ..Default::default()
        }
    }

    /// Lay the tree out with this orientation.
    pub fn oriented(mut self, orientation: TreeOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Link path from parent `p` to child `c` (data coordinates).
    fn link_path(&self, p: (f64, f64), c: (f64, f64)) -> Vec<(f64, f64)> {
        let horizontal = self.orientation == TreeOrientation::LeftRight;
        match (self.link, self.orientation) {
            (TreeLink::Straight, _)
            | (TreeLink::Elbow | TreeLink::Step, TreeOrientation::Radial) => {
                vec![p, c]
            }
            (TreeLink::Elbow, _) if horizontal => {
                let m = (p.0 + c.0) / 2.0;
                vec![p, (m, p.1), (m, c.1), c]
            }
            (TreeLink::Elbow, _) => {
                let m = (p.1 + c.1) / 2.0;
                vec![p, (p.0, m), (c.0, m), c]
            }
            (TreeLink::Step, _) if horizontal => vec![p, (p.0, c.1), c],
            (TreeLink::Step, _) => vec![p, (c.0, p.1), c],
            (TreeLink::Diagonal, _) => {
                // Cubic Bézier with both controls halfway along the depth axis
                // (towards the centre's ring for radial trees: plain midpoints).
                let (c1, c2) = if horizontal {
                    let m = (p.0 + c.0) / 2.0;
                    ((m, p.1), (m, c.1))
                } else if self.orientation == TreeOrientation::TopDown {
                    let m = (p.1 + c.1) / 2.0;
                    ((p.0, m), (c.0, m))
                } else {
                    let mid = ((p.0 + c.0) / 2.0, (p.1 + c.1) / 2.0);
                    (mid, mid)
                };
                (0..=16)
                    .map(|k| {
                        let t = k as f64 / 16.0;
                        let u = 1.0 - t;
                        let b = |a: f64, b1: f64, b2: f64, d: f64| {
                            u * u * u * a
                                + 3.0 * u * u * t * b1
                                + 3.0 * u * t * t * b2
                                + t * t * t * d
                        };
                        (b(p.0, c1.0, c2.0, c.0), b(p.1, c1.1, c2.1, c.1))
                    })
                    .collect()
            }
        }
    }
}

impl Geom for GeomTree {
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
        let (node, from, to, leaf) = (col("node")?, col("from")?, col("to")?, col("leaf")?);
        let fill_col = data.column("fill");
        let area = backend.plot_area();
        let px = |(x, y): (f64, f64)| {
            let sx = scales.get(&Aesthetic::X).map(|s| s.map(&Value::Float(x)));
            let sy = scales.get(&Aesthetic::Y).map(|s| s.map(&Value::Float(y)));
            coord.transform((sx.unwrap_or(0.5), sy.unwrap_or(0.5)), &area)
        };
        let is = |i: usize, p: &str| part[i].key_str() == p;
        let family = super::resolve_font_family("", theme);
        let text = |_s: &str, pos: (f64, f64), anchor, color, face| {
            (
                pos,
                TextStyle {
                    color,
                    size: self.label_size,
                    anchor,
                    angle: 0.0,
                    family: family.clone(),
                    face,
                    bg_colour: None,
                    bg_r: 0.0,
                },
            )
        };

        for i in (0..data.nrows()).filter(|&i| is(i, "link")) {
            let (Some(x0), Some(y0), Some(x1), Some(y1)) = (
                xc[i].as_f64(),
                yc[i].as_f64(),
                xe[i].as_f64(),
                ye[i].as_f64(),
            ) else {
                continue;
            };
            let pts: Vec<(f64, f64)> = self
                .link_path((x0, y0), (x1, y1))
                .into_iter()
                .map(px)
                .collect();
            let (f, t) = (super::tip_value(&from[i]), super::tip_value(&to[i]));
            super::set_mark(backend, Some(format!("{f} → {t}")), Some(t), Some(f), None);
            backend.draw_line(
                &pts,
                &LineStyle {
                    color: self.link_color,
                    alpha: 1.0,
                    width: self.link_width,
                    linetype: Linetype::Solid,
                    ..Default::default()
                },
            )?;
        }

        for i in (0..data.nrows()).filter(|&i| is(i, "node")) {
            let (Some(x), Some(y)) = (xc[i].as_f64(), yc[i].as_f64()) else {
                continue;
            };
            let c = px((x, y));
            let name = super::tip_value(&node[i]);
            let is_leaf = matches!(leaf[i], Value::Bool(true));
            let fill = fill_col
                .and_then(|f| scales.map_color(&Aesthetic::Fill, &f[i]))
                .unwrap_or(self.fill);
            super::set_mark(
                backend,
                Some(name.clone()),
                Some(name.clone()),
                Some(name.clone()),
                None,
            );
            match self.node {
                TreeNode::Box => {
                    let w = name.chars().count() as f64 * self.label_size * 0.62 + 14.0;
                    let h = self.label_size + 12.0;
                    backend.draw_rect(
                        (c.0 - w / 2.0, c.1 - h / 2.0),
                        (c.0 + w / 2.0, c.1 + h / 2.0),
                        &RectStyle {
                            fill: Some(fill),
                            stroke: None,
                            stroke_width: 0.0,
                            alpha: 1.0,
                            clip: false,
                            corner_radius: 4.0,
                        },
                    )?;
                    if self.labels {
                        super::clear_mark(backend);
                        let (pos, style) = text(
                            &name,
                            c,
                            TextAnchor::Middle,
                            contrast_text(fill),
                            FontFace::Bold,
                        );
                        backend.draw_text(&name, pos, &style)?;
                    }
                    continue;
                }
                TreeNode::Point => {
                    backend.draw_circle(
                        c,
                        4.0,
                        &PointStyle {
                            color: fill,
                            alpha: 1.0,
                            filled: true,
                            ..Default::default()
                        },
                    )?;
                }
                TreeNode::None => {}
            }
            if !self.labels || (self.leaf_labels_only && !is_leaf) {
                continue;
            }
            super::clear_mark(backend);
            let (pos, anchor) = match self.orientation {
                TreeOrientation::TopDown if is_leaf => ((c.0, c.1 + 12.0), TextAnchor::Middle),
                TreeOrientation::TopDown => ((c.0 + 7.0, c.1 - 8.0), TextAnchor::Start),
                TreeOrientation::LeftRight => ((c.0 + 7.0, c.1), TextAnchor::Start),
                TreeOrientation::Radial => {
                    // Outward from the centre.
                    let o = px((0.0, 0.0));
                    let (dx, dy) = (c.0 - o.0, c.1 - o.1);
                    let len = dx.hypot(dy);
                    if len < 1e-6 {
                        ((c.0, c.1 - 10.0), TextAnchor::Middle)
                    } else {
                        let at = (c.0 + dx / len * 8.0, c.1 + dy / len * 8.0);
                        let anchor = if dx.abs() < 0.2 * len {
                            TextAnchor::Middle
                        } else if dx > 0.0 {
                            TextAnchor::Start
                        } else {
                            TextAnchor::End
                        };
                        (at, anchor)
                    }
                }
            };
            let (pos, style) = text(&name, pos, anchor, self.label_color, FontFace::Plain);
            backend.draw_text(&name, pos, &style)?;
        }
        super::clear_mark(backend);
        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::From, Aesthetic::To]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatTree {
            orientation: self.orientation,
            align_leaves: self.align_leaves,
            height: self.height.clone(),
        })
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "tree"
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
