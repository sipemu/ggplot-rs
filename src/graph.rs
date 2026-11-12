//! Drawing a laid-out graph of boxes: swimlane bands, edges (polylines with
//! an arrowhead, a linetype, a width and an optional label placed clear of
//! the boxes) and node boxes (a wrapped label, an optional value line,
//! status badge and sparkline, and an accent bar), as one [`GGPlot`] of
//! ordinary layers.
//!
//! Works with [`graph_layout`](crate::graph_layout): size the boxes with
//! [`node_box`], lay them out with [`layered`](crate::graph_layout::layered),
//! then [`draw`] the result. What each mark says and what colour it takes
//! is the caller's (the [`DrawNode`] / [`DrawEdge`] fields); where things go
//! inside a box and along an edge is decided here. Every mark carries the
//! caller's `data-*` attributes, and what belongs to one node is wrapped in
//! `<g data-of="<id>">`, so a browser can address a node as one element.
//! Moved here from anofox-visualization's `::DIAGRAM`
//! (sipemu/ggplot-rs#497, DataZooDE/anofox-visualization#1153).

use crate::geom::arrow::arrow;
use crate::graph_layout::{GraphLayout, PlacedNode};
use crate::prelude::*;
use crate::render::backend::{LineEnd, Linetype};
use crate::scale::linewidth::ScaleLinewidthContinuous;

type Rgb = (u8, u8, u8);

/// Box geometry (px).
const LABEL_PX: f64 = 13.0;
const VALUE_PX: f64 = 12.0;
const BADGE_PX: f64 = 11.0;
/// An edge label's and a lane name's text size.
const SMALL_PX: f64 = 11.0;
const LINE_H: f64 = 16.0;
const BOX_PAD_X: f64 = 12.0;
const BOX_PAD_Y: f64 = 9.0;
const BOX_MIN_W: f64 = 92.0;
const BOX_MAX_W: f64 = 208.0;
/// A sparkline's strip in a box.
const SPARK_H: f64 = 22.0;
/// The arrowhead's length (px) at zero line width; the layout's
/// [`LayeredParams::arrow`](crate::graph_layout::LayeredParams::arrow).
const ARROW: f64 = 7.0;
/// A node label's line limit; only beyond it is the last line cut with "…".
const MAX_LINES: usize = 3;

/// Approximate rendered width of `s` at `px` (proportional sans-serif).
fn text_w(s: &str, px: f64, bold: bool) -> f64 {
    let per = if bold { 0.6 } else { 0.56 };
    s.chars().count() as f64 * px * per
}

/// The width (px) of an edge label as [`draw`] sets it (its widest line):
/// the layout's [`GraphEdge::label_w`](crate::graph_layout::GraphEdge::label_w).
pub fn edge_label_width(label: &str) -> f64 {
    label
        .split('\n')
        .map(|l| text_w(l, SMALL_PX, false))
        .fold(0.0, f64::max)
}

/// `s` wrapped into at most [`MAX_LINES`] lines of `max` px (the last cut
/// with "…" when the label doesn't fit). Breaks at spaces; a word longer
/// than a line breaks after a `/`, `-`, `_`, `.` or `,` in it
/// (`duckdb/community-` + `extensions`, not `community-ex` + `tensions`),
/// and only mid-word when it has none.
fn wrap(s: &str, max: f64) -> Vec<String> {
    let cap = ((max / (LABEL_PX * 0.6)).floor() as usize).max(4);
    // (piece, glued to the previous one) — a long word's pieces are glued
    let mut pieces: Vec<(String, bool)> = Vec::new();
    for word in s.split_whitespace() {
        let chars: Vec<char> = word.chars().collect();
        let mut start = 0;
        let mut first = true;
        while chars.len() - start > cap {
            let window = &chars[start..start + cap];
            let cut = window
                .iter()
                .rposition(|c| matches!(c, '/' | '-' | '_' | '.' | ','))
                .filter(|&i| i + 1 >= cap / 3)
                .map(|i| i + 1)
                .unwrap_or(cap);
            pieces.push((chars[start..start + cut].iter().collect(), !first));
            first = false;
            start += cut;
        }
        pieces.push((chars[start..].iter().collect(), !first));
    }
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for (piece, glued) in pieces {
        let sep = if glued || cur.is_empty() { 0 } else { 1 };
        if !cur.is_empty() && cur.chars().count() + sep + piece.chars().count() > cap {
            lines.push(std::mem::take(&mut cur));
        } else if sep == 1 {
            cur.push(' ');
        }
        cur.push_str(&piece);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    if lines.len() > MAX_LINES {
        let rest = lines[MAX_LINES - 1..].join(" ");
        let mut last: String = rest.chars().take(cap.saturating_sub(1)).collect();
        last = last.trim_end().to_string();
        last.push('…');
        lines.truncate(MAX_LINES - 1);
        lines.push(last);
    }
    lines
}

/// A node box's size and its label wrapped to fit.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeBox {
    pub w: f64,
    pub h: f64,
    pub lines: Vec<String>,
}

