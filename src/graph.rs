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
    /// A variable's circle ([`NodeShape::Circle`]).
    pub variable_fill: Rgb,
    /// An observed variable's circle ([`NodeShape::FilledCircle`]) and a
    /// delta node's square; their labels are drawn in `card`.
    pub observed_fill: Rgb,
    /// A plate's dashed outline and its label.
    pub plate: Rgb,
    /// Prefix of the layer classes (`<prefix>node`, `<prefix>edge`,
    /// `<prefix>edge-label`, `<prefix>lane`, `<prefix>lane-label`,
    /// `<prefix>node-label`, `<prefix>node-value`, `<prefix>node-badge`,
    /// `<prefix>node-spark`, `<prefix>plate`, `<prefix>plate-label`,
    /// `<prefix>node-state`, `<prefix>node-state-bar`,
    /// `<prefix>node-state-prior`, `<prefix>node-state-value`,
    /// `<prefix>node-outline`, `<prefix>node-diamond-mark`,
    /// `<prefix>node-state-delta`, `<prefix>node-icon`, `<prefix>edge-cut`;
    /// a shaped node's mark also has `<prefix>node-<shape>`).
    pub class_prefix: String,
    /// A factor graph's math font: when set, the labels of shaped nodes
    /// (variables, observations, equality nodes, glyph factors) are set in
    /// this family, italic and regular weight — `θ`, `x₁`, `𝒩` read as
    /// mathematics. `None` keeps the bold sans labels.
    pub math_font: Option<String>,
    /// A glyph factor's fill ([`NodeShape::Glyph`]); `None` is `card`.
    pub glyph_fill: Option<Rgb>,
    /// A glyph factor's glyph colour; `None` is `heading`.
    pub glyph_ink: Option<Rgb>,
    /// A discrete node's tint by how far its belief moved (every
    /// [`StateBar::was`] given): the box fill mixed towards this colour by
    /// the [`js_distance`] between the two, up to 45 % at a distance of 0.4
    /// or more. Only a node without a [`DrawNode::fill`] of its own; `None`
    /// tints nothing.
    pub moved: Option<Rgb>,
}

impl Default for GraphStyle {
    fn default() -> Self {
        GraphStyle {
            card: (255, 255, 255),
            band: (226, 228, 233),
            muted: (107, 114, 128),
            heading: (17, 24, 39),
            ink: (31, 41, 55),
            variable_fill: (229, 231, 235),
            observed_fill: (75, 85, 99),
            plate: (107, 114, 128),
            class_prefix: "gg-graph-".to_string(),
            math_font: None,
            glyph_fill: None,
            glyph_ink: None,
            moved: None,
        }
    }
}

/// A node's mark. A factor graph (Forney style) draws factors as boxes,
/// variables as circles (filled when observed), equality nodes as small
/// squares, deterministic (delta) nodes as small filled squares, and
/// constants as plain text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NodeShape {
    /// A box with a label, value, badge and sparkline ([`node_box`]).
    #[default]
    Box,
    Circle,
    FilledCircle,
    /// A small square, its label above it.
    Square,
    /// A small filled square, its label above it.
    FilledSquare,
    /// The label alone.
    Text,
    /// A small square box with a short glyph centred in it (a factor drawn
    /// as `𝒩`, `Γ`, `Ber`, …): uniform, so a factor graph stays compact.
    Glyph,
    /// A small filled square at the end of an edge, its label beside it (to
    /// the right): an observation in Forney style, where data enters.
    Terminal,
    /// A box with square corners and a box's contents: an influence
    /// diagram's decision.
    Rect,
    /// A diamond with the label (and a value line) inside: an influence
    /// diagram's utility.
    Diamond,
    /// A box with a second outline inside it and a box's contents: a
    /// deterministic node of a Bayesian network.
    DoubleBox,
}

impl NodeShape {
    fn name(self) -> &'static str {
        match self {
            NodeShape::Box => "box",
            NodeShape::Circle => "circle",
            NodeShape::FilledCircle => "filled-circle",
            NodeShape::Square => "square",
            NodeShape::FilledSquare => "filled-square",
            NodeShape::Text => "text",
            NodeShape::Glyph => "glyph",
            NodeShape::Terminal => "terminal",
            NodeShape::Rect => "rect",
            NodeShape::Diamond => "diamond",
            NodeShape::DoubleBox => "double-box",
        }
    }

    /// A box's contents (label, value, badge, sparkline, states) go inside:
    /// [`NodeShape::Box`], [`NodeShape::Rect`], [`NodeShape::DoubleBox`].
    pub fn boxed(self) -> bool {
        matches!(
            self,
            NodeShape::Box | NodeShape::Rect | NodeShape::DoubleBox
        )
    }
}

/// A belief curve's size over its node (px).
pub const BELIEF_W: f64 = 46.0;
pub const BELIEF_H: f64 = 16.0;
/// A small square's side (px).
const SQUARE: f64 = 14.0;
/// A terminal's side (px).
const TERMINAL: f64 = 11.0;
/// A glyph factor's side (px) and its glyph's size.
const GLYPH_SIDE: f64 = 34.0;
const GLYPH_PX: f64 = 15.0;

/// A state row's height, its bar's length and the widest state name (px).
const STATE_H: f64 = 15.0;
const STATE_BAR_W: f64 = 64.0;
const STATE_NAME_MAX: f64 = 84.0;
/// The gaps around a state's bar, and the room for its text ("100%").
const STATE_GAP: f64 = 6.0;
const STATE_TEXT_W: f64 = 30.0;
/// A box with state rows may be this much wider than a plain one.
const STATES_MAX_W: f64 = 224.0;
/// The room for a state's change after its probability (`▲21`).
const STATE_DELTA_W: f64 = 28.0;
/// A change smaller than this is not shown.
const MIN_DELTA: f64 = 0.005;

/// The Jensen–Shannon distance between two distributions over the same
/// states (base 2: 0 the same, 1 disjoint), each normalised to sum to 1;
/// 0 when either sums to 0 or they differ in length.
pub fn js_distance(p: &[f64], q: &[f64]) -> f64 {
    let clean = |v: &[f64]| -> Option<Vec<f64>> {
        let v: Vec<f64> = v
            .iter()
            .map(|x| if x.is_finite() { x.max(0.0) } else { 0.0 })
            .collect();
        let s: f64 = v.iter().sum();
        (s > 0.0).then(|| v.into_iter().map(|x| x / s).collect())
    };
    let (Some(p), Some(q)) = (clean(p), clean(q)) else {
        return 0.0;
    };
    if p.len() != q.len() {
        return 0.0;
    }
    let kl = |a: &[f64], m: &[f64]| -> f64 {
        a.iter()
            .zip(m)
            .filter(|(x, _)| **x > 0.0)
            .map(|(x, y)| x * (x / y).log2())
            .sum()
    };
    let m: Vec<f64> = p.iter().zip(&q).map(|(a, b)| (a + b) / 2.0).collect();
    let jsd = (kl(&p, &m) + kl(&q, &m)) / 2.0;
    jsd.clamp(0.0, 1.0).sqrt()
}

/// `a` mixed towards `b` by `t` (0..=1).
fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    let m = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * t).round() as u8;
    (m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
}

/// A node's tint ([`GraphStyle::moved`]): how much of the colour, by how
/// far its belief moved.
fn moved_share(states: &[StateBar]) -> Option<f64> {
    let was: Option<Vec<f64>> = states.iter().map(|r| r.was).collect();
    let was = was?;
    let now: Vec<f64> = states.iter().map(|r| r.p).collect();
    let d = js_distance(&now, &was);
    (d >= 0.01).then(|| (d / 0.4).min(1.0) * 0.45)
}

/// A state's change as its row shows it: `▲21`, `▼4` (percentage
/// points), `None` under [`MIN_DELTA`].
fn delta_text(p: f64, was: f64) -> Option<(String, bool)> {
    let d = p - was;
    if !d.is_finite() || d.abs() < MIN_DELTA {
        return None;
    }
    let pts = (d.abs() * 100.0).round().max(1.0);
    Some((
        format!("{}{pts:.0}", if d > 0.0 { '▲' } else { '▼' }),
        d > 0.0,
    ))
}

/// A hand (an intervention), in a 12 × 12 px box from its top left: the
/// palm and four fingers as rounded rects `(x0, y0, x1, y1)`, the thumb as
/// a polygon.
const HAND_RECTS: [(f64, f64, f64, f64); 5] = [
    (0.6, 5.2, 9.2, 11.8),
    (0.6, 1.8, 2.5, 7.0),
    (2.8, 0.4, 4.7, 7.0),
    (5.0, 0.8, 6.9, 7.0),
    (7.2, 2.2, 9.1, 7.0),
];
const HAND_THUMB: [(f64, f64); 4] = [(8.2, 8.4), (11.0, 5.2), (12.2, 6.2), (9.4, 10.6)];

/// A state's name as its row shows it: cut with "…" past
/// `STATE_NAME_MAX`.
fn state_name(name: &str) -> String {
    let cap = (STATE_NAME_MAX / (SMALL_PX * 0.56)).floor() as usize;
    if name.chars().count() <= cap {
        return name.to_string();
    }
    let mut s: String = name.chars().take(cap.saturating_sub(1)).collect();
    s = s.trim_end().to_string();
    s.push('…');
    s
}

/// The box a discrete node needs (a Bayesian network's node, Netica /
/// GeNIe style): [`node_box`]'s label, value and badge, then one row per
/// state (its name, a bar and the probability as text), as [`draw`] fills
/// it from [`DrawNode::states`]. No states: [`node_box`].
pub fn states_box(
    label: &str,
    value: Option<&str>,
    badge: Option<&str>,
    states: &[String],
) -> NodeBox {
    states_box_with_deltas(label, value, badge, states, false)
}

/// [`states_box`] with room for each row's change after its probability
/// (`deltas`, see [`StateBar::was`]).
pub fn states_box_with_deltas(
    label: &str,
    value: Option<&str>,
    badge: Option<&str>,
    states: &[String],
    deltas: bool,
) -> NodeBox {
    let b = node_box(label, value, badge, false);
    if states.is_empty() {
        return b;
    }
    let names = states
        .iter()
        .map(|n| text_w(&state_name(n), SMALL_PX, true))
        .fold(0.0, f64::max);
    let delta = if deltas { STATE_DELTA_W } else { 0.0 };
    let rows = names + STATE_GAP + STATE_BAR_W + STATE_GAP + STATE_TEXT_W + delta;
    NodeBox {
        w: b.w.max(rows + 2.0 * BOX_PAD_X).min(STATES_MAX_W + delta),
        h: b.h + 4.0 + STATE_H * states.len() as f64,
        lines: b.lines,
    }
}