/// The box a node needs: its label wrapped onto at most three lines, plus a
/// line for `value`, a badge row for `badge` and a sparkline strip when
/// `spark` (as [`draw`] fills it).
pub fn node_box(label: &str, value: Option<&str>, badge: Option<&str>, spark: bool) -> NodeBox {
    let lines = wrap(label, BOX_MAX_W - 2.0 * BOX_PAD_X);
    let mut w = lines
        .iter()
        .map(|l| text_w(l, LABEL_PX, true))
        .fold(0.0, f64::max);
    let mut h = 2.0 * BOX_PAD_Y + LINE_H * lines.len() as f64;
    if let Some(v) = value {
        w = w.max(text_w(v, VALUE_PX, true));
        h += LINE_H;
    }
    if let Some(b) = badge {
        w = w.max(text_w(b, BADGE_PX, false) + 22.0);
        h += 20.0;
    }
    if spark {
        w = w.max(110.0);
        h += SPARK_H;
    }
    NodeBox {
        w: (w + 2.0 * BOX_PAD_X).clamp(BOX_MIN_W, BOX_MAX_W),
        h,
        lines,
    }
}

/// The colours and class names of a drawing.
#[derive(Clone, Debug)]
pub struct GraphStyle {
    /// Box and badge fill, and the halo behind edge labels.
    pub card: Rgb,
    /// Swimlane band tint.
    pub band: Rgb,
    /// Lane names and edge labels.
    pub muted: Rgb,
    /// Node labels.
    pub heading: Rgb,
    /// Node values and badge text.
    pub ink: Rgb,
    /// Prefix of the layer classes (`<prefix>node`, `<prefix>edge`,
    /// `<prefix>edge-label`, `<prefix>lane`, `<prefix>lane-label`,
    /// `<prefix>node-label`, `<prefix>node-value`, `<prefix>node-badge`,
    /// `<prefix>node-spark`).
    pub class_prefix: String,
}

impl Default for GraphStyle {
    fn default() -> Self {
        GraphStyle {
            card: (255, 255, 255),
            band: (226, 228, 233),
            muted: (107, 114, 128),
            heading: (17, 24, 39),
            ink: (31, 41, 55),
            class_prefix: "gg-graph-".to_string(),
        }
    }
}

/// A `data-*` attribute: its name (without `data-`) and value; `None` leaves
/// it off that mark.
pub type DataAttr = (String, Option<String>);

/// What a node's box shows, and its attributes. Its geometry comes from the
/// layout ([`PlacedNode`]); its label lines from [`node_box`].
#[derive(Clone, Debug, Default)]
pub struct DrawNode {
    /// The `data-of` group of everything drawn for the node.
    pub id: String,
    /// The label, wrapped ([`NodeBox::lines`]).
    pub lines: Vec<String>,
    /// A value line under the label.
    pub value: Option<String>,
    /// A badge row: its text and its dot's (and outline's) colour.
    pub badge: Option<(String, Rgb)>,
    /// The box outline.
    pub stroke: Rgb,
    /// A thin bar along the box's left edge.
    pub accent: Option<Rgb>,
    /// A sparkline (two values or more) along the bottom, and its colour.
    pub spark: Option<(Vec<f64>, Rgb)>,
    pub tooltip: String,
    /// `data-*` on the box (the hit target); every node lists the same
    /// names, in the same order.
    pub attrs: Vec<DataAttr>,
}

/// An edge's line pattern.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EdgeLine {
    #[default]
    Solid,
    Dashed,
    /// Round dots (zero-length dashes with round caps).
    Dots,
}

/// What an edge looks like, and its attributes. Its points come from the
/// layout ([`GraphLayout::edges`]); the arrowhead is on its last point.
#[derive(Clone, Debug, Default)]
pub struct DrawEdge {
    pub colour: Rgb,
    /// Line width (px), see [`flow_width`].
    pub width: f64,
    pub line: EdgeLine,
    /// Extra CSS classes on the path.
    pub class: Option<String>,
    pub tooltip: String,
    /// A label (`\n` breaks lines) on the edge, clear of the boxes.
    pub label: Option<String>,
    /// `data-*` on the path; every edge lists the same names, in the same
    /// order.
    pub attrs: Vec<DataAttr>,
    /// `data-*` on the label.
    pub label_attrs: Vec<DataAttr>,
}

/// An edge's width (px) for what it carries, `value` of the graph's `max`:
/// area-true (`sqrt`), 1.4 px to 7 px; 1.6 px without a value; dotted edges
/// 3–6 px so their dots show. Rounded to 0.1 px.
pub fn flow_width(value: Option<f64>, max: f64, line: EdgeLine) -> f64 {
    let width = match value {
        Some(v) if max > 0.0 => 1.4 + 5.6 * (v.max(0.0) / max).sqrt(),
        _ => 1.6,
    };
    let width = match line {
        EdgeLine::Dots => width.clamp(3.0, 6.0),
        _ => width,
    };
    (width * 10.0).round() / 10.0
}

/// Columns built row by row, in px with y down.
#[derive(Default)]
struct Frame(Vec<(String, Vec<Value>)>);

impl Frame {
    fn col(&mut self, name: &str) -> &mut Vec<Value> {
        let i = match self.0.iter().position(|(n, _)| n == name) {
            Some(i) => i,
            None => {
                self.0.push((name.to_string(), Vec::new()));
                self.0.len() - 1
            }
        };
        &mut self.0[i].1
    }
    fn num(&mut self, name: &str, v: f64) -> &mut Self {
        self.col(name).push(Value::Float(v));
        self
    }
    fn txt(&mut self, name: &str, v: impl Into<String>) -> &mut Self {
        self.col(name).push(Value::Str(v.into()));
        self
    }
    fn opt(&mut self, name: &str, v: Option<String>) -> &mut Self {
        self.col(name).push(v.map_or(Value::Na, Value::Str));
        self
    }
    /// `attrs` as columns `a:<name>`.
    fn attrs(&mut self, attrs: &[DataAttr]) -> &mut Self {
        for (n, v) in attrs {
            self.opt(&format!("a:{n}"), v.clone());
        }
        self
    }
    fn is_empty(&self) -> bool {
        self.0.first().is_none_or(|(_, v)| v.is_empty())
    }
    /// `aes` plus a `data_attr` per `a:<name>` column, in column order.
    fn attr_aes(&self, mut aes: Aes) -> Aes {
        for (n, _) in &self.0 {
            if let Some(name) = n.strip_prefix("a:") {
                aes = aes.data_attr(name, n);
            }
        }
        aes
    }
    /// The frame's columns, its pixel y (down) flipped to data y (up) on a
    /// canvas `h` tall: `y` → `h - y`, and a rect's `y0`/`y1` swapped as
    /// they flip so `y0` stays the low edge.
    fn take(&mut self, h: f64) -> Vec<(String, Vec<Value>)> {
        let flip = |v: Vec<Value>| -> Vec<Value> {
            v.into_iter()
                .map(|x| match x {
                    Value::Float(y) => Value::Float(h - y),
                    other => other,
                })
                .collect()
        };
        std::mem::take(&mut self.0)
            .into_iter()
            .map(|(n, v)| match n.as_str() {
                "y" => (n, flip(v)),
                "y0" => ("y1".to_string(), flip(v)),
                "y1" => ("y0".to_string(), flip(v)),
                _ => (n, v),
            })
            .collect()
    }
}

/// Frames keyed by a colour a geom only takes as a fixed parameter (a
/// rect's stroke), in first-seen order: one layer per colour.
fn by_colour(v: &mut Vec<(Rgb, Frame)>, c: Rgb) -> &mut Frame {
    let i = match v.iter().position(|(k, _)| *k == c) {
        Some(i) => i,
        None => {
            v.push((c, Frame::default()));
            v.len() - 1
        }
    };
    &mut v[i].1
}

fn hex(c: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
}