/// The diamond a [`NodeShape::Diamond`] needs for its label and an
/// optional value line: the text's box, its corners on the diamond's edges.
pub fn diamond_box(label: &str, value: Option<&str>) -> NodeBox {
    let lines = wrap(label, 132.0);
    let mut w = lines
        .iter()
        .map(|l| text_w(l, LABEL_PX, true))
        .fold(0.0, f64::max);
    let mut h = LINE_H * lines.len() as f64;
    if let Some(v) = value {
        w = w.max(text_w(v, VALUE_PX, true));
        h += LINE_H;
    }
    // a w × h box inscribed in the diamond takes twice each, plus a margin
    NodeBox {
        w: (2.0 * w + 16.0).max(72.0),
        h: (2.0 * h + 8.0).max(44.0),
        lines,
    }
}

/// A table's widest first column (row names) and widest further column (px).
const TABLE_NAME_MAX: f64 = 96.0;
const TABLE_CELL_MAX: f64 = 84.0;
/// The gap between a table's columns (px).
const TABLE_GAP: f64 = 10.0;

/// A small table in a node's box, under its label (an influence diagram's
/// policy: a row per configuration of the decision's informational parents,
/// a column per option with its expected utility, the chosen option
/// marked). The first column holds the row names, left-aligned; the others
/// are right-aligned. Size the box with [`table_box`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NodeTable {
    /// The column heads (the first one names the rows and may be empty);
    /// all empty: no header row.
    pub header: Vec<String>,
    /// The rows: the name, then a cell per further column.
    pub rows: Vec<Vec<String>>,
    /// Per row, the column (1 or more) of its marked cell, e.g. the chosen
    /// option: bold, on a tint of the table's colour.
    pub marked: Vec<Option<usize>>,
    /// A row to highlight with a band, e.g. the configuration the evidence
    /// picks.
    pub current: Option<usize>,
    /// Per column, the `data-state` its head and cells carry (empty: none),
    /// e.g. the option a click on the column names.
    pub keys: Vec<String>,
}

/// `s` cut with "…" to at most `max` px (bold).
fn fit_text(s: &str, max: f64) -> String {
    let cap = (max / (SMALL_PX * 0.6)).floor() as usize;
    if s.chars().count() <= cap {
        return s.to_string();
    }
    let mut out: String = s.chars().take(cap.saturating_sub(1)).collect();
    out = out.trim_end().to_string();
    out.push('…');
    out
}

impl NodeTable {
    fn has_header(&self) -> bool {
        self.header.iter().any(|h| !h.is_empty())
    }

    fn columns(&self) -> usize {
        self.rows
            .iter()
            .map(Vec::len)
            .chain([self.header.len()])
            .max()
            .unwrap_or(0)
    }

    /// Column `c`'s `data-state`, if it has one.
    fn key(&self, c: usize) -> Option<String> {
        self.keys.get(c).filter(|k| !k.is_empty()).cloned()
    }

    /// A cell's text as drawn: cut to its column's limit.
    fn cell(&self, c: usize, s: &str) -> String {
        fit_text(
            s,
            if c == 0 {
                TABLE_NAME_MAX
            } else {
                TABLE_CELL_MAX
            },
        )
    }

    /// Each column's width (px), header and cells as drawn (bold, the
    /// widest case).
    fn widths(&self) -> Vec<f64> {
        (0..self.columns())
            .map(|c| {
                self.rows
                    .iter()
                    .filter_map(|r| r.get(c))
                    .chain(self.header.get(c))
                    .map(|s| text_w(&self.cell(c, s), SMALL_PX, true))
                    .fold(0.0, f64::max)
            })
            .collect()
    }

    /// The width of the table (px).
    fn width(&self) -> f64 {
        let w = self.widths();
        w.iter().sum::<f64>() + TABLE_GAP * w.len().saturating_sub(1) as f64
    }
}

/// The box a node with a [`NodeTable`] needs: [`node_box`]'s label, value
/// and badge, then the header row (when there is one) and a row per table
/// row, as [`draw`] fills it from [`DrawNode::table`].
pub fn table_box(
    label: &str,
    value: Option<&str>,
    badge: Option<&str>,
    table: &NodeTable,
) -> NodeBox {
    let b = node_box(label, value, badge, false);
    if table.rows.is_empty() {
        return b;
    }
    let rows = table.rows.len() + usize::from(table.has_header());
    NodeBox {
        w: b.w.max(table.width() + 2.0 * BOX_PAD_X),
        h: b.h + 4.0 + STATE_H * rows as f64,
        lines: b.lines,
    }
}

/// The size a node of `shape` needs for `label` (a [`NodeShape::Box`] is
/// [`node_box`] without a value, badge or sparkline). A circle fits its
/// label on one line, at least 30 px across.
pub fn node_size(shape: NodeShape, label: &str) -> NodeBox {
    let one = || vec![label.to_string()];
    match shape {
        NodeShape::Box | NodeShape::Rect | NodeShape::DoubleBox => {
            node_box(label, None, None, false)
        }
        NodeShape::Diamond => diamond_box(label, None),
        NodeShape::Circle | NodeShape::FilledCircle => {
            let d = (text_w(label, LABEL_PX, true) + 14.0).max(30.0);
            NodeBox {
                w: d,
                h: d,
                lines: one(),
            }
        }
        NodeShape::Square | NodeShape::FilledSquare => NodeBox {
            w: SQUARE,
            h: SQUARE,
            lines: if label.is_empty() { Vec::new() } else { one() },
        },
        NodeShape::Terminal => NodeBox {
            w: TERMINAL,
            h: TERMINAL,
            lines: if label.is_empty() { Vec::new() } else { one() },
        },
        NodeShape::Text => NodeBox {
            w: text_w(label, LABEL_PX, false) + 8.0,
            h: 20.0,
            lines: one(),
        },
        NodeShape::Glyph => {
            let side = (text_w(label, GLYPH_PX, true) + 14.0).max(GLYPH_SIDE);
            NodeBox {
                w: side,
                h: GLYPH_SIDE,
                lines: one(),
            }
        }
    }
}

/// A plate: a dashed box around a collection of nodes (indices in the
/// layout's order), labelled at its top left (`x_t`, `t = 1..T`).
#[derive(Clone, Debug, Default)]
pub struct Plate {
    pub label: String,
    pub nodes: Vec<usize>,
}

/// A `data-*` attribute: its name (without `data-`) and value; `None` leaves
/// it off that mark.
pub type DataAttr = (String, Option<String>);

/// One state of a discrete node: a row in its box with the state's name, a
/// bar for its probability and that probability as text, the prior as a
/// faint bar behind it with a tick where it ends (Netica / GeNIe style).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StateBar {
    pub name: String,
    /// The probability, 0..=1 (clamped).
    pub p: f64,
    /// The probability before any evidence, 0..=1.
    pub prior: Option<f64>,
    /// What the row says after the bar (e.g. `36%`).
    pub text: String,
    /// The observed state: its name in bold.
    pub observed: bool,
    /// The probability before the last change (e.g. under the evidence as it
    /// was), 0..=1: a ▲ / ▼ and the change in percentage points after the
    /// probability (`▲21`), none for a change under half a point. Size the
    /// box with [`states_box_with_deltas`].
    pub was: Option<f64>,
}

/// A mark on a node, at its top right corner in a small round badge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeIcon {
    /// A hand: the node is set by an intervention (`do`).
    Hand,
}