/// The point a fraction `t` (0..1) of the way along a polyline.
fn point_at(pts: &[(f64, f64)], t: f64) -> (f64, f64) {
    let seg = |i: usize| {
        let (a, b) = (pts[i], pts[i + 1]);
        ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt()
    };
    let total: f64 = (0..pts.len().saturating_sub(1)).map(seg).sum();
    let mut left = total * t;
    for i in 0..pts.len().saturating_sub(1) {
        let l = seg(i);
        if l >= left && l > 0.0 {
            let t = left / l;
            let (a, b) = (pts[i], pts[i + 1]);
            return (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
        }
        left -= l;
    }
    pts.first().copied().unwrap_or((0.0, 0.0))
}

/// Where an edge's label goes: the midpoint of the edge, else the nearest
/// point along it where the label's box (11 px lines drawn around `y - 4`)
/// touches no node box — so a node doesn't cover it.
fn edge_label_at(pts: &[(f64, f64)], label: &str, nodes: &[PlacedNode]) -> (f64, f64) {
    let lines = label.split('\n').count();
    let hw = edge_label_width(label) / 2.0 + 3.0;
    let hh = lines as f64 * 6.6 + 3.0;
    let clear = |(x, y): (f64, f64)| {
        let cy = y - 4.0 - 0.36 * SMALL_PX;
        nodes
            .iter()
            .all(|n| (x - n.cx).abs() >= hw + n.w / 2.0 || (cy - n.cy).abs() >= hh + n.h / 2.0)
    };
    let mid = point_at(pts, 0.5);
    [0.5, 0.42, 0.58, 0.34, 0.66, 0.26, 0.74, 0.18, 0.82]
        .into_iter()
        .map(|t| point_at(pts, t))
        .find(|&p| clear(p))
        .unwrap_or(mid)
}

/// Text is centred on its y (`dominant-baseline="middle"`); the layout
/// places lines by their baseline. A cap-height-ish shift between the two.
fn mid_of_baseline(baseline: f64, px: f64) -> f64 {
    baseline - 0.36 * px
}

/// Draw a laid-out graph on a `layout.w` × `layout.h` px canvas: `lanes`
/// names the layout's bands, `nodes` and `edges` are in the layout's order.
/// `theme` is the base (a void theme; the legend is turned off). Render the
/// result with e.g. `render_svg_native_at(0.0, 0.0, w, h)` at the canvas
/// size rounded up.
pub fn draw(
    layout: &GraphLayout,
    lanes: &[String],
    nodes: &[DrawNode],
    edges: &[DrawEdge],
    style: &GraphStyle,
    mut theme: Theme,
) -> GGPlot {
    let s = style;
    let class = |name: &str| format!("{}{name}", s.class_prefix);
    let (w, h) = (layout.w.ceil().max(1.0), layout.h.ceil().max(1.0));
    theme.legend_position = crate::theme::LegendPosition::None;
    let mut plot = GGPlot::new(vec![
        ("x".to_string(), vec![Value::Float(0.0), Value::Float(w)]),
        ("y".to_string(), vec![Value::Float(0.0), Value::Float(h)]),
    ])
    .theme(theme)
    // the canvas is the view (a zoom, so a mark reaching past it is clipped
    // rather than dropped); `Frame::take` flips the layout's y-down pixels
    .scale_x_continuous(ScaleContinuous::new().with_expand(0.0, 0.0))
    .scale_y_continuous(ScaleContinuous::new().with_expand(0.0, 0.0))
    .coord_cartesian_zoom(Some((0.0, w)), Some((0.0, h)))
    .scale_color_identity()
    .scale_fill_identity()
    .scale_linewidth(
        ScaleLinewidthContinuous::new()
            .with_limits(0.0, 16.0)
            .with_range(0.0, 16.0),
    )
    .scale_linetype_manual(vec![
        ("solid", Linetype::Solid),
        ("dashed", Linetype::Custom(vec![(6.0, 4.0)])),
        // round caps on a zero-length dash: moving dots
        ("dots", Linetype::Custom(vec![(0.0, 10.0)])),
    ]);
    let rect_aes = || Aes::new().xmin("x0").xmax("x1").ymin("y0").ymax("y1");
    let inert = [("pointer-events", "none")];

    // swimlanes (under everything): a tinted band per lane, its name on top
    let (mut even, mut odd, mut names) = (Frame::default(), Frame::default(), Frame::default());
    for &(l, x0, y0, x1, y1) in &layout.bands {
        let name = lanes.get(l).cloned().unwrap_or_default();
        let f = if l % 2 == 0 { &mut even } else { &mut odd };
        f.num("x0", x0).num("x1", x1).num("y0", y0).num("y1", y1);
        f.txt("lane", name.clone());
        names
            .num("x", x0 + 8.0)
            .num("y", mid_of_baseline(y0 + 13.0, SMALL_PX));
        names.txt("label", name);
    }
    for (mut f, alpha) in [(even, 0.35), (odd, 0.18)] {
        if f.is_empty() {
            continue;
        }
        plot = plot
            .geom_rect_with(
                GeomRect {
                    fill: s.band,
                    alpha,
                    line_width: 0.0,
                    ..Default::default()
                }
                .with_radius(6.0),
            )
            .layer_data(f.take(h))
            .layer_aes(rect_aes().data_attr("lane", "lane"))
            .layer_svg_attrs([("aria-hidden", "true"), ("class", &*class("lane"))]);
    }
    if !names.is_empty() {
        plot = plot
            .geom_text_with(GeomText {
                size: SMALL_PX,
                color: s.muted,
                hjust: 0.0,
                fontface: FontFace::Bold,
                ..Default::default()
            })
            .layer_data(names.take(h))
            .layer_aes(Aes::new().x("x").y("y").label("label"))
            .layer_svg_attrs([("aria-hidden", "true"), ("class", &*class("lane-label"))]);
    }

    // edges (under the nodes): one path per edge
    let (mut ef, mut lf) = (Frame::default(), Frame::default());
    for (k, e) in edges.iter().enumerate() {
        let Some(route) = layout.edges.get(k) else {
            continue;
        };
        let mut pts = route.clone();
        if pts.len() < 2 {
            continue;
        }
        // the arrow's tip is on the path's last point and its head is
        // stroked as wide as the line (its mitred tip reaching about one
        // line width further): end the path one width short of the box so
        // the tip lands on its edge.
        let n = pts.len();
        let ((fx, fy), (tx, ty)) = (pts[n - 2], pts[n - 1]);
        let len = ((tx - fx).powi(2) + (ty - fy).powi(2)).sqrt().max(1e-6);
        let pull = e.width.min(len / 2.0);
        pts[n - 1] = (tx - (tx - fx) / len * pull, ty - (ty - fy) / len * pull);
        let lt = match e.line {
            EdgeLine::Solid => "solid",
            EdgeLine::Dashed => "dashed",
            EdgeLine::Dots => "dots",
        };
        for &(x, y) in &pts {
            ef.num("x", x).num("y", y).txt("g", format!("e{k:04}"));
            ef.txt("col", hex(e.colour))
                .num("lw", e.width)
                .txt("lt", lt);
            ef.opt("cls", e.class.clone()).txt("tip", e.tooltip.clone());
            ef.attrs(&e.attrs);
        }
        if let Some(l) = &e.label {
            let (mx, my) = edge_label_at(route, l, &layout.nodes);
            lf.num("x", mx)
                .num("y", mid_of_baseline(my - 4.0, SMALL_PX));
            lf.txt("label", l.clone());
            lf.attrs(&e.label_attrs);
        }
    }
    if !ef.is_empty() {
        let aes = ef.attr_aes(
            Aes::new()
                .x("x")
                .y("y")
                .group("g")
                .color("col")
                .linewidth("lw")
                .linetype("lt")
                .class("cls")
                .tooltip("tip"),
        );
        plot = plot
            .geom_path_with(
                GeomPath::default()
                    .lineend(LineEnd::Round)
                    // a 7 px + width head, ~29° half-angle
                    .arrow(
                        arrow()
                            .closed()
                            .angle(29.0)
                            .length(ARROW + 1.0)
                            .length_per_linewidth(1.2),
                    ),
            )
            .layer_data(ef.take(h))
            .layer_aes(aes)
            .layer_svg_attr("class", class("edge"));
    }
    if !lf.is_empty() {
        // text directly on the line, a card-coloured halo keeping it legible
        let aes = lf.attr_aes(Aes::new().x("x").y("y").label("label"));
        plot = plot
            .geom_text_with(
                GeomText {
                    size: SMALL_PX,
                    color: s.muted,
                    ..Default::default()
                }
                .bg_colour(s.card)
                .bg_r(2.0),
            )
            .layer_data(lf.take(h))
            .layer_aes(aes)
            .layer_svg_attrs(inert)
            .layer_svg_attrs([("aria-hidden", "true"), ("class", &*class("edge-label"))]);
    }

    // nodes in reading order: the box (the hit target, outside any `data-of`
    // group), then what it shows, grouped per node
    let mut order: Vec<usize> = (0..nodes.len().min(layout.nodes.len())).collect();
    order.sort_by_key(|&i| (layout.nodes[i].order, i));
    let mut boxes: Vec<(Rgb, Frame)> = Vec::new();
    let mut badges: Vec<(Rgb, Frame)> = Vec::new();
    let (mut accent, mut lines, mut values) =
        (Frame::default(), Frame::default(), Frame::default());
    let (mut dots, mut btext, mut spark, mut ends) = (
        Frame::default(),
        Frame::default(),
        Frame::default(),
        Frame::default(),
    );
    for v in order {
        let nd = &nodes[v];
        let pn = &layout.nodes[v];
        let id = nd.id.clone();
        let (x, y) = (pn.cx - pn.w / 2.0, pn.cy - pn.h / 2.0);
        let f = by_colour(&mut boxes, nd.stroke);
        f.num("x0", x)
            .num("x1", x + pn.w)
            .num("y0", y)
            .num("y1", y + pn.h);
        f.txt("tip", nd.tooltip.clone()).attrs(&nd.attrs);

        if let Some(a) = nd.accent {
            accent.num("x0", x + 1.5).num("x1", x + 4.5);
            accent
                .num("y0", y + 6.0)
                .num("y1", y + 6.0 + (pn.h - 12.0).max(4.0));
            accent.txt("fill", hex(a)).txt("of", id.clone());
        }
        let mut base = y + BOX_PAD_Y + 12.0;
        for line in &nd.lines {
            lines
                .num("x", pn.cx)
                .num("y", mid_of_baseline(base, LABEL_PX));
            lines.txt("label", line.clone()).txt("of", id.clone());
            base += LINE_H;
        }
        if let Some(fv) = &nd.value {
            values
                .num("x", pn.cx)
                .num("y", mid_of_baseline(base, VALUE_PX));
            values.txt("label", fv.clone()).txt("of", id.clone());
            base += LINE_H;
        }
        if let Some((b, dot)) = &nd.badge {
            let bw = text_w(b, BADGE_PX, false) + 20.0;
            let (bx, by) = (pn.cx - bw / 2.0, base - 9.0);
            let f = by_colour(&mut badges, *dot);
            f.num("x0", bx)
                .num("x1", bx + bw)
                .num("y0", by)
                .num("y1", by + 16.0);
            f.txt("of", id.clone());
            dots.num("x", bx + 9.0).num("y", by + 8.0);
            dots.txt("col", hex(*dot)).txt("of", id.clone());
            btext.num("x", bx + 15.0).num("y", by + 8.0);
            btext.txt("label", b.clone()).txt("of", id.clone());
            base += 20.0;
        }
        // a mini trend along the bottom of the box
        if let Some((values, colour)) = nd.spark.as_ref().filter(|(v, _)| v.len() >= 2) {
            let (lo, hi) = values
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &v| {
                    (a.min(v), b.max(v))
                });
            let rng = if hi > lo { hi - lo } else { 1.0 };
            let (sx0, sw) = (x + BOX_PAD_X, pn.w - 2.0 * BOX_PAD_X);
            let (top, sh) = (base - 8.0, SPARK_H - 8.0);
            let last = values.len() - 1;
            let col = hex(*colour);
            for (i, &sv) in values.iter().enumerate() {
                let (px, py) = (
                    sx0 + sw * i as f64 / last as f64,
                    top + sh - (sv - lo) / rng * sh,
                );
                spark.num("x", px).num("y", py).txt("g", id.clone());
                spark.txt("col", col.clone()).txt("of", id.clone());
                if i == last {
                    ends.num("x", px).num("y", py);
                    ends.txt("col", col.clone()).txt("of", id.clone());
                }
            }
        }
    }
    for (stroke, mut f) in boxes {
        let aes = f.attr_aes(rect_aes().tooltip("tip"));
        plot = plot
            .geom_rect_with(
                GeomRect {
                    fill: s.card,
                    color: stroke,
                    alpha: 1.0,
                    line_width: 1.2,
                    ..Default::default()
                }
                .with_radius(8.0),
            )
            .layer_data(f.take(h))
            .layer_aes(aes)
            .layer_svg_attr("class", class("node"));
    }
    // everything below: dims / lights with its node (`data-of`), inert
    let of = |a: Aes| a.group_wrap("of", "of");
    if !accent.is_empty() {
        plot = plot
            .geom_rect_with(
                GeomRect {
                    alpha: 1.0,
                    line_width: 0.0,
                    ..Default::default()
                }
                .with_radius(1.5),
            )
            .layer_data(accent.take(h))
            .layer_aes(of(rect_aes().fill("fill")))
            .decorative()
            .layer_svg_attrs(inert);
    }
    for (mut f, size, colour, cls) in [
        (lines, LABEL_PX, s.heading, class("node-label")),
        (values, VALUE_PX, s.ink, class("node-value")),
    ] {
        if f.is_empty() {
            continue;
        }
        plot = plot
            .geom_text_with(GeomText {
                size,
                color: colour,
                fontface: FontFace::Bold,
                ..Default::default()
            })
            .layer_data(f.take(h))
            .layer_aes(of(Aes::new().x("x").y("y").label("label")))
            .layer_svg_attrs(inert)
            .layer_svg_attrs([("aria-hidden", "true"), ("class", cls.as_str())]);
    }
    for (dot, mut f) in badges {
        plot = plot
            .geom_rect_with(
                GeomRect {
                    fill: s.card,
                    color: dot,
                    alpha: 0.0,
                    line_width: 1.0,
                    ..Default::default()
                }
                .with_radius(8.0),
            )
            .layer_data(f.take(h))
            .layer_aes(of(rect_aes()))
            .decorative()
            .layer_svg_attrs(inert);
    }
    for (mut f, size) in [(dots, 3.0), (ends, 2.2)] {
        if f.is_empty() {
            continue;
        }
        plot = plot
            .geom_point_with(GeomPoint {
                size,
                ..Default::default()
            })
            .layer_data(f.take(h))
            .layer_aes(of(Aes::new().x("x").y("y").color("col")))
            .decorative()
            .layer_svg_attrs(inert);
    }
    if !btext.is_empty() {
        plot = plot
            .geom_text_with(GeomText {
                size: BADGE_PX,
                color: s.ink,
                hjust: 0.0,
                ..Default::default()
            })
            .layer_data(btext.take(h))
            .layer_aes(of(Aes::new().x("x").y("y").label("label")))
            .layer_svg_attrs(inert)
            .layer_svg_attrs([("aria-hidden", "true"), ("class", &*class("node-badge"))]);
    }
    if !spark.is_empty() {
        plot = plot
            .geom_path_with(GeomPath::default().linewidth(1.4))
            .layer_data(spark.take(h))
            .layer_aes(of(Aes::new().x("x").y("y").group("g").color("col")))
            .decorative()
            .layer_svg_attrs(inert)
            .layer_svg_attr("class", class("node-spark"));
    }
    plot
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph_layout::{layered, GraphEdge, GraphNode, LayeredParams};

    #[test]
    fn wraps_long_labels_in_three_lines() {
        let l = wrap(
            "Recommend replenishment orders for every store and every week of the coming year",
            152.0,
        );
        assert_eq!(l.len(), 3);
        assert!(l[2].ends_with('…'));
        assert_eq!(wrap("Ingest", 152.0), ["Ingest"]);
    }

    /// No ellipsis while the label fits; breaks at word boundaries and
    /// after `/` / `-`, never mid-word when there is a separator.
    #[test]
    fn wraps_at_word_boundaries() {
        let max = BOX_MAX_W - 2.0 * BOX_PAD_X;
        for label in [
            "duckdb/community-extensions (descriptions)",
            "Locked DuckDB server (anofox-visualization)",
            "Parquet snapshot (downloads, extensions)",
            "fetch.sh: download, file by ISO week, merge",
        ] {
            let l = wrap(label, max);
            assert!(l.len() <= MAX_LINES, "{l:?}");
            assert!(!l.iter().any(|x| x.contains('…')), "{label}: {l:?}");
            // every line ends at a space or after a separator
            let joined: String = l.concat();
            assert_eq!(joined.replace(' ', ""), label.replace(' ', ""), "{l:?}");
            for w in l.windows(2) {
                let end = w[0].chars().last().unwrap();
                let next = w[1].chars().next().unwrap();
                let at_space = label.contains(&format!(
                    "{} {}",
                    w[0].split(' ').next_back().unwrap(),
                    w[1].split(' ').next().unwrap()
                ));
                assert!(
                    at_space || matches!(end, '/' | '-' | '_' | '.' | ','),
                    "{l:?} breaks at {end}{next}"
                );
            }
        }
        assert_eq!(
            wrap("duckdb/community-extensions (descriptions)", max),
            ["duckdb/community-", "extensions", "(descriptions)"]
        );
    }

    #[test]
    fn node_box_grows_with_its_contents() {
        let plain = node_box("Ingest", None, None, false);
        assert_eq!((plain.w, plain.h), (BOX_MIN_W, 2.0 * BOX_PAD_Y + LINE_H));
        let full = node_box("Ingest", Some("1,240"), Some("late"), true);
        assert_eq!(full.h, plain.h + LINE_H + 20.0 + SPARK_H);
        assert!(full.w >= 110.0 + 2.0 * BOX_PAD_X);
        let long = node_box(&"word ".repeat(40), None, None, false);
        assert!(long.w <= BOX_MAX_W && long.lines.len() == MAX_LINES);
    }

    /// An edge label a node box would cover moves along the edge.
    #[test]
    fn edge_label_keeps_clear_of_nodes() {
        let node = |cx: f64, w: f64| PlacedNode {
            cx,
            cy: 0.0,
            w,
            h: 40.0,
            layer: 0,
            order: 0,
        };
        let pts = [(0.0, 0.0), (300.0, 0.0)];
        // nothing in the way: the midpoint
        assert_eq!(edge_label_at(&pts, "spikes flagged", &[]), (150.0, 0.0));
        // a box over the middle: somewhere else along the edge, clear of it
        let nodes = [node(150.0, 60.0)];
        let (x, _) = edge_label_at(&pts, "spikes flagged", &nodes);
        let hw = edge_label_width("spikes flagged") / 2.0 + 3.0;
        assert!((x - 150.0).abs() >= hw + 30.0, "{x}");
        assert_eq!(point_at(&pts, 0.25), (75.0, 0.0));
    }

    #[test]
    fn flow_width_is_area_true_and_rounded() {
        assert_eq!(flow_width(None, 10.0, EdgeLine::Solid), 1.6);
        assert_eq!(flow_width(Some(10.0), 10.0, EdgeLine::Solid), 7.0);
        assert_eq!(flow_width(Some(2.5), 10.0, EdgeLine::Solid), 4.2);
        assert_eq!(flow_width(None, 10.0, EdgeLine::Dots), 3.0);
    }

    /// Two lanes, three boxes, a labelled edge: the marks carry the
    /// caller's attributes and classes, a node's contents sit in its
    /// `data-of` group, and the box itself (the hit target) stays outside.
    #[test]
    fn draws_marks_with_attributes_and_groups() {
        let labels = ["Ingest", "Model", "Publish"];
        let boxes: Vec<NodeBox> = labels
            .iter()
            .enumerate()
            .map(|(i, l)| node_box(l, (i == 1).then_some("42"), None, i == 2))
            .collect();
        let gnodes: Vec<GraphNode> = boxes
            .iter()
            .enumerate()
            .map(|(i, b)| GraphNode {
                w: b.w,
                h: b.h,
                lane: Some(i % 2),
                ..Default::default()
            })
            .collect();
        let gedges = [
            GraphEdge {
                from: 0,
                to: 1,
                label_w: Some(edge_label_width("rows")),
            },
            GraphEdge {
                from: 1,
                to: 2,
                label_w: None,
            },
        ];
        let params = LayeredParams {
            lanes: 2,
            ..Default::default()
        };
        let l = layered(&gnodes, &gedges, &params);
        let nodes: Vec<DrawNode> = labels
            .iter()
            .zip(&boxes)
            .enumerate()
            .map(|(i, (id, b))| DrawNode {
                id: id.to_string(),
                lines: b.lines.clone(),
                value: (i == 1).then(|| "42".to_string()),
                stroke: (90, 90, 90),
                accent: (i == 0).then_some((200, 0, 0)),
                spark: (i == 2).then(|| (vec![1.0, 3.0, 2.0], (0, 0, 200))),
                tooltip: format!("{id}!"),
                attrs: vec![
                    ("node".to_string(), Some(id.to_string())),
                    ("value".to_string(), (i == 1).then(|| "42".to_string())),
                ],
                ..Default::default()
            })
            .collect();
        let edges: Vec<DrawEdge> = [(0, 1), (1, 2)]
            .iter()
            .map(|&(a, b)| DrawEdge {
                colour: (100, 100, 100),
                width: flow_width(None, 0.0, EdgeLine::Solid),
                line: if a == 0 {
                    EdgeLine::Solid
                } else {
                    EdgeLine::Dashed
                },
                class: (a == 1).then(|| "flowing".to_string()),
                tooltip: format!("{} → {}", labels[a], labels[b]),
                label: (a == 0).then(|| "rows".to_string()),
                attrs: vec![("edge".to_string(), Some(format!("{a}>{b}")))],
                label_attrs: vec![("from".to_string(), Some(labels[a].to_string()))],
            })
            .collect();
        let lanes = ["Left".to_string(), "Right".to_string()];
        let svg = draw(
            &l,
            &lanes,
            &nodes,
            &edges,
            &GraphStyle::default(),
            crate::theme::presets::theme_void(),
        )
        .render_svg_native_at(0.0, 0.0, l.w.ceil() as u32, l.h.ceil() as u32)
        .unwrap();
        // start tags, each with whether it sits inside a `data-of` group
        let mut tags: Vec<(&str, bool)> = Vec::new();
        let mut stack: Vec<bool> = Vec::new();
        for part in svg.split('<').skip(1) {
            let tag = &part[..part.find('>').unwrap()];
            if tag.starts_with('/') {
                stack.pop();
                continue;
            }
            let inside = stack.iter().any(|&g| g);
            tags.push((tag, inside));
            if !tag.ends_with('/') {
                stack.push(tag.contains("data-of="));
            }
        }
        let attr = |tag: &str, name: &str| {
            let key = format!(" {name}=\"");
            tag.find(&key).map(|i| {
                let rest = &tag[i + key.len()..];
                rest[..rest.find('"').unwrap()].to_string()
            })
        };
        // boxes: one per node, outside the groups, an NA attribute left off
        let boxes: Vec<_> = tags
            .iter()
            .filter(|(t, _)| t.contains("data-node="))
            .collect();
        assert_eq!(boxes.len(), 3);
        for (b, inside) in &boxes {
            assert!(!inside, "{b}");
            let id = attr(b, "data-node").unwrap();
            assert_eq!(attr(b, "data-value").is_some(), id == "Model", "{id}");
        }
        // edges: the caller's class on a path; the label's attributes
        assert!(tags.iter().any(|(t, _)| t.contains("data-edge=\"1&gt;2\"")
            && attr(t, "class").is_some_and(|c| c.contains("flowing"))));
        assert!(tags
            .iter()
            .any(|(t, _)| attr(t, "data-from").as_deref() == Some("Ingest")));
        assert!(svg.contains(" gg-graph-node\"") && svg.contains("gg-graph-edge-label"));
        // node contents grouped by node; lanes drawn with their names
        for id in labels {
            assert!(svg.contains(&format!("data-of=\"{id}\"")), "{id}");
        }
        assert!(svg.contains("data-lane=\"Right\""));
        assert!(svg.contains(">Left<") && svg.contains(">42<"));
    }
}