/// What a node's box shows, and its attributes. Its geometry comes from the
/// layout ([`PlacedNode`]); its label lines from [`node_box`].
#[derive(Clone, Debug, Default)]
pub struct DrawNode {
    /// The `data-of` group of everything drawn for the node.
    pub id: String,
    /// Its mark; anything but a box shows only the label.
    pub shape: NodeShape,
    /// The label, wrapped ([`NodeBox::lines`]).
    pub lines: Vec<String>,
    /// A value line under the label.
    pub value: Option<String>,
    /// A badge row: its text and its dot's (and outline's) colour.
    pub badge: Option<(String, Rgb)>,
    /// The box (or circle, square) outline.
    pub stroke: Rgb,
    /// A thin bar along the box's left edge; a filled mark (an observation,
    /// a terminal) is filled with it instead.
    pub accent: Option<Rgb>,
    /// A sparkline (two values or more) along the bottom, and its colour.
    pub spark: Option<(Vec<f64>, Rgb)>,
    /// A belief curve over the node (a factor graph variable's density):
    /// heights 0..=1 at evenly spaced points (three or more), and its colour.
    pub belief: Option<(Vec<f64>, Rgb)>,
    /// The prior behind the belief curve: heights on the same scale and
    /// span as [`DrawNode::belief`], drawn as a faint dashed outline, and
    /// its colour.
    pub prior: Option<(Vec<f64>, Rgb)>,
    /// A discrete node's states, one row each under the label (a boxed
    /// shape; size it with [`states_box`]), and the bars' colour.
    pub states: Option<(Vec<StateBar>, Rgb)>,
    /// The box's (or diamond's) fill; `None` is the card colour. E.g. a
    /// node with evidence, shaded.
    pub fill: Option<Rgb>,
    /// A mark at the box's top right corner, e.g. a hand for a node set by
    /// an intervention.
    pub icon: Option<NodeIcon>,
    /// A table under the label (a boxed shape; size it with
    /// [`table_box`]), e.g. an influence diagram's policy, and the colour
    /// its marked cells are tinted with.
    pub table: Option<(NodeTable, Rgb)>,
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
/// layout ([`GraphLayout::edges`]); the arrowhead is on its last point,
/// unless `undirected`.
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
    /// Where along the edge the label goes (0 at its start, 1 at its end),
    /// e.g. near a factor for an interface name; `None`: the middle, moved
    /// along the edge until no box covers it.
    pub label_at: Option<f64>,
    /// No arrowhead (a factor graph's edges).
    pub undirected: bool,
    /// The label names a variable (Forney style: the edge is the variable):
    /// set in [`GraphStyle::math_font`] when there is one, in the heading
    /// colour.
    pub label_math: bool,
    /// `data-*` on the path; every edge lists the same names, in the same
    /// order.
    pub attrs: Vec<DataAttr>,
    /// `data-*` on the label (and on a cut edge's break mark).
    pub label_attrs: Vec<DataAttr>,
    /// The edge is cut (its child is set by an intervention, so the parent
    /// no longer acts on it): a break mark across its middle — a gap and
    /// two slashes in the edge's colour. Give it a muted colour.
    pub cut: bool,
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

/// `aes` plus `data-state` when the frame has a `state` column (a state
/// row's name and probability: what a click on the row names; the bars are
/// decorative and carry none).
fn state_attr(f: &Frame, aes: Aes) -> Aes {
    if f.0.iter().any(|(n, _)| n == "state") {
        aes.data_attr("state", "state")
    } else {
        aes
    }
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

/// An end label's anchor (as for a mid label: the text goes 4 px above
/// it): on the edge's last (first) segment, its near side `LABEL_CLEAR`
/// from the box the edge ends at; on a vertical segment beside the line
/// instead, its top `LABEL_CLEAR` below (above) that box. At most half
/// the segment in.
fn end_label_at(pts: &[(f64, f64)], label: &str, t: f64) -> (f64, f64) {
    use crate::graph_layout::LABEL_CLEAR;
    let n = pts.len();
    if n < 2 {
        return pts.first().copied().unwrap_or((0.0, 0.0));
    }
    let (end, prev) = if t >= 0.5 {
        (pts[n - 1], pts[n - 2])
    } else {
        (pts[0], pts[1])
    };
    let (dx, dy) = (prev.0 - end.0, prev.1 - end.1);
    let len = (dx * dx + dy * dy).sqrt();
    if len == 0.0 {
        return end;
    }
    let (ux, uy) = (dx / len, dy / len);
    let lines = label.split('\n').count() as f64;
    let hw = edge_label_width(label) / 2.0;
    let hh = lines * 6.6;
    // the anchor sits 4 px + 0.36 em below the text's middle
    let below_mid = 4.0 + 0.36 * SMALL_PX;
    if ux.abs() >= uy.abs() {
        // along the line, above it: its near side LABEL_CLEAR from the box
        let s = (hw + LABEL_CLEAR).min(len * 0.5);
        (end.0 + ux * s, end.1 + uy * s)
    } else {
        // beside a vertical line, its top (bottom) clear of the box
        let s = (hh + LABEL_CLEAR).min(len * 0.5);
        (end.0 + LABEL_CLEAR + hw, end.1 + uy * s + below_mid)
    }
}

/// Text is centred on its y (`dominant-baseline="middle"`); the layout
/// places lines by their baseline. A cap-height-ish shift between the two.
fn mid_of_baseline(baseline: f64, px: f64) -> f64 {
    baseline - 0.36 * px
}

/// Draw a laid-out graph on a `layout.w` × `layout.h` px canvas: `lanes`
/// names the layout's bands, `plates` group nodes, `nodes` and `edges` are
/// in the layout's order.
/// `theme` is the base (a void theme; the legend is turned off). Render the
/// result with e.g. `render_svg_native_at(0.0, 0.0, w, h)` at the canvas
/// size rounded up.
pub fn draw(
    layout: &GraphLayout,
    lanes: &[String],
    plates: &[Plate],
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

    // plates (under the edges): a dashed box around each collection
    let (mut pf, mut pl) = (Frame::default(), Frame::default());
    for p in plates {
        let members: Vec<&PlacedNode> = p
            .nodes
            .iter()
            .filter_map(|&i| layout.nodes.get(i))
            .collect();
        if members.is_empty() {
            continue;
        }
        let pad = 8.0;
        let x0 = members
            .iter()
            .map(|n| n.cx - n.w / 2.0)
            .fold(f64::INFINITY, f64::min)
            - pad;
        let x1 = members
            .iter()
            .map(|n| n.cx + n.w / 2.0)
            .fold(f64::NEG_INFINITY, f64::max)
            + pad;
        let y0 = members
            .iter()
            .map(|n| n.cy - n.h / 2.0)
            .fold(f64::INFINITY, f64::min)
            - pad
            - 14.0;
        let y1 = members
            .iter()
            .map(|n| n.cy + n.h / 2.0)
            .fold(f64::NEG_INFINITY, f64::max)
            + pad;
        for (x, y) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1), (x0, y0)] {
            pf.num("x", x).num("y", y).txt("lt", "dashed");
            pf.txt("g", p.label.clone()).txt("plate", p.label.clone());
        }
        pl.num("x", x0 + 6.0)
            .num("y", mid_of_baseline(y0 + 13.0, SMALL_PX));
        pl.txt("label", p.label.clone());
    }
    if !pf.is_empty() {
        plot = plot
            .geom_path_with(GeomPath::default().color(s.plate).linewidth(1.0))
            .layer_data(pf.take(h))
            .layer_aes(
                Aes::new()
                    .x("x")
                    .y("y")
                    .group("g")
                    .linetype("lt")
                    .data_attr("plate", "plate"),
            )
            .layer_svg_attrs([("aria-hidden", "true"), ("class", &*class("plate"))]);
        plot = plot
            .geom_text_with(GeomText {
                size: SMALL_PX,
                color: s.plate,
                hjust: 0.0,
                ..Default::default()
            })
            .layer_data(pl.take(h))
            .layer_aes(Aes::new().x("x").y("y").label("label"))
            .layer_svg_attrs([("aria-hidden", "true"), ("class", &*class("plate-label"))]);
    }

    // edges (under the nodes): one path per edge, arrowed ones first
    let (mut ef, mut uf, mut lf) = (Frame::default(), Frame::default(), Frame::default());
    // a variable's name on its edge (Forney style), in the math font
    let mut lmf = Frame::default();
    // a cut edge's break mark: the gap (card-coloured) and two slashes
    let (mut cgap, mut cmark) = (Frame::default(), Frame::default());
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
        if !e.undirected {
            let n = pts.len();
            let ((fx, fy), (tx, ty)) = (pts[n - 2], pts[n - 1]);
            let len = ((tx - fx).powi(2) + (ty - fy).powi(2)).sqrt().max(1e-6);
            let pull = e.width.min(len / 2.0);
            pts[n - 1] = (tx - (tx - fx) / len * pull, ty - (ty - fy) / len * pull);
        }
        if e.cut {
            let total: f64 = pts
                .windows(2)
                .map(|w| ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt())
                .sum();
            if total > 16.0 {
                let (mx, my) = point_at(&pts, 0.5);
                let (ax, ay) = point_at(&pts, 0.5 - 2.0 / total);
                let (bx, by) = point_at(&pts, 0.5 + 2.0 / total);
                let len = ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt().max(1e-6);
                let (ux, uy) = ((bx - ax) / len, (by - ay) / len);
                let g = format!("c{k:04}");
                for d in [-4.0, 4.0] {
                    cgap.num("x", mx + ux * d).num("y", my + uy * d);
                    cgap.txt("g", g.clone()).num("lw", e.width + 2.5);
                    cgap.attrs(&e.label_attrs);
                }
                // each slash across the line, leaning 30° along it
                let (c30, s30) = (30f64.to_radians().cos(), 30f64.to_radians().sin());
                let (vx, vy) = (-uy * c30 + ux * s30, ux * c30 + uy * s30);
                for (i, off) in [-3.5, 3.5].into_iter().enumerate() {
                    let (cx, cy) = (mx + ux * off, my + uy * off);
                    for t in [-5.0, 5.0] {
                        cmark.num("x", cx + vx * t).num("y", cy + vy * t);
                        cmark.txt("g", format!("{g}-{i}")).txt("col", hex(e.colour));
                        cmark.attrs(&e.label_attrs);
                    }
                }
            }
        }
        let ef = if e.undirected { &mut uf } else { &mut ef };
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
        // a bus-routed edge's label is left to its tooltip: on the shared
        // lane or at a shared entry it would only clutter
        let routed = layout.routed.get(k).copied().unwrap_or(false);
        if let Some(l) = e.label.as_ref().filter(|_| !routed) {
            let (mx, my) = match e.label_at {
                // near an end (a factor's interface): beside the box at
                // that end, clear of it (anofox-visualization#1238)
                Some(t) => end_label_at(route, l, t),
                None => edge_label_at(route, l, &layout.nodes),
            };
            let f = if e.label_math && s.math_font.is_some() {
                &mut lmf
            } else {
                &mut lf
            };
            f.num("x", mx).num("y", mid_of_baseline(my - 4.0, SMALL_PX));
            f.txt("label", l.clone());
            f.attrs(&e.label_attrs);
        }
    }
    for (mut f, arrowed) in [(ef, true), (uf, false)] {
        if f.is_empty() {
            continue;
        }
        let aes = f.attr_aes(
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
        let mut geom = GeomPath::default().lineend(LineEnd::Round);
        if arrowed {
            // a 7 px + width head, ~29° half-angle
            geom = geom.arrow(
                arrow()
                    .closed()
                    .angle(29.0)
                    .length(ARROW + 1.0)
                    .length_per_linewidth(1.2),
            );
        }
        plot = plot
            .geom_path_with(geom)
            .layer_data(f.take(h))
            .layer_aes(aes)
            .layer_svg_attr("class", class("edge"));
    }
    if !cgap.is_empty() {
        let aes = cgap.attr_aes(Aes::new().x("x").y("y").group("g").linewidth("lw"));
        plot = plot
            .geom_path_with(GeomPath::default().color(s.card).lineend(LineEnd::Butt))
            .layer_data(cgap.take(h))
            .layer_aes(aes)
            .decorative()
            .layer_svg_attrs(inert)
            .layer_svg_attrs([("aria-hidden", "true"), ("class", &*class("edge-cut"))]);
        let aes = cmark.attr_aes(Aes::new().x("x").y("y").group("g").color("col"));
        plot = plot
            .geom_path_with(GeomPath::default().linewidth(1.4).lineend(LineEnd::Round))
            .layer_data(cmark.take(h))
            .layer_aes(aes)
            .decorative()
            .layer_svg_attrs(inert)
            .layer_svg_attrs([("aria-hidden", "true"), ("class", &*class("edge-cut"))]);
    }
    if !lmf.is_empty() {
        let aes = lmf.attr_aes(Aes::new().x("x").y("y").label("label"));
        plot = plot
            .geom_text_with(
                GeomText {
                    size: LABEL_PX + 1.0,
                    color: s.heading,
                    fontface: FontFace::Italic,
                    fontfamily: s.math_font.clone().unwrap_or_default(),
                    ..Default::default()
                }
                .bg_colour(s.card)
                .bg_r(2.0),
            )
            .layer_data(lmf.take(h))
            .layer_aes(aes)
            .layer_svg_attrs(inert)
            .layer_svg_attrs([("aria-hidden", "true"), ("class", &*class("edge-label"))]);
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
    // boxes, a layer per (outline, shape, fill)
    let mut boxes: Vec<((Rgb, NodeShape, Rgb), Frame)> = Vec::new();
    // a double box's inner outline, by colour
    let mut outlines: Vec<(Rgb, Frame)> = Vec::new();
    // a diamond's hit target (an invisible box) and its mark, by outline
    let (mut hits, mut diamonds) = (Frame::default(), Vec::<(Rgb, Frame)>::new());
    // state rows: the prior's faint bar, its tick, the bar, the names (the
    // observed one bold) and the probabilities
    let (mut sprior, mut stick, mut sbar) = (Frame::default(), Frame::default(), Frame::default());
    let (mut sname, mut sseen, mut stext) = (Frame::default(), Frame::default(), Frame::default());
    // a state's change (▲ up in the heading colour, bold; ▼ down muted)
    let (mut sup, mut sdown) = (Frame::default(), Frame::default());
    // a node's table: the current row's band, the marked cells' tint, the
    // header (left, right), the row names, the cells and the marked cells
    let (mut tband, mut tmarkbg) = (Frame::default(), Frame::default());
    let (mut thead_l, mut thead_r) = (Frame::default(), Frame::default());
    let (mut tname, mut tcell, mut tmark) = (Frame::default(), Frame::default(), Frame::default());
    // an icon's badge, its rounded rects and polygons
    let (mut ibadge, mut irect, mut ipoly) = (Frame::default(), Frame::default(), Frame::default());
    let mut beliefs = Frame::default();
    let mut priors = Frame::default();
    // shaped nodes, a layer per (shape, corner radius, outline)
    let mut shaped: Vec<((NodeShape, u64, Rgb), Frame)> = Vec::new();
    let (mut inside, mut inverse, mut above, mut glyphs, mut texts, mut beside) = (
        Frame::default(),
        Frame::default(),
        Frame::default(),
        Frame::default(),
        Frame::default(),
        Frame::default(),
    );
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
        // the belief curve sits over the node (over a square's label)
        let squared = matches!(nd.shape, NodeShape::Square | NodeShape::FilledSquare);
        let base = y - if squared { 17.0 } else { 3.0 };
        let x0 = pn.cx - BELIEF_W / 2.0;
        let curve = |hs: &[f64]| -> Vec<(f64, f64)> {
            let last = (hs.len() - 1) as f64;
            hs.iter()
                .enumerate()
                .map(|(i, &h)| {
                    (
                        x0 + BELIEF_W * i as f64 / last,
                        base - h.clamp(0.0, 1.0) * BELIEF_H,
                    )
                })
                .collect()
        };
        // the prior: a faint outline behind the belief
        if let Some((hs, col)) = nd.prior.as_ref().filter(|(h, _)| h.len() >= 3) {
            for (px, py) in curve(hs) {
                priors.num("x", px).num("y", py).txt("g", id.clone());
                priors.txt("col", hex(*col)).txt("lt", "dashed");
                priors.txt("of", id.clone());
            }
        }
        if let Some((hs, col)) = nd.belief.as_ref().filter(|(h, _)| h.len() >= 3) {
            let mut pts = curve(hs);
            pts.push((x0 + BELIEF_W, base));
            pts.push((x0, base));
            for (px, py) in pts {
                beliefs.num("x", px).num("y", py).txt("g", id.clone());
                beliefs.txt("fill", hex(*col)).txt("of", id.clone());
            }
        }
        if nd.shape == NodeShape::Diamond {
            hits.num("x0", x)
                .num("x1", x + pn.w)
                .num("y0", y)
                .num("y1", y + pn.h);
            hits.txt("tip", nd.tooltip.clone()).attrs(&nd.attrs);
            let f = by_colour(&mut diamonds, nd.stroke);
            for (px, py) in [(pn.cx, y), (x + pn.w, pn.cy), (pn.cx, y + pn.h), (x, pn.cy)] {
                f.num("x", px).num("y", py).txt("g", id.clone());
                f.txt("fill", hex(nd.fill.unwrap_or(s.card)))
                    .txt("of", id.clone());
            }
            // the label (and value) centred in it
            let n = nd.lines.len() + usize::from(nd.value.is_some());
            let mut base = pn.cy - LINE_H * n as f64 / 2.0 + 12.0;
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
            }
            continue;
        }
        if !nd.shape.boxed() {
            let r = match nd.shape {
                NodeShape::Circle | NodeShape::FilledCircle => pn.w.min(pn.h) / 2.0,
                NodeShape::Square | NodeShape::FilledSquare | NodeShape::Terminal => 1.5,
                NodeShape::Glyph => 4.0,
                _ => 0.0,
            };
            let (fill, stroke) = match nd.shape {
                NodeShape::Circle => (s.variable_fill, nd.stroke),
                // a filled mark takes the node's accent (a status: an
                // observation the model did not expect) over the observed fill
                NodeShape::FilledCircle | NodeShape::FilledSquare | NodeShape::Terminal => {
                    let c = nd.accent.unwrap_or(s.observed_fill);
                    (c, c)
                }
                NodeShape::Square => (s.card, nd.stroke),
                NodeShape::Glyph => (s.glyph_fill.unwrap_or(s.card), nd.stroke),
                _ => (s.card, s.card),
            };
            let key = (nd.shape, r.to_bits(), stroke);
            let i = match shaped.iter().position(|(k, _)| *k == key) {
                Some(i) => i,
                None => {
                    shaped.push((key, Frame::default()));
                    shaped.len() - 1
                }
            };
            let f = &mut shaped[i].1;
            f.num("x0", x)
                .num("x1", x + pn.w)
                .num("y0", y)
                .num("y1", y + pn.h);
            f.txt("fill", hex(fill));
            f.txt("tip", nd.tooltip.clone()).attrs(&nd.attrs);
            let label = nd.lines.join(" ");
            if label.is_empty() {
                continue;
            }
            let target = match nd.shape {
                NodeShape::Glyph => &mut glyphs,
                NodeShape::FilledCircle => &mut inverse,
                NodeShape::Square | NodeShape::FilledSquare => &mut above,
                // a constant is a number: upright, never the math italic
                NodeShape::Text => &mut texts,
                NodeShape::Terminal => &mut beside,
                _ => &mut inside,
            };
            let ly = match nd.shape {
                NodeShape::Square | NodeShape::FilledSquare => y - 4.0 - 0.36 * SMALL_PX,
                _ => pn.cy,
            };
            // a terminal's label starts just right of it
            let lx = match nd.shape {
                NodeShape::Terminal => x + pn.w + 5.0,
                _ => pn.cx,
            };
            target.num("x", lx).num("y", ly);
            target.txt("label", label).txt("of", id.clone());
            continue;
        }
        // a node without a fill of its own: tinted by how far it moved
        let fill = nd.fill.unwrap_or_else(|| {
            match (
                s.moved,
                nd.states.as_ref().and_then(|(r, _)| moved_share(r)),
            ) {
                (Some(c), Some(t)) => mix(s.card, c, t),
                _ => s.card,
            }
        });
        let key = (nd.stroke, nd.shape, fill);
        let i = match boxes.iter().position(|(k, _)| *k == key) {
            Some(i) => i,
            None => {
                boxes.push((key, Frame::default()));
                boxes.len() - 1
            }
        };
        let f = &mut boxes[i].1;
        if nd.shape == NodeShape::DoubleBox {
            let o = by_colour(&mut outlines, nd.stroke);
            o.num("x0", x + 3.5)
                .num("x1", x + pn.w - 3.5)
                .num("y0", y + 3.5)
                .num("y1", y + pn.h - 3.5);
            o.txt("of", id.clone());
        }
        f.num("x0", x)
            .num("x1", x + pn.w)
            .num("y0", y)
            .num("y1", y + pn.h);
        f.txt("tip", nd.tooltip.clone()).attrs(&nd.attrs);
        // the icon in a round badge on the top right corner
        if let Some(NodeIcon::Hand) = nd.icon {
            let (bx, by) = (x + pn.w - 11.0, y - 7.0);
            ibadge
                .num("x0", bx)
                .num("x1", bx + 18.0)
                .num("y0", by)
                .num("y1", by + 18.0);
            ibadge.txt("of", id.clone());
            let (ox, oy) = (bx + 3.0, by + 3.0);
            for (x0, y0, x1, y1) in HAND_RECTS {
                irect
                    .num("x0", ox + x0)
                    .num("x1", ox + x1)
                    .num("y0", oy + y0)
                    .num("y1", oy + y1);
                irect.txt("of", id.clone());
            }
            for (px, py) in HAND_THUMB {
                ipoly.num("x", ox + px).num("y", oy + py);
                ipoly.txt("g", id.clone()).txt("of", id.clone());
            }
        }

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
        // the states, a row each: name, the prior's faint bar with a tick
        // where it ends, the bar, the probability
        if let Some((rows, colour)) = nd.states.as_ref().filter(|(r, _)| !r.is_empty()) {
            let names = rows
                .iter()
                .map(|r| text_w(&state_name(&r.name), SMALL_PX, true))
                .fold(0.0, f64::max);
            let bx = x + BOX_PAD_X + names + STATE_GAP;
            let delta = if rows.iter().any(|r| r.was.is_some()) {
                STATE_DELTA_W
            } else {
                0.0
            };
            // the bar takes what the box leaves it, at most its full length
            let bw = (pn.w - 2.0 * BOX_PAD_X - names - 2.0 * STATE_GAP - STATE_TEXT_W - delta)
                .clamp(16.0, STATE_BAR_W);
            base += 4.0;
            for r in rows {
                let cy = mid_of_baseline(base, SMALL_PX);
                let f = if r.observed { &mut sseen } else { &mut sname };
                f.num("x", x + BOX_PAD_X).num("y", cy);
                f.txt("label", state_name(&r.name)).txt("of", id.clone());
                f.txt("state", r.name.clone());
                if let Some(q) = r.prior {
                    let q = if q.is_finite() {
                        q.clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    sprior
                        .num("x0", bx)
                        .num("x1", bx + bw * q)
                        .num("y0", cy - 5.0)
                        .num("y1", cy + 5.0);
                    sprior.txt("fill", hex(s.muted)).txt("of", id.clone());
                    stick
                        .num("x0", bx + bw * q - 0.75)
                        .num("x1", bx + bw * q + 0.75)
                        .num("y0", cy - 6.0)
                        .num("y1", cy + 6.0);
                    stick.txt("fill", hex(s.muted)).txt("of", id.clone());
                }
                let p = if r.p.is_finite() {
                    r.p.clamp(0.0, 1.0)
                } else {
                    0.0
                };
                sbar.num("x0", bx)
                    .num("x1", bx + bw * p)
                    .num("y0", cy - 3.5)
                    .num("y1", cy + 3.5);
                sbar.txt("fill", hex(*colour)).txt("of", id.clone());
                stext.num("x", bx + bw + STATE_GAP).num("y", cy);
                stext.txt("label", r.text.clone()).txt("of", id.clone());
                stext.txt("state", r.name.clone());
                if let Some((t, up)) = r.was.and_then(|w| delta_text(p, w)) {
                    let f = if up { &mut sup } else { &mut sdown };
                    f.num("x", bx + bw + STATE_GAP + STATE_TEXT_W).num("y", cy);
                    f.txt("label", t).txt("of", id.clone());
                }
                base += STATE_H;
            }
        }
        // a table (an influence diagram's policy): the header, then a row
        // each; the first column left, the others right-aligned
        if let Some((t, colour)) = nd.table.as_ref().filter(|(t, _)| !t.rows.is_empty()) {
            let widths = t.widths();
            let x0 = pn.cx - t.width() / 2.0;
            // each column's left and right edge
            let mut edges = Vec::with_capacity(widths.len());
            let mut cx = x0;
            for w in &widths {
                edges.push((cx, cx + w));
                cx += w + TABLE_GAP;
            }
            base += 4.0;
            if t.has_header() {
                let cy = mid_of_baseline(base, SMALL_PX);
                for (c, h) in t.header.iter().enumerate().filter(|(_, h)| !h.is_empty()) {
                    let (f, hx) = if c == 0 {
                        (&mut thead_l, edges[c].0)
                    } else {
                        (&mut thead_r, edges[c].1)
                    };
                    f.num("x", hx).num("y", cy);
                    f.txt("label", t.cell(c, h)).txt("of", id.clone());
                    if c > 0 {
                        f.opt("state", t.key(c));
                    }
                }
                base += STATE_H;
            }
            for (k, row) in t.rows.iter().enumerate() {
                let cy = mid_of_baseline(base, SMALL_PX);
                if t.current == Some(k) {
                    tband
                        .num("x0", x + 4.0)
                        .num("x1", x + pn.w - 4.0)
                        .num("y0", cy - 7.0)
                        .num("y1", cy + 7.0);
                    tband.txt("fill", hex(s.muted)).txt("of", id.clone());
                }
                let mark = t.marked.get(k).copied().flatten();
                for (c, cell) in row.iter().enumerate().take(edges.len()) {
                    let marked = c > 0 && mark == Some(c);
                    if marked {
                        tmarkbg
                            .num("x0", edges[c].0 - 3.0)
                            .num("x1", edges[c].1 + 3.0)
                            .num("y0", cy - 6.5)
                            .num("y1", cy + 6.5);
                        tmarkbg.txt("fill", hex(*colour)).txt("of", id.clone());
                    }
                    let (f, cx) = match (c, marked) {
                        (0, _) => (&mut tname, edges[c].0),
                        (_, true) => (&mut tmark, edges[c].1),
                        _ => (&mut tcell, edges[c].1),
                    };
                    f.num("x", cx).num("y", cy);
                    f.txt("label", t.cell(c, cell)).txt("of", id.clone());
                    if c > 0 {
                        f.opt("state", t.key(c));
                    }
                }
                base += STATE_H;
            }
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
    for ((stroke, shape, fill), mut f) in boxes {
        let aes = f.attr_aes(rect_aes().tooltip("tip"));
        let cls = match shape {
            NodeShape::Box => class("node"),
            _ => format!(
                "{} {}",
                class("node"),
                class(&format!("node-{}", shape.name()))
            ),
        };
        plot = plot
            .geom_rect_with(
                GeomRect {
                    fill,
                    color: stroke,
                    alpha: 1.0,
                    line_width: 1.2,
                    ..Default::default()
                }
                .with_radius(if shape == NodeShape::Rect { 0.0 } else { 8.0 }),
            )
            .layer_data(f.take(h))
            .layer_aes(aes)
            .layer_svg_attr("class", cls);
    }
    if !hits.is_empty() {
        // a diamond's hit target: its bounding box, invisible
        let aes = hits.attr_aes(rect_aes().tooltip("tip"));
        let cls = format!("{} {}", class("node"), class("node-diamond"));
        plot = plot
            .geom_rect_with(GeomRect {
                fill: s.card,
                alpha: 0.0,
                line_width: 0.0,
                ..Default::default()
            })
            .layer_data(hits.take(h))
            .layer_aes(aes)
            .layer_svg_attr("class", cls);
    }
    for ((shape, r, stroke), mut f) in shaped {
        let aes = f.attr_aes(rect_aes().fill("fill").tooltip("tip"));
        let cls = format!(
            "{} {}",
            class("node"),
            class(&format!("node-{}", shape.name()))
        );
        plot = plot
            .geom_rect_with(
                GeomRect {
                    color: stroke,
                    alpha: 1.0,
                    line_width: if shape == NodeShape::Text { 0.0 } else { 1.2 },
                    ..Default::default()
                }
                .with_radius(f64::from_bits(r)),
            )
            .layer_data(f.take(h))
            .layer_aes(aes)
            .layer_svg_attr("class", cls);
    }
    if !priors.is_empty() {
        plot = plot
            .geom_path_with(GeomPath {
                alpha: 0.7,
                ..GeomPath::default().linewidth(1.0)
            })
            .layer_data(priors.take(h))
            .layer_aes(
                Aes::new()
                    .x("x")
                    .y("y")
                    .group("g")
                    .color("col")
                    .linetype("lt")
                    .group_wrap("of", "of"),
            )
            .decorative()
            .layer_svg_attrs(inert)
            .layer_svg_attr("class", class("node-prior"));
    }
    if !beliefs.is_empty() {
        plot = plot
            .geom_polygon_with(GeomPolygon {
                alpha: 0.45,
                line_width: 0.0,
                ..Default::default()
            })
            .layer_data(beliefs.take(h))
            .layer_aes(
                Aes::new()
                    .x("x")
                    .y("y")
                    .group("g")
                    .fill("fill")
                    .group_wrap("of", "of"),
            )
            .decorative()
            .layer_svg_attrs(inert)
            .layer_svg_attr("class", class("node-belief"));
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
    for (stroke, mut f) in outlines {
        plot = plot
            .geom_rect_with(
                GeomRect {
                    fill: s.card,
                    color: stroke,
                    alpha: 0.0,
                    line_width: 1.0,
                    ..Default::default()
                }
                .with_radius(5.5),
            )
            .layer_data(f.take(h))
            .layer_aes(of(rect_aes()))
            .decorative()
            .layer_svg_attrs(inert)
            .layer_svg_attr("class", class("node-outline"));
    }
    for (stroke, mut f) in diamonds {
        plot = plot
            .geom_polygon_with(GeomPolygon {
                color: stroke,
                alpha: 1.0,
                line_width: 1.2,
                ..Default::default()
            })
            .layer_data(f.take(h))
            .layer_aes(of(Aes::new().x("x").y("y").group("g").fill("fill")))
            .decorative()
            .layer_svg_attrs(inert)
            .layer_svg_attr("class", class("node-diamond-mark"));
    }
    for (mut f, alpha, cls) in [
        (sprior, 0.28, "node-state-prior"),
        (sbar, 0.9, "node-state-bar"),
        (stick, 0.9, "node-state-prior"),
        (tband, 0.14, "node-table-current"),
        (tmarkbg, 0.2, "node-table-mark"),
    ] {
        if f.is_empty() {
            continue;
        }
        plot = plot
            .geom_rect_with(GeomRect {
                alpha,
                line_width: 0.0,
                ..Default::default()
            })
            .layer_data(f.take(h))
            .layer_aes(of(rect_aes().fill("fill")))
            .decorative()
            .layer_svg_attrs(inert)
            .layer_svg_attr("class", class(cls));
    }
    for (mut f, colour, face, cls) in [
        (sname, s.ink, FontFace::Plain, "node-state"),
        (sseen, s.heading, FontFace::Bold, "node-state"),
        (stext, s.muted, FontFace::Plain, "node-state-value"),
    ] {
        if f.is_empty() {
            continue;
        }
        plot = plot
            .geom_text_with(GeomText {
                size: SMALL_PX,
                color: colour,
                fontface: face,
                hjust: 0.0,
                ..Default::default()
            })
            .layer_aes(of(state_attr(&f, Aes::new().x("x").y("y").label("label"))))
            .layer_data(f.take(h))
            .layer_svg_attrs(inert)
            .layer_svg_attrs([("aria-hidden", "true"), ("class", &*class(cls))]);
    }
    for (mut f, colour, face, hjust, cls) in [
        (thead_l, s.muted, FontFace::Plain, 0.0, "node-table-head"),
        (thead_r, s.muted, FontFace::Plain, 1.0, "node-table-head"),
        (tname, s.ink, FontFace::Plain, 0.0, "node-table-name"),
        (tcell, s.muted, FontFace::Plain, 1.0, "node-table-cell"),
        (
            tmark,
            s.heading,
            FontFace::Bold,
            1.0,
            "node-table-cell node-table-chosen",
        ),
    ] {
        if f.is_empty() {
            continue;
        }
        let cls = cls.split(' ').map(&class).collect::<Vec<_>>().join(" ");
        plot = plot
            .geom_text_with(GeomText {
                size: SMALL_PX,
                color: colour,
                fontface: face,
                hjust,
                ..Default::default()
            })
            .layer_aes(of(state_attr(&f, Aes::new().x("x").y("y").label("label"))))
            .layer_data(f.take(h))
            .layer_svg_attrs(inert)
            .layer_svg_attrs([("aria-hidden", "true"), ("class", &*cls)]);
    }
    for (mut f, face, colour) in [
        (sup, FontFace::Bold, s.heading),
        (sdown, FontFace::Plain, s.muted),
    ] {
        if f.is_empty() {
            continue;
        }
        plot = plot
            .geom_text_with(GeomText {
                size: SMALL_PX - 1.0,
                color: colour,
                fontface: face,
                hjust: 0.0,
                ..Default::default()
            })
            .layer_data(f.take(h))
            .layer_aes(of(Aes::new().x("x").y("y").label("label")))
            .layer_svg_attrs(inert)
            .layer_svg_attrs([
                ("aria-hidden", "true"),
                ("class", &*class("node-state-delta")),
            ]);
    }
    if !ibadge.is_empty() {
        plot = plot
            .geom_rect_with(
                GeomRect {
                    fill: s.card,
                    color: s.heading,
                    alpha: 1.0,
                    line_width: 1.1,
                    ..Default::default()
                }
                .with_radius(9.0),
            )
            .layer_data(ibadge.take(h))
            .layer_aes(of(rect_aes()))
            .decorative()
            .layer_svg_attrs(inert)
            .layer_svg_attrs([("aria-hidden", "true"), ("class", &*class("node-icon"))]);
        plot = plot
            .geom_rect_with(
                GeomRect {
                    fill: s.heading,
                    alpha: 1.0,
                    line_width: 0.0,
                    ..Default::default()
                }
                .with_radius(0.9),
            )
            .layer_data(irect.take(h))
            .layer_aes(of(rect_aes()))
            .decorative()
            .layer_svg_attrs(inert)
            .layer_svg_attrs([("aria-hidden", "true"), ("class", &*class("node-icon"))]);
        plot = plot
            .geom_polygon_with(GeomPolygon {
                fill: s.heading,
                alpha: 1.0,
                line_width: 0.0,
                ..Default::default()
            })
            .layer_data(ipoly.take(h))
            .layer_aes(of(Aes::new().x("x").y("y").group("g")))
            .decorative()
            .layer_svg_attrs(inert)
            .layer_svg_attrs([("aria-hidden", "true"), ("class", &*class("node-icon"))]);
    }
    for (mut f, size, colour, cls, math, hjust) in [
        (lines, LABEL_PX, s.heading, class("node-label"), false, 0.5),
        (values, VALUE_PX, s.ink, class("node-value"), false, 0.5),
        (inside, LABEL_PX, s.heading, class("node-label"), true, 0.5),
        (texts, LABEL_PX, s.heading, class("node-label"), false, 0.5),
        (inverse, LABEL_PX, s.card, class("node-label"), true, 0.5),
        (above, SMALL_PX, s.muted, class("node-label"), true, 0.5),
        (beside, LABEL_PX, s.heading, class("node-label"), true, 0.0),
        (
            glyphs,
            GLYPH_PX,
            s.glyph_ink.unwrap_or(s.heading),
            class("node-label"),
            true,
            0.5,
        ),
    ] {
        if f.is_empty() {
            continue;
        }
        // a factor graph's labels as mathematics: the math font, italic
        let (fontfamily, fontface, size) = match s.math_font.as_ref().filter(|_| math) {
            Some(family) => (family.clone(), FontFace::Italic, size + 2.0),
            None => (String::new(), FontFace::Bold, size),
        };
        plot = plot
            .geom_text_with(GeomText {
                size,
                color: colour,
                fontface,
                fontfamily,
                hjust,
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
    fn an_end_label_sits_beside_its_box() {
        // anofox-visualization#1238: an interface name near a factor is
        // clear of the factor's box, not centred on its border
        let hw = edge_label_width("variance") / 2.0;
        let (x, y) = end_label_at(&[(0.0, 0.0), (100.0, 0.0)], "variance", 0.8);
        assert!(
            (x - (100.0 - hw - crate::graph_layout::LABEL_CLEAR)).abs() < 1e-9 && y == 0.0,
            "{x} {y}"
        );
        let (x, _) = end_label_at(&[(0.0, 0.0), (100.0, 0.0)], "mean", 0.2);
        assert!(
            x - edge_label_width("mean") / 2.0 >= crate::graph_layout::LABEL_CLEAR - 1e-9,
            "{x}"
        );
        // a vertical edge: beside the line, below the box it leaves
        let (x, y) = end_label_at(&[(50.0, 0.0), (50.0, 40.0)], "out", 0.2);
        assert!(
            x - edge_label_width("out") / 2.0 > 50.0 && y > 0.0,
            "{x} {y}"
        );
        // a short edge: the label stays on it
        let (x, _) = end_label_at(&[(0.0, 0.0), (20.0, 0.0)], "variance", 0.8);
        assert!((x - 10.0).abs() < 1e-9, "{x}");
    }

    /// A Bayesian network's nodes (Netica / GeNIe style): a chance node's
    /// states as rows of bars with the prior behind them, an observed node
    /// shaded with its state in bold, a decision as a square-cornered box,
    /// a utility as a diamond (an invisible box as its hit target), a
    /// deterministic node with a double outline.
    #[test]
    fn draws_bayes_net_nodes_with_state_bars() {
        let states = vec!["yes".to_string(), "no".to_string()];
        let spec = [
            ("Rain", NodeShape::Box),
            ("Wet grass", NodeShape::Box),
            ("Water?", NodeShape::Rect),
            ("Cost", NodeShape::Diamond),
            ("Either", NodeShape::DoubleBox),
        ];
        let sizes: Vec<NodeBox> = spec
            .iter()
            .map(|(l, sh)| match sh {
                NodeShape::Diamond => diamond_box(l, Some("-12.5")),
                _ => states_box(l, None, None, &states),
            })
            .collect();
        // two state rows below the label; room for the bars
        let plain = node_box("Rain", None, None, false);
        assert_eq!(sizes[0].h, plain.h + 4.0 + 2.0 * STATE_H);
        assert!(sizes[0].w >= 2.0 * BOX_PAD_X + STATE_BAR_W + STATE_TEXT_W);
        assert!(sizes[3].w > sizes[3].h, "a diamond wider than tall");
        let gnodes: Vec<GraphNode> = sizes
            .iter()
            .map(|b| GraphNode {
                w: b.w,
                h: b.h,
                ..Default::default()
            })
            .collect();
        let links = [(0, 1), (2, 1), (1, 3), (0, 4)];
        let gedges: Vec<GraphEdge> = links
            .iter()
            .map(|&(a, b)| GraphEdge {
                from: a,
                to: b,
                label_w: None,
            })
            .collect();
        let params = LayeredParams {
            direction: crate::graph_layout::Direction::TopBottom,
            ..Default::default()
        };
        let l = layered(&gnodes, &gedges, &params);
        let bar = |name: &str, p: f64, prior: Option<f64>, observed: bool| StateBar {
            name: name.to_string(),
            p,
            prior,
            text: format!("{:.0}%", p * 100.0),
            observed,
            was: None,
        };
        let nodes: Vec<DrawNode> = spec
            .iter()
            .zip(&sizes)
            .enumerate()
            .map(|(i, ((_, shape), b))| DrawNode {
                id: format!("n{i}"),
                shape: *shape,
                lines: b.lines.clone(),
                value: (*shape == NodeShape::Diamond).then(|| "-12.5".to_string()),
                stroke: (60, 60, 60),
                tooltip: b.lines.join(" "),
                attrs: vec![("node".to_string(), Some(format!("n{i}")))],
                states: (*shape != NodeShape::Diamond).then(|| {
                    if i == 1 {
                        (
                            vec![
                                bar("yes", 1.0, Some(0.65), true),
                                bar("no", 0.0, Some(0.35), false),
                            ],
                            (37, 99, 235),
                        )
                    } else {
                        (
                            vec![
                                bar("yes", 0.3, Some(0.5), false),
                                bar("no", 0.7, Some(0.5), false),
                            ],
                            (37, 99, 235),
                        )
                    }
                }),
                fill: (i == 1).then_some((230, 236, 250)),
                ..Default::default()
            })
            .collect();
        let edges: Vec<DrawEdge> = links
            .iter()
            .map(|&(a, b)| DrawEdge {
                colour: (90, 90, 90),
                width: 1.6,
                attrs: vec![("edge".to_string(), Some(format!("{a}>{b}")))],
                ..Default::default()
            })
            .collect();
        let svg = draw(
            &l,
            &[],
            &[],
            &nodes,
            &edges,
            &GraphStyle::default(),
            crate::theme::presets::theme_void(),
        )
        .render_svg_native_at(0.0, 0.0, l.w.ceil() as u32, l.h.ceil() as u32)
        .unwrap();
        // one hit target per node, classed by shape; directed edges
        assert_eq!(svg.matches("data-node=").count(), spec.len());
        for c in [
            "gg-graph-node-rect",
            "gg-graph-node-diamond",
            "gg-graph-node-double-box",
        ] {
            assert!(svg.contains(c), "{c}");
        }
        assert!(
            svg.contains("gg-graph-node-diamond-mark") && svg.contains("gg-graph-node-outline")
        );
        assert!(
            svg.contains("data-part=\"arrow\""),
            "arrowheads at the child"
        );
        // state rows: four bars, their priors, the names and probabilities
        assert!(
            svg.contains("gg-graph-node-state-bar") && svg.contains("gg-graph-node-state-prior")
        );
        assert!(svg.contains(">30%<") && svg.contains(">100%<"));
        // the observed node shaded, its observed state in bold
        assert!(svg.contains("fill=\"#E6ECFA\""), "shaded");
        assert!(
            svg.match_indices(">yes</text>")
                .any(|(i, _)| svg[svg[..i].rfind("<text").unwrap()..i].contains("bold")),
            "the observed state in bold"
        );
        // the diamond's value inside it
        assert!(svg.contains(">-12.5<"));
        // a plain box keeps its round corners and the plain class
        let plain = svg
            .split("<rect")
            .find(|r| r.contains("data-node=\"n0\""))
            .unwrap();
        assert!(plain.contains("gg-graph-node\""), "{plain}");
    }

    /// An influence diagram's policy as a table in the decision's box
    /// (DataZooDE/anofox-visualization#1279): a header, a row per
    /// configuration of the informational parents, the chosen option bold
    /// on a tint, the configuration the evidence picks on a band.
    #[test]
    fn draws_a_policy_table_in_a_decision() {
        let table = NodeTable {
            header: vec!["Seismic".into(), "drill".into(), "don't drill".into()],
            rows: vec![
                vec!["closed".into(), "87.5".into(), "0".into()],
                vec!["open".into(), "22.9".into(), "0".into()],
                vec!["diffuse".into(), "-25.6".into(), "0".into()],
                vec![
                    "a very long configuration name".into(),
                    "1".into(),
                    "2".into(),
                ],
            ],
            marked: vec![Some(1), Some(1), Some(2)],
            current: Some(1),
            keys: vec![String::new(), "drill".into(), "dont".into()],
        };
        let b = table_box("Drill?", None, None, &table);
        let plain = node_box("Drill?", None, None, false);
        // a header row and four rows under the label, room for the columns
        assert_eq!(b.h, plain.h + 4.0 + 5.0 * STATE_H);
        assert!(
            b.w >= table.width() + 2.0 * BOX_PAD_X,
            "{} {}",
            b.w,
            table.width()
        );
        // no header row when every head is empty; no rows: a plain box
        let bare = NodeTable {
            header: vec![String::new(), "EU".into()],
            ..table.clone()
        };
        assert_eq!(table_box("Drill?", None, None, &bare).h, b.h);
        let headless = NodeTable {
            header: Vec::new(),
            ..table.clone()
        };
        assert_eq!(table_box("Drill?", None, None, &headless).h, b.h - STATE_H);
        assert_eq!(
            table_box("Drill?", None, None, &NodeTable::default()),
            plain
        );
        let gnodes = [GraphNode {
            w: b.w,
            h: b.h,
            ..Default::default()
        }];
        let l = layered(&gnodes, &[], &LayeredParams::default());
        let nodes = [DrawNode {
            id: "drill".into(),
            shape: NodeShape::Rect,
            lines: b.lines.clone(),
            stroke: (60, 60, 60),
            tooltip: "Drill?".into(),
            attrs: vec![("node".to_string(), Some("drill".to_string()))],
            table: Some((table, (37, 99, 235))),
            ..Default::default()
        }];
        let svg = draw(
            &l,
            &[],
            &[],
            &nodes,
            &[],
            &GraphStyle::default(),
            crate::theme::presets::theme_void(),
        )
        .render_svg_native_at(0.0, 0.0, l.w.ceil() as u32, l.h.ceil() as u32)
        .unwrap();
        assert!(
            svg.contains(">Seismic<") && svg.contains(">don&#39;t drill<"),
            "{svg}"
        );
        assert!(svg.contains(">closed<") && svg.contains(">-25.6<"));
        // the long name cut with …
        assert!(svg.contains(">a very long c…<"), "{svg}");
        // three chosen cells, bold on a tint; one band for the current row
        assert_eq!(svg.matches("gg-graph-node-table-chosen").count(), 3);
        assert_eq!(svg.matches("gg-graph-node-table-mark").count(), 3);
        assert!(svg.contains("gg-graph-node-table-mark"));
        assert!(svg.contains("gg-graph-node-table-current"));
        assert!(
            svg.match_indices(">87.5</text>")
                .any(|(i, _)| svg[svg[..i].rfind("<text").unwrap()..i].contains("bold")),
            "the chosen option in bold"
        );
        assert!(
            svg.match_indices(">0</text>")
                .any(|(i, _)| !svg[svg[..i].rfind("<text").unwrap()..i].contains("bold")),
            "the others plain"
        );
        // a column's head and cells name its key; the row names none
        assert_eq!(svg.matches("data-state=\"dont\"").count(), 5);
        assert_eq!(svg.matches("data-state=\"drill\"").count(), 5);
        assert!(!svg
            .split("<text")
            .any(|t| t.contains(">closed<") && t.contains("data-state")));
        // the cells are right-aligned
        assert!(
            svg.match_indices(">22.9</text>")
                .any(|(i, _)| svg[svg[..i].rfind("<text").unwrap()..i]
                    .contains("text-anchor=\"end\"")),
            "right-aligned"
        );
    }

    /// Interactive evidence on a Bayesian network
    /// (DataZooDE/anofox-visualization#1276): a state's change after its
    /// probability, a node tinted by how far it moved, a hand on a node set
    /// by an intervention, its incoming edge cut, `data-state` on each row.
    #[test]
    fn draws_belief_deltas_cut_edges_and_a_hand() {
        let states = vec!["yes".to_string(), "no".to_string()];
        let plain = states_box("Rain", None, None, &states);
        let wide = states_box_with_deltas("Rain", None, None, &states, true);
        assert_eq!(wide.w, plain.w + STATE_DELTA_W, "room for the change");
        let gnodes = vec![
            GraphNode {
                w: wide.w,
                h: wide.h,
                ..Default::default()
            };
            2
        ];
        let l = layered(
            &gnodes,
            &[GraphEdge {
                from: 0,
                to: 1,
                label_w: None,
            }],
            &LayeredParams {
                direction: crate::graph_layout::Direction::TopBottom,
                ..Default::default()
            },
        );
        let bar = |name: &str, p: f64, was: f64| StateBar {
            name: name.to_string(),
            p,
            text: format!("{:.0}%", p * 100.0),
            was: Some(was),
            ..Default::default()
        };
        let nodes: Vec<DrawNode> = (0..2)
            .map(|i| DrawNode {
                id: format!("n{i}"),
                lines: vec![if i == 0 { "Cloudy" } else { "Rain" }.to_string()],
                stroke: (60, 60, 60),
                attrs: vec![("node".to_string(), Some(format!("n{i}")))],
                states: Some((
                    if i == 0 {
                        vec![bar("yes", 0.5, 0.5), bar("no", 0.5, 0.5)]
                    } else {
                        vec![bar("yes", 0.71, 0.5), bar("no", 0.29, 0.5)]
                    },
                    (37, 99, 235),
                )),
                icon: (i == 0).then_some(NodeIcon::Hand),
                ..Default::default()
            })
            .collect();
        let edges = vec![DrawEdge {
            colour: (160, 160, 160),
            width: 1.6,
            cut: true,
            attrs: vec![("edge".to_string(), Some("n0>n1".to_string()))],
            label_attrs: vec![("to".to_string(), Some("n1".to_string()))],
            ..Default::default()
        }];
        let style = GraphStyle {
            moved: Some((245, 158, 11)),
            ..Default::default()
        };
        let svg = draw(
            &l,
            &[],
            &[],
            &nodes,
            &edges,
            &style,
            crate::theme::presets::theme_void(),
        )
        .render_svg_native_at(0.0, 0.0, l.w.ceil() as u32, l.h.ceil() as u32)
        .unwrap();
        // Rain moved: ▲21 / ▼21, tinted; Cloudy did not move: no change, no tint
        assert!(svg.contains(">▲21<") && svg.contains(">▼21<"), "deltas");
        assert_eq!(svg.matches("gg-graph-node-state-delta").count(), 2);
        let tint = mix(
            (255, 255, 255),
            (245, 158, 11),
            moved_share(&nodes[1].states.as_ref().unwrap().0).unwrap(),
        );
        assert!(
            svg.to_ascii_lowercase()
                .contains(&format!("fill=\"{}\"", hex(tint))),
            "tinted"
        );
        assert!(
            svg.contains("fill=\"#FFFFFF\"") || svg.contains("fill=\"#ffffff\""),
            "Cloudy untinted"
        );
        // the hand and the cut mark
        assert!(svg.contains("gg-graph-node-icon"), "hand");
        assert!(svg.contains("gg-graph-edge-cut"), "cut mark");
        // a click on a row names its state
        assert!(svg.contains("data-state=\"yes\"") && svg.contains("data-state=\"no\""));
    }

    #[test]
    fn js_distance_is_zero_for_the_same_and_one_for_disjoint() {
        assert!(js_distance(&[0.5, 0.5], &[0.5, 0.5]).abs() < 1e-12);
        assert!((js_distance(&[1.0, 0.0], &[0.0, 1.0]) - 1.0).abs() < 1e-12);
        let d = js_distance(&[0.71, 0.29], &[0.5, 0.5]);
        assert!(d > 0.1 && d < 0.2, "{d}");
        // unnormalised input, or a length mismatch
        assert!((js_distance(&[2.0, 2.0], &[0.5, 0.5])).abs() < 1e-12);
        assert_eq!(js_distance(&[1.0], &[0.5, 0.5]), 0.0);
        assert_eq!(delta_text(0.71, 0.5), Some(("▲21".to_string(), true)));
        assert_eq!(delta_text(0.5, 0.503), None);
    }

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
                ..Default::default()
            })
            .collect();
        let lanes = ["Left".to_string(), "Right".to_string()];
        let svg = draw(
            &l,
            &lanes,
            &[],
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

    /// Forney style (#1172): an observation as a terminal square, its label
    /// beside it; a variable as a labelled edge, its name in the math font.
    #[test]
    fn forney_terminals_and_variable_edges() {
        use crate::graph_layout::{layered, GraphEdge, GraphNode, LayeredParams};
        let spec = [
            ("Cat", NodeShape::Glyph),
            ("T", NodeShape::Glyph),
            ("x₁", NodeShape::Terminal),
        ];
        let sizes: Vec<NodeBox> = spec.iter().map(|(l, sh)| node_size(*sh, l)).collect();
        assert_eq!((sizes[2].w, sizes[2].h), (TERMINAL, TERMINAL));
        let gnodes: Vec<GraphNode> = sizes
            .iter()
            .map(|b| GraphNode {
                w: b.w,
                h: b.h,
                ..Default::default()
            })
            .collect();
        let gedges = vec![
            GraphEdge {
                from: 0,
                to: 1,
                label_w: None,
            },
            GraphEdge {
                from: 1,
                to: 2,
                label_w: None,
            },
        ];
        let l = layered(&gnodes, &gedges, &LayeredParams::default());
        let nodes: Vec<DrawNode> = spec
            .iter()
            .zip(&sizes)
            .enumerate()
            .map(|(i, ((_, shape), b))| DrawNode {
                id: format!("n{i}"),
                shape: *shape,
                lines: b.lines.clone(),
                stroke: (13, 148, 136),
                tooltip: b.lines.join(" "),
                attrs: vec![("node".to_string(), Some(format!("n{i}")))],
                // the observation flagged: filled in its accent
                accent: (*shape == NodeShape::Terminal).then_some((217, 119, 6)),
                ..Default::default()
            })
            .collect();
        let edges = vec![
            // the variable s₀ between the two factors
            DrawEdge {
                colour: (13, 148, 136),
                width: 1.2,
                undirected: true,
                label: Some("s₀".to_string()),
                label_math: true,
                ..Default::default()
            },
            DrawEdge {
                colour: (13, 148, 136),
                width: 1.2,
                undirected: true,
                ..Default::default()
            },
        ];
        let render = |style: &GraphStyle| {
            draw(
                &l,
                &[],
                &[],
                &nodes,
                &edges,
                style,
                crate::theme::presets::theme_void(),
            )
            .render_svg_native_at(0.0, 0.0, l.w.ceil() as u32 + 40, l.h.ceil() as u32)
            .unwrap()
        };
        let math = GraphStyle {
            math_font: Some("STIX Two Math, serif".to_string()),
            ..GraphStyle::default()
        };
        let svg = render(&math);
        let el = |text: &str| {
            let at = svg
                .find(&format!(">{text}</text>"))
                .unwrap_or_else(|| panic!("{text}"));
            svg[svg[..at].rfind('<').unwrap()..at].to_string()
        };
        // the terminal: a filled square, its label starting beside it
        assert!(svg.contains("gg-graph-node-terminal"), "terminal mark");
        assert!(
            svg.contains("fill=\"#D97706\""),
            "the terminal in its accent"
        );
        let x1 = el("x₁");
        assert!(
            x1.contains("text-anchor=\"start\"") && x1.contains("italic"),
            "{x1}"
        );
        let t = &l.nodes[2];
        let lx: f64 = x1
            .split("x=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        assert!(
            lx > t.cx + t.w / 2.0,
            "the label right of the square: {lx} vs {}",
            t.cx
        );
        // the variable's edge label in the math font
        let s0 = el("s₀");
        assert!(
            s0.contains("STIX Two Math") && s0.contains("italic"),
            "{s0}"
        );
        // without a math font: a plain edge label
        let plain = render(&GraphStyle::default());
        let at = plain.find(">s₀</text>").unwrap();
        assert!(!plain[plain[..at].rfind('<').unwrap()..at].contains("italic"));
    }

    /// A coin toss as a Forney-style factor graph: a Beta prior box, the
    /// parameter's circle, an equality node, Bernoulli boxes and observed
    /// circles in a plate; undirected edges with interface names near the
    /// factor.
    #[test]
    fn draws_a_factor_graph_with_shapes_and_a_plate() {
        let n = 3;
        // (label, shape)
        let mut spec = vec![
            ("Beta", NodeShape::Box),
            ("θ", NodeShape::Circle),
            ("=", NodeShape::Square),
        ];
        for _ in 0..n {
            spec.push(("Ber", NodeShape::Box));
        }
        for _ in 0..n {
            spec.push(("y", NodeShape::FilledCircle));
        }
        spec.push(("2.0", NodeShape::Text));
        let sizes: Vec<NodeBox> = spec.iter().map(|(l, sh)| node_size(*sh, l)).collect();
        // (from, to, interface)
        let mut links = vec![(0, 1, "out"), (1, 2, ""), (9, 0, "a")];
        for i in 0..n {
            links.push((2, 3 + i, "p"));
            links.push((3 + i, 3 + n + i, "out"));
        }
        let gnodes: Vec<GraphNode> = sizes
            .iter()
            .map(|b| GraphNode {
                w: b.w,
                h: b.h,
                ..Default::default()
            })
            .collect();
        let gedges: Vec<GraphEdge> = links
            .iter()
            .map(|&(a, b, _)| GraphEdge {
                from: a,
                to: b,
                label_w: None,
            })
            .collect();
        // a plate reaches past its nodes: room for it in the margin
        let params = LayeredParams {
            margin: 32.0,
            ..Default::default()
        };
        let l = layered(&gnodes, &gedges, &params);
        let nodes: Vec<DrawNode> = spec
            .iter()
            .zip(&sizes)
            .enumerate()
            .map(|(i, ((_, shape), b))| DrawNode {
                id: format!("n{i}"),
                shape: *shape,
                lines: b.lines.clone(),
                stroke: (60, 60, 60),
                tooltip: b.lines.join(" "),
                attrs: vec![("node".to_string(), Some(format!("n{i}")))],
                // θ's belief curve
                belief: (i == 1).then(|| (vec![0.0, 0.3, 1.0, 0.3, 0.0], (120, 80, 200))),
                // and its prior, wider
                prior: (i == 1).then(|| (vec![0.2, 0.4, 0.5, 0.4, 0.2], (150, 150, 150))),
                ..Default::default()
            })
            .collect();
        let edges: Vec<DrawEdge> = links
            .iter()
            .map(|&(a, b, iface)| DrawEdge {
                colour: (90, 90, 90),
                width: 1.2,
                undirected: true,
                label: (!iface.is_empty()).then(|| iface.to_string()),
                // the interface name near the factor
                label_at: Some(if spec[b].1 == NodeShape::Box {
                    0.8
                } else {
                    0.2
                }),
                attrs: vec![("edge".to_string(), Some(format!("{a}>{b}")))],
                ..Default::default()
            })
            .collect();
        let plates = [Plate {
            label: "i = 1..3".to_string(),
            nodes: (3..3 + 2 * n).collect(),
        }];
        let style = GraphStyle::default();
        let svg = draw(
            &l,
            &[],
            &plates,
            &nodes,
            &edges,
            &style,
            crate::theme::presets::theme_void(),
        )
        .render_svg_native_at(0.0, 0.0, l.w.ceil() as u32, l.h.ceil() as u32)
        .unwrap();
        // every node is a mark, shapes classed, undirected: no arrowheads
        assert_eq!(svg.matches("data-node=").count(), spec.len());
        for shape in ["circle", "filled-circle", "square", "text"] {
            assert!(svg.contains(&format!("gg-graph-node-{shape}")), "{shape}");
        }
        assert!(
            !svg.contains("data-part=\"arrow\""),
            "an undirected edge has no arrowhead"
        );
        assert_eq!(svg.matches("data-edge=").count(), links.len());
        // θ's belief curve: a filled shape over its circle, in its group
        assert!(svg.contains("gg-graph-node-belief"), "belief layer");
        assert!(svg.contains("fill=\"#7850C8\""), "belief colour");
        // its prior: a dashed outline, drawn before (under) the belief
        let (pr, be) = (
            svg.find("gg-graph-node-prior").expect("prior layer"),
            svg.find("gg-graph-node-belief").unwrap(),
        );
        assert!(pr < be, "the prior under the belief");
        let mark = &svg[svg[..pr].rfind('<').unwrap()..pr];
        assert!(mark.contains("stroke-dasharray"), "dashed prior: {mark}");
        // no math font: bold sans labels
        assert!(!svg.contains("font-style=\"italic\""), "plain labels");
        // a math font: the shaped nodes' labels in it, italic; glyph colours
        let math = GraphStyle {
            math_font: Some("STIX Two Math, serif".to_string()),
            glyph_fill: Some((240, 247, 246)),
            glyph_ink: Some((13, 148, 136)),
            ..style.clone()
        };
        let msvg = draw(
            &l,
            &[],
            &plates,
            &nodes,
            &edges,
            &math,
            crate::theme::presets::theme_void(),
        )
        .render_svg_native_at(0.0, 0.0, l.w.ceil() as u32, l.h.ceil() as u32)
        .unwrap();
        assert!(msvg.contains("STIX Two Math"), "math font");
        assert!(msvg.contains("font-style=\"italic\""), "italic labels");
        // …but a constant stays upright
        let two = msvg.find(">2.0<").expect("the constant 2.0");
        let el = &msvg[msvg[..two].rfind('<').unwrap()..two];
        assert!(!el.contains("italic"), "upright constant: {el}");
        // the plate, and its label
        assert!(svg.contains("data-plate=\"i = 1..3\"") && svg.contains(">i = 1..3<"));
        // the observed circles' dark fill
        assert!(svg.contains(&format!(
            "fill=\"{}\"",
            hex(style.observed_fill).to_uppercase()
        )));
        // no two nodes overlap
        for (i, a) in l.nodes.iter().enumerate() {
            for b in &l.nodes[i + 1..] {
                let apart = (a.cx - b.cx).abs() >= (a.w + b.w) / 2.0
                    || (a.cy - b.cy).abs() >= (a.h + b.h) / 2.0;
                assert!(apart, "overlap");
            }
        }
        // the circle is round: rx is half its width
        let c = &l.nodes[1];
        assert!(svg.contains(&format!("rx=\"{:.2}\"", c.w / 2.0)), "{}", c.w);
    }

    /// An HMM over five slices with learned transition (A) and observation
    /// (B) matrices, in bands param / state / obs: the state chain is one
    /// straight row, each observation below its state.
    #[test]
    fn banded_hmm_keeps_the_state_chain_straight() {
        use crate::graph_layout::{banded, BandedParams};
        let t_n = 5;
        // (label, shape, band)
        let mut spec: Vec<(String, NodeShape, usize)> = vec![
            ("Dir".into(), NodeShape::Glyph, 0), // 0: prior on A
            ("A".into(), NodeShape::Square, 0),  // 1: = of A
            ("Dir".into(), NodeShape::Glyph, 0), // 2: prior on B
            ("B".into(), NodeShape::Square, 0),  // 3: = of B
            ("Cat".into(), NodeShape::Glyph, 1), // 4: initial state prior
            ("s₀".into(), NodeShape::Circle, 1), // 5
        ];
        let mut links: Vec<(usize, usize, &str)> =
            vec![(0, 1, "out"), (2, 3, "out"), (4, 5, "out")];
        let mut prev = 5;
        for t in 1..=t_n {
            let tr = spec.len();
            spec.push(("T".into(), NodeShape::Glyph, 1));
            let eq = spec.len();
            spec.push((format!("s{t}"), NodeShape::Square, 1));
            let ob = spec.len();
            spec.push(("T".into(), NodeShape::Glyph, 2));
            let x = spec.len();
            spec.push((format!("x{t}"), NodeShape::FilledCircle, 2));
            links.extend([
                (prev, tr, "in"),
                (1, tr, "a"),
                (tr, eq, "out"),
                (eq, ob, "in"),
                (3, ob, "a"),
                (ob, x, "out"),
            ]);
            prev = eq;
        }
        let sizes: Vec<NodeBox> = spec.iter().map(|(l, sh, _)| node_size(*sh, l)).collect();
        let gnodes: Vec<GraphNode> = spec
            .iter()
            .zip(&sizes)
            .map(|((_, _, b), s)| GraphNode {
                w: s.w,
                h: s.h,
                lane: Some(*b),
                ..Default::default()
            })
            .collect();
        let gedges: Vec<GraphEdge> = links
            .iter()
            .map(|&(a, b, _)| GraphEdge {
                from: a,
                to: b,
                label_w: None,
            })
            .collect();
        let l = banded(
            &gnodes,
            &gedges,
            &BandedParams {
                bands: 3,
                margin: 32.0,
                ..Default::default()
            },
        );
        // the chain: every transition and state node on one row
        let chain: Vec<usize> = (0..spec.len())
            .filter(|&i| spec[i].2 == 1 && i != 4)
            .collect();
        let y = l.nodes[chain[0]].cy;
        for &i in &chain {
            assert_eq!(l.nodes[i].cy, y, "{} off the row", spec[i].0);
        }
        // each observation factor straight below its state's = node
        for t in 0..t_n {
            let (eq, ob) = (6 + 4 * t + 1, 6 + 4 * t + 2);
            assert_eq!(l.nodes[eq].cx, l.nodes[ob].cx, "slice {t}");
            assert!(l.nodes[ob].cy > y);
        }
        // no edge passes through a box other than its own two ends
        let crosses = |(x0, y0): (f64, f64), (x1, y1): (f64, f64), n: &PlacedNode| {
            // sampled along the segment, the box shrunk by a pixel
            (0..=40).any(|k| {
                let t = k as f64 / 40.0;
                let (x, y) = (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t);
                (x - n.cx).abs() < n.w / 2.0 - 1.0 && (y - n.cy).abs() < n.h / 2.0 - 1.0
            })
        };
        for (k, &(a, b, _)) in links.iter().enumerate() {
            for w in l.edges[k].windows(2) {
                for (i, n) in l.nodes.iter().enumerate() {
                    if i != a && i != b {
                        assert!(
                            !crosses(w[0], w[1], n),
                            "edge {a}->{b} crosses {}",
                            spec[i].0
                        );
                    }
                }
            }
        }
        // nothing overlaps
        for (i, a) in l.nodes.iter().enumerate() {
            for b in &l.nodes[i + 1..] {
                assert!(
                    (a.cx - b.cx).abs() >= (a.w + b.w) / 2.0
                        || (a.cy - b.cy).abs() >= (a.h + b.h) / 2.0
                );
            }
        }
        let nodes: Vec<DrawNode> = spec
            .iter()
            .zip(&sizes)
            .enumerate()
            .map(|(i, ((_, shape, _), b))| DrawNode {
                id: format!("n{i}"),
                shape: *shape,
                lines: b.lines.clone(),
                stroke: (60, 60, 60),
                ..Default::default()
            })
            .collect();
        let edges: Vec<DrawEdge> = links
            .iter()
            .map(|&(_, b, iface)| DrawEdge {
                colour: (90, 90, 90),
                width: 1.2,
                undirected: true,
                label: Some(iface.to_string()),
                label_at: Some(if spec[b].1 == NodeShape::Glyph {
                    0.8
                } else {
                    0.2
                }),
                ..Default::default()
            })
            .collect();
        let lanes = ["param".to_string(), "state".to_string(), "obs".to_string()];
        let svg = draw(
            &l,
            &lanes,
            &[],
            &nodes,
            &edges,
            &GraphStyle::default(),
            crate::theme::presets::theme_void(),
        )
        .render_svg_native_at(0.0, 0.0, l.w.ceil() as u32, l.h.ceil() as u32)
        .unwrap();
        if let Ok(path) = std::env::var("GGPLOT_GRAPH_SVG") {
            std::fs::write(path, &svg).unwrap();
        }
        assert_eq!(svg.matches("data-lane=").count(), 3);
    }
}
