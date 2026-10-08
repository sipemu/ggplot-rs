//! Repelling text labels — `geom_text_repel` / `geom_label_repel` (R's
//! ggrepel).
//!
//! Labels start at their data point (plus an optional nudge) and are pushed
//! apart by a deterministic, seeded relaxation until they no longer overlap
//! each other or the labelled points; a spring pulls each label back towards
//! its point and every label stays inside the panel. Labels that still overlap
//! more than `max_overlaps` other labels/points are dropped (reported as a
//! render warning, like ggrepel's "unlabeled data points"). A segment connects
//! a label to its point when it ended up farther away than
//! `min_segment_length`.
//!
//! The layout runs in pixel space at draw time (text extents depend on the
//! rendered size), costs O(n log n + overlapping pairs) per iteration thanks to
//! a sweep over sorted box edges, and is bounded by `max_iter` and `max_time`.
//! Above `max_labels` labels the simulation is skipped (greedy non-overlapping
//! placement instead) with a warning.

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{
    DrawBackend, FontFace, LineStyle, Linetype, RectStyle, TextAnchor, TextStyle,
};
use crate::render::{Rect, RenderError};
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Axis along which labels may move.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RepelDirection {
    /// Move freely (default).
    #[default]
    Both,
    /// Only horizontally.
    X,
    /// Only vertically.
    Y,
}

/// Shared layout parameters of [`GeomTextRepel`] / [`GeomLabelRepel`].
#[derive(Clone, Debug)]
pub struct RepelParams {
    /// Padding (px) around each label's box when testing for overlaps
    /// (ggrepel `box.padding`).
    pub box_padding: f64,
    /// Padding (px) around each data point (ggrepel `point.padding`); also
    /// the gap left between a segment and its point.
    pub point_padding: f64,
    /// Radius (px) of the data points labels must avoid (match the point
    /// layer's size; `GeomPoint`'s default is 3).
    pub point_size: f64,
    /// Horizontal nudge of the starting position, in data units (numeric x).
    pub nudge_x: f64,
    /// Vertical nudge of the starting position, in data units (numeric y).
    pub nudge_y: f64,
    /// Strength of the repulsion between overlapping boxes.
    pub force: f64,
    /// Strength of the spring pulling a label back to its point.
    pub force_pull: f64,
    /// Maximum relaxation iterations.
    pub max_iter: usize,
    /// Wall-clock bound for the relaxation (a safety net — the layout is
    /// deterministic as long as it converges or hits `max_iter` first).
    /// Ignored on `wasm32`, which has no monotonic clock.
    pub max_time: std::time::Duration,
    /// Drop labels that still overlap more than this many other labels or
    /// points after the layout (ggrepel `max.overlaps`, default 10).
    pub max_overlaps: usize,
    /// Draw a segment when the label box is farther than this (px) from its
    /// point (ggrepel `min.segment.length`).
    pub min_segment_length: f64,
    /// Seed of the tie-breaking jitter; equal seeds give equal layouts.
    pub seed: u64,
    /// Movement axis.
    pub direction: RepelDirection,
    /// Above this many labels the force layout is skipped (it is O(n²) in
    /// the worst case) and labels are placed greedily, dropping overlaps.
    pub max_labels: usize,
    /// Segment colour; `None` = the label's text colour.
    pub segment_color: Option<(u8, u8, u8)>,
    /// Segment width (px).
    pub segment_width: f64,
}

impl Default for RepelParams {
    fn default() -> Self {
        RepelParams {
            box_padding: 3.0,
            point_padding: 2.0,
            point_size: 3.0,
            nudge_x: 0.0,
            nudge_y: 0.0,
            force: 1.0,
            force_pull: 1.0,
            max_iter: 2000,
            max_time: std::time::Duration::from_millis(500),
            max_overlaps: 10,
            min_segment_length: 12.0,
            seed: 42,
            direction: RepelDirection::Both,
            max_labels: 500,
            segment_color: None,
            segment_width: 0.6,
        }
    }
}

macro_rules! repel_builders {
    ($t:ty) => {
        impl $t {
            /// Padding (px) around label boxes when testing overlaps.
            pub fn box_padding(mut self, px: f64) -> Self {
                self.repel.box_padding = px;
                self
            }
            /// Padding (px) around the labelled points.
            pub fn point_padding(mut self, px: f64) -> Self {
                self.repel.point_padding = px;
                self
            }
            /// Radius (px) of the points to avoid.
            pub fn point_size(mut self, px: f64) -> Self {
                self.repel.point_size = px;
                self
            }
            /// Nudge the starting position (data units).
            pub fn nudge(mut self, x: f64, y: f64) -> Self {
                self.repel.nudge_x = x;
                self.repel.nudge_y = y;
                self
            }
            /// Repulsion strength.
            pub fn force(mut self, force: f64) -> Self {
                self.repel.force = force;
                self
            }
            /// Spring strength back to the point.
            pub fn force_pull(mut self, force: f64) -> Self {
                self.repel.force_pull = force;
                self
            }
            /// Maximum iterations.
            pub fn max_iter(mut self, n: usize) -> Self {
                self.repel.max_iter = n;
                self
            }
            /// Wall-clock bound of the layout.
            pub fn max_time(mut self, t: std::time::Duration) -> Self {
                self.repel.max_time = t;
                self
            }
            /// Drop labels overlapping more than `n` labels/points.
            pub fn max_overlaps(mut self, n: usize) -> Self {
                self.repel.max_overlaps = n;
                self
            }
            /// Minimum label–point distance (px) that gets a segment.
            pub fn min_segment_length(mut self, px: f64) -> Self {
                self.repel.min_segment_length = px;
                self
            }
            /// Seed of the deterministic tie-breaking jitter.
            pub fn seed(mut self, seed: u64) -> Self {
                self.repel.seed = seed;
                self
            }
            /// Restrict movement to one axis.
            pub fn direction(mut self, d: RepelDirection) -> Self {
                self.repel.direction = d;
                self
            }
            /// Label-count cap of the force layout.
            pub fn max_labels(mut self, n: usize) -> Self {
                self.repel.max_labels = n;
                self
            }
            /// Segment colour.
            pub fn segment_color(mut self, color: (u8, u8, u8)) -> Self {
                self.repel.segment_color = Some(color);
                self
            }
        }
    };
}

/// Text labels repelled away from each other and from their points
/// (`ggrepel::geom_text_repel`). Requires `x`, `y`, `label`; an empty / `NA`
/// label is not drawn but its point is still avoided (as in ggrepel).
pub struct GeomTextRepel {
    pub size: f64,
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub fontface: FontFace,
    pub repel: RepelParams,
}

impl Default for GeomTextRepel {
    fn default() -> Self {
        GeomTextRepel {
            size: 10.0,
            color: (0, 0, 0),
            alpha: 1.0,
            fontface: FontFace::Plain,
            repel: RepelParams::default(),
        }
    }
}

repel_builders!(GeomTextRepel);

/// Boxed labels repelled away from each other and from their points
/// (`ggrepel::geom_label_repel`).
pub struct GeomLabelRepel {
    pub size: f64,
    pub color: (u8, u8, u8),
    pub fill: (u8, u8, u8),
    pub alpha: f64,
    /// Inner padding (px) between text and box border.
    pub label_padding: f64,
    pub fontface: FontFace,
    pub repel: RepelParams,
}

impl Default for GeomLabelRepel {
    fn default() -> Self {
        GeomLabelRepel {
            size: 10.0,
            color: (0, 0, 0),
            fill: (255, 255, 255),
            alpha: 0.9,
            label_padding: 3.0,
            fontface: FontFace::Plain,
            repel: RepelParams::default(),
        }
    }
}

repel_builders!(GeomLabelRepel);

/// Axis-aligned label box: centre and half extents (px).
#[derive(Clone, Copy, Debug)]
struct LabelBox {
    cx: f64,
    cy: f64,
    hw: f64,
    hh: f64,
}

impl LabelBox {
    fn overlaps(&self, o: &LabelBox, pad: f64) -> bool {
        (self.cx - o.cx).abs() < self.hw + o.hw + 2.0 * pad
            && (self.cy - o.cy).abs() < self.hh + o.hh + 2.0 * pad
    }

    /// Distance from `(x, y)` to the box (0 inside).
    fn dist_to(&self, x: f64, y: f64) -> f64 {
        let dx = ((x - self.cx).abs() - self.hw).max(0.0);
        let dy = ((y - self.cy).abs() - self.hh).max(0.0);
        dx.hypot(dy)
    }

    /// Closest point of the box border/interior to `(x, y)`.
    fn closest(&self, x: f64, y: f64) -> (f64, f64) {
        (
            x.clamp(self.cx - self.hw, self.cx + self.hw),
            y.clamp(self.cy - self.hh, self.cy + self.hh),
        )
    }
}

/// Result of a layout: final boxes, which labels were kept, and how many
/// were dropped for exceeding `max_overlaps`.
pub(crate) struct RepelLayout {
    boxes: Vec<LabelBox>,
    kept: Vec<bool>,
    dropped: usize,
    /// The force layout was skipped because there were too many labels.
    capped: bool,
}

/// Lay out `labels` (anchor point, half extents incl. any label padding) so
/// they avoid each other and `points`, inside `panel`. Pure and
/// deterministic for a given `params.seed` (unless `max_time` cuts it short).
pub(crate) fn layout(
    anchors: &[(f64, f64)],
    targets: &[(f64, f64)],
    half: &[(f64, f64)],
    points: &[(f64, f64)],
    panel: &Rect,
    p: &RepelParams,
) -> RepelLayout {
    let n = anchors.len();
    let finite = |v: f64, d: f64| if v.is_finite() { v } else { d };
    let box_pad = finite(p.box_padding, 0.0).max(0.0);
    let pt_r = finite(p.point_size, 0.0).max(0.0) + finite(p.point_padding, 0.0).max(0.0);
    let force = finite(p.force, 1.0).clamp(0.0, 100.0);
    let pull = finite(p.force_pull, 1.0).clamp(0.0, 100.0);
    let (x0, x1) = (panel.x, panel.x + panel.width);
    let (y0, y1) = (panel.y, panel.y + panel.height);
    let clamp_box = |b: &mut LabelBox| {
        // A box wider than the panel is centred on it.
        b.cx = if 2.0 * b.hw >= x1 - x0 {
            (x0 + x1) / 2.0
        } else {
            b.cx.clamp(x0 + b.hw, x1 - b.hw)
        };
        b.cy = if 2.0 * b.hh >= y1 - y0 {
            (y0 + y1) / 2.0
        } else {
            b.cy.clamp(y0 + b.hh, y1 - b.hh)
        };
    };

    let mut rng = crate::rng::SplitMix64::new(p.seed);
    let mut boxes: Vec<LabelBox> = (0..n)
        .map(|i| {
            let mut b = LabelBox {
                cx: targets[i].0 + rng.range_f64(-0.5, 0.5),
                cy: targets[i].1 + rng.range_f64(-0.5, 0.5),
                hw: half[i].0,
                hh: half[i].1,
            };
            clamp_box(&mut b);
            b
        })
        .collect();

    let capped = n > p.max_labels;
    let (move_x, move_y) = match p.direction {
        RepelDirection::Both => (1.0, 1.0),
        RepelDirection::X => (1.0, 0.0),
        RepelDirection::Y => (0.0, 1.0),
    };

    // Points sorted by x for range queries.
    let mut pts: Vec<(f64, f64)> = points
        .iter()
        .copied()
        .filter(|(x, y)| x.is_finite() && y.is_finite())
        .collect();
    pts.sort_by(|a, b| a.0.total_cmp(&b.0));
    let points_near = |pts: &[(f64, f64)], lo: f64, hi: f64| {
        let start = pts.partition_point(|q| q.0 < lo);
        let end = pts.partition_point(|q| q.0 <= hi);
        start..end.max(start)
    };

    if !capped && n > 0 {
        #[cfg(not(target_arch = "wasm32"))]
        let started = std::time::Instant::now();
        let mut order: Vec<usize> = (0..n).collect();
        let mut delta = vec![(0.0f64, 0.0f64); n];
        let mut hit = vec![false; n];
        // The spring is active for the first three quarters; the rest only
        // separates, so the layout ends in a non-overlapping state whenever
        // one is reachable.
        let pull_until = p.max_iter - p.max_iter / 4;
        // Largest step per iteration, against overshooting when many
        // overlaps add up.
        let max_step = 12.0;
        for iter in 0..p.max_iter {
            #[cfg(not(target_arch = "wasm32"))]
            if iter % 16 == 15 && started.elapsed() >= p.max_time {
                break;
            }
            for d in delta.iter_mut() {
                *d = (0.0, 0.0);
            }
            hit.iter_mut().for_each(|h| *h = false);
            let mut overlaps = 0usize;

            // Box–box: sweep over boxes sorted by their left edge.
            order.sort_by(|&a, &b| {
                (boxes[a].cx - boxes[a].hw).total_cmp(&(boxes[b].cx - boxes[b].hw))
            });
            for (oi, &i) in order.iter().enumerate() {
                let bi = boxes[i];
                for &j in &order[oi + 1..] {
                    let bj = boxes[j];
                    if bj.cx - bj.hw - box_pad >= bi.cx + bi.hw + box_pad {
                        break;
                    }
                    if !bi.overlaps(&bj, box_pad) {
                        continue;
                    }
                    overlaps += 1;
                    hit[i] = true;
                    hit[j] = true;
                    let (dx, dy) = (bi.cx - bj.cx, bi.cy - bj.cy);
                    let ox = bi.hw + bj.hw + 2.0 * box_pad - dx.abs();
                    let oy = bi.hh + bj.hh + 2.0 * box_pad - dy.abs();
                    let sx = if dx == 0.0 {
                        tie(&mut rng)
                    } else {
                        dx.signum()
                    };
                    let sy = if dy == 0.0 {
                        tie(&mut rng)
                    } else {
                        dy.signum()
                    };
                    // Separate along the axis of least overlap (respecting the
                    // allowed direction).
                    let along_x = match p.direction {
                        RepelDirection::X => true,
                        RepelDirection::Y => false,
                        RepelDirection::Both => ox * move_x <= oy * move_y,
                    };
                    let k = 0.5 * force;
                    if along_x {
                        delta[i].0 += sx * ox * k;
                        delta[j].0 -= sx * ox * k;
                    } else {
                        delta[i].1 += sy * oy * k;
                        delta[j].1 -= sy * oy * k;
                    }
                }
            }

            let label_overlaps = overlaps;

            // Box–point: push labels off every data point they cover. The
            // final phase only separates labels from each other (a point
            // that cannot be avoided must not keep two labels overlapping).
            let point_force = if iter < pull_until { force } else { 0.0 };
            for i in 0..n {
                if point_force == 0.0 {
                    break;
                }
                let b = boxes[i];
                let r = pt_r + box_pad;
                for q in &pts[points_near(&pts, b.cx - b.hw - r, b.cx + b.hw + r)] {
                    let (dx, dy) = (b.cx - q.0, b.cy - q.1);
                    let ox = b.hw + r - dx.abs();
                    let oy = b.hh + r - dy.abs();
                    if ox <= 0.0 || oy <= 0.0 {
                        continue;
                    }
                    overlaps += 1;
                    hit[i] = true;
                    let sx = if dx == 0.0 {
                        tie(&mut rng)
                    } else {
                        dx.signum()
                    };
                    let sy = if dy == 0.0 {
                        tie(&mut rng)
                    } else {
                        dy.signum()
                    };
                    let along_x = match p.direction {
                        RepelDirection::X => true,
                        RepelDirection::Y => false,
                        RepelDirection::Both => ox <= oy,
                    };
                    if along_x {
                        delta[i].0 += sx * ox * point_force;
                    } else {
                        delta[i].1 += sy * oy * point_force;
                    }
                }
            }

            // Spring back towards the target, then move and clamp.
            let mut moved = 0.0f64;
            for i in 0..n {
                let b = &mut boxes[i];
                // Pull back only labels that are currently free, so the spring
                // never holds two labels in a residual overlap.
                let k = if iter < pull_until && !hit[i] {
                    0.1 * pull
                } else {
                    0.0
                };
                let sx = (targets[i].0 - b.cx) * k;
                let sy = (targets[i].1 - b.cy) * k;
                let cap = |v: f64| {
                    if v.is_finite() {
                        v.clamp(-max_step, max_step)
                    } else {
                        0.0
                    }
                };
                let (mx, my) = (cap(delta[i].0 + sx) * move_x, cap(delta[i].1 + sy) * move_y);
                let (px, py) = (b.cx, b.cy);
                b.cx += mx;
                b.cy += my;
                clamp_box(b);
                moved = moved.max((b.cx - px).abs() + (b.cy - py).abs());
            }
            if (overlaps == 0 && moved < 0.05) || (label_overlaps == 0 && iter >= pull_until) {
                break;
            }
        }
    }

    // Drop labels that still overlap too much: each label counts overlaps
    // with already-kept labels and with points other than its own.
    let mut kept = vec![false; n];
    let mut dropped = 0usize;
    let mut kept_boxes: Vec<LabelBox> = Vec::new();
    let limit = if capped { 0 } else { p.max_overlaps };
    for i in 0..n {
        let b = boxes[i];
        let mut count = kept_boxes.iter().filter(|o| b.overlaps(o, 0.0)).count();
        // Without a layout, covered points are not held against a label.
        if count <= limit && !capped {
            let r = pt_r;
            count += pts[points_near(&pts, b.cx - b.hw - r, b.cx + b.hw + r)]
                .iter()
                .filter(|q| {
                    !(q.0 == anchors[i].0 && q.1 == anchors[i].1)
                        && (q.0 - b.cx).abs() < b.hw + r
                        && (q.1 - b.cy).abs() < b.hh + r
                })
                .count();
        }
        if count > limit {
            dropped += 1;
        } else {
            kept[i] = true;
            kept_boxes.push(b);
        }
    }

    RepelLayout {
        boxes,
        kept,
        dropped,
        capped,
    }
}

/// Deterministic ±1 for coincident centres.
fn tie(rng: &mut crate::rng::SplitMix64) -> f64 {
    if rng.next_u64() & 1 == 0 {
        -1.0
    } else {
        1.0
    }
}

/// One label to place.
struct Item {
    row: usize,
    anchor: (f64, f64),
    target: (f64, f64),
    text: String,
}

/// Shared draw routine; `label` = `Some((fill, padding))` draws boxes.
#[allow(clippy::too_many_arguments)]
fn draw_repel(
    name: &str,
    data: &DataFrame,
    coord: &dyn Coord,
    scales: &ScaleSet,
    backend: &mut dyn DrawBackend,
    size: f64,
    color: (u8, u8, u8),
    alpha: f64,
    face: FontFace,
    label: Option<((u8, u8, u8), f64)>,
    p: &RepelParams,
) -> Result<(), RenderError> {
    let x_col = data
        .column("x")
        .ok_or(RenderError::MissingAesthetic("x".into()))?;
    let y_col = data
        .column("y")
        .ok_or(RenderError::MissingAesthetic("y".into()))?;
    let label_col = data
        .column("label")
        .ok_or(RenderError::MissingAesthetic("label".into()))?;
    let color_col = data.column("color");
    let fill_col = data.column("fill");

    let panel = backend.plot_area();
    let x_scale = scales.get(&Aesthetic::X);
    let y_scale = scales.get(&Aesthetic::Y);
    let size = if size.is_finite() && size > 0.0 {
        size
    } else {
        10.0
    };

    let to_px = |xv: &Value, yv: &Value| -> (f64, f64) {
        let nx = x_scale.map(|s| s.map(xv)).unwrap_or(0.5);
        let ny = y_scale.map(|s| s.map(yv)).unwrap_or(0.5);
        coord.transform((nx, ny), &panel)
    };
    let nudged = |v: &Value, d: f64, discrete: bool| -> Value {
        match v.as_f64() {
            Some(f) if d != 0.0 && d.is_finite() && !discrete => Value::Float(f + d),
            _ => v.clone(),
        }
    };
    let x_discrete = x_scale.map(|s| s.is_discrete()).unwrap_or(false);
    let y_discrete = y_scale.map(|s| s.is_discrete()).unwrap_or(false);

    let mut points = Vec::with_capacity(data.nrows());
    let mut items: Vec<Item> = Vec::new();
    for i in 0..data.nrows() {
        if x_col[i].is_na() || y_col[i].is_na() {
            continue;
        }
        let anchor = to_px(&x_col[i], &y_col[i]);
        if !(anchor.0.is_finite() && anchor.1.is_finite()) {
            continue;
        }
        points.push(anchor);
        let text = if label_col[i].is_na() {
            String::new()
        } else {
            label_col[i].to_group_key()
        };
        if text.trim().is_empty() {
            continue;
        }
        let target = to_px(
            &nudged(&x_col[i], p.nudge_x, x_discrete),
            &nudged(&y_col[i], p.nudge_y, y_discrete),
        );
        let target = if target.0.is_finite() && target.1.is_finite() {
            target
        } else {
            anchor
        };
        items.push(Item {
            row: i,
            anchor,
            target,
            text,
        });
    }
    if items.is_empty() {
        return Ok(());
    }

    let pad = label.map(|(_, pd)| pd.max(0.0)).unwrap_or(0.0);
    let half: Vec<(f64, f64)> = items
        .iter()
        .map(|it| {
            (
                it.text.chars().count() as f64 * size * 0.3 + pad,
                size * 0.5 + pad,
            )
        })
        .collect();
    let anchors: Vec<(f64, f64)> = items.iter().map(|it| it.anchor).collect();
    let targets: Vec<(f64, f64)> = items.iter().map(|it| it.target).collect();
    let lay = layout(&anchors, &targets, &half, &points, &panel, p);

    if lay.capped {
        backend.warn(format!(
            "geom_{name}: {} labels exceed the repel limit of {}; placed without \
             repulsion (overlapping labels dropped)",
            items.len(),
            p.max_labels
        ));
    }
    if lay.dropped > 0 {
        backend.warn(format!(
            "geom_{name}: {} unlabeled data point{} (too many overlaps). Consider \
             increasing max_overlaps",
            lay.dropped,
            if lay.dropped == 1 { "" } else { "s" }
        ));
    }

    let seg_min = if p.min_segment_length.is_finite() {
        p.min_segment_length.max(0.0)
    } else {
        f64::INFINITY
    };
    let alpha = if alpha.is_finite() {
        alpha.clamp(0.0, 1.0)
    } else {
        1.0
    };

    // Segments first so labels paint over them.
    super::clear_mark(backend);
    for (k, it) in items.iter().enumerate() {
        if !lay.kept[k] {
            continue;
        }
        let b = lay.boxes[k];
        let d = b.dist_to(it.anchor.0, it.anchor.1);
        if d <= seg_min || d <= 0.0 {
            continue;
        }
        let (ex, ey) = b.closest(it.anchor.0, it.anchor.1);
        // Leave a gap of `point_padding` at the point end.
        let gap = if p.point_padding.is_finite() {
            p.point_padding.max(0.0)
        } else {
            0.0
        };
        let (dx, dy) = (ex - it.anchor.0, ey - it.anchor.1);
        let len = dx.hypot(dy);
        if len <= gap {
            continue;
        }
        let start = (it.anchor.0 + dx / len * gap, it.anchor.1 + dy / len * gap);
        let c = p
            .segment_color
            .unwrap_or_else(|| row_color(scales, color_col, it.row, color));
        backend.draw_line(
            &[start, (ex, ey)],
            &LineStyle {
                color: c,
                width: if p.segment_width.is_finite() {
                    p.segment_width.max(0.0)
                } else {
                    0.6
                },
                alpha,
                linetype: Linetype::Solid,
            },
        )?;
    }

    for (k, it) in items.iter().enumerate() {
        if !lay.kept[k] {
            continue;
        }
        let b = lay.boxes[k];
        let c = row_color(scales, color_col, it.row, color);
        if let Some((fill, _)) = label {
            let f = fill_col
                .and_then(|fc| scales.map_color(&Aesthetic::Fill, &fc[it.row]))
                .unwrap_or(fill);
            super::set_mark(
                backend,
                Some(it.text.clone()),
                Some(super::tip_value(&x_col[it.row])),
                super::series_key(data, it.row),
                super::measured_value(data, it.row),
            );
            backend.draw_rect(
                (b.cx - b.hw, b.cy - b.hh),
                (b.cx + b.hw, b.cy + b.hh),
                &RectStyle {
                    fill: Some(f),
                    stroke: Some(c),
                    stroke_width: 0.5,
                    alpha,
                    clip: true,
                },
            )?;
            super::clear_mark(backend);
        }
        backend.draw_text(
            &it.text,
            (b.cx, b.cy),
            &TextStyle {
                color: c,
                size,
                anchor: TextAnchor::Middle,
                angle: 0.0,
                family: None,
                face,
            },
        )?;
    }
    Ok(())
}

fn row_color(
    scales: &ScaleSet,
    color_col: Option<&[Value]>,
    row: usize,
    fallback: (u8, u8, u8),
) -> (u8, u8, u8) {
    color_col
        .and_then(|cc| scales.map_color(&Aesthetic::Color, &cc[row]))
        .unwrap_or(fallback)
}

impl Geom for GeomTextRepel {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        draw_repel(
            "text_repel",
            data,
            coord,
            scales,
            backend,
            self.size,
            self.color,
            self.alpha,
            self.fontface,
            None,
            &self.repel,
        )
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y, Aesthetic::Label]
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
        "text_repel"
    }
}

impl Geom for GeomLabelRepel {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        draw_repel(
            "label_repel",
            data,
            coord,
            scales,
            backend,
            self.size,
            self.color,
            self.alpha,
            self.fontface,
            Some((self.fill, self.label_padding)),
            &self.repel,
        )
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y, Aesthetic::Label]
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
        "label_repel"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel() -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 300.0,
        }
    }

    fn overlapping(lay: &RepelLayout) -> usize {
        let mut c = 0;
        for i in 0..lay.boxes.len() {
            for j in i + 1..lay.boxes.len() {
                if lay.kept[i] && lay.kept[j] && lay.boxes[i].overlaps(&lay.boxes[j], 0.0) {
                    c += 1;
                }
            }
        }
        c
    }

    #[test]
    fn coincident_labels_are_separated_deterministically() {
        let anchors = vec![(200.0, 150.0); 6];
        let half = vec![(20.0, 6.0); 6];
        let p = RepelParams::default();
        let a = layout(&anchors, &anchors, &half, &anchors, &panel(), &p);
        let b = layout(&anchors, &anchors, &half, &anchors, &panel(), &p);
        assert_eq!(overlapping(&a), 0);
        assert_eq!(a.dropped, 0);
        for (x, y) in a.boxes.iter().zip(&b.boxes) {
            assert_eq!((x.cx, x.cy), (y.cx, y.cy), "same seed, same layout");
        }
        let c = layout(
            &anchors,
            &anchors,
            &half,
            &anchors,
            &panel(),
            &RepelParams {
                seed: 7,
                ..RepelParams::default()
            },
        );
        assert_eq!(overlapping(&c), 0);
    }

    #[test]
    fn labels_avoid_points_and_stay_inside() {
        // A dense cloud near the corner.
        let anchors: Vec<(f64, f64)> = (0..30)
            .map(|i| (5.0 + (i % 6) as f64 * 3.0, 5.0 + (i / 6) as f64 * 3.0))
            .collect();
        let half = vec![(15.0, 5.0); anchors.len()];
        let lay = layout(
            &anchors,
            &anchors,
            &half,
            &anchors,
            &panel(),
            &RepelParams {
                max_overlaps: usize::MAX,
                ..RepelParams::default()
            },
        );
        let pa = panel();
        for b in &lay.boxes {
            assert!(b.cx - b.hw >= pa.x - 1e-9 && b.cx + b.hw <= pa.x + pa.width + 1e-9);
            assert!(b.cy - b.hh >= pa.y - 1e-9 && b.cy + b.hh <= pa.y + pa.height + 1e-9);
        }
        assert_eq!(overlapping(&lay), 0, "all 30 labels separated");
    }

    #[test]
    fn max_overlaps_zero_drops_what_cannot_be_placed() {
        // 40 labels in a panel that only fits a handful.
        let tiny = Rect {
            x: 0.0,
            y: 0.0,
            width: 60.0,
            height: 30.0,
        };
        let anchors = vec![(30.0, 15.0); 40];
        let half = vec![(20.0, 6.0); 40];
        let lay = layout(
            &anchors,
            &anchors,
            &half,
            &[],
            &tiny,
            &RepelParams {
                max_overlaps: 0,
                ..RepelParams::default()
            },
        );
        assert!(lay.dropped > 0);
        assert_eq!(overlapping(&lay), 0);
        assert_eq!(lay.kept.iter().filter(|k| **k).count() + lay.dropped, 40);
    }

    #[test]
    fn direction_y_only_moves_vertically() {
        let anchors = vec![(100.0, 150.0), (101.0, 150.0), (102.0, 150.0)];
        let half = vec![(20.0, 6.0); 3];
        let lay = layout(
            &anchors,
            &anchors,
            &half,
            &[],
            &panel(),
            &RepelParams {
                direction: RepelDirection::Y,
                ..RepelParams::default()
            },
        );
        for (b, a) in lay.boxes.iter().zip(&anchors) {
            assert!((b.cx - a.0).abs() <= 0.5, "x stayed put");
        }
        assert_eq!(overlapping(&lay), 0);
    }

    #[test]
    fn cap_skips_simulation() {
        let anchors: Vec<(f64, f64)> = (0..20).map(|i| (i as f64, 10.0)).collect();
        let half = vec![(5.0, 3.0); 20];
        let lay = layout(
            &anchors,
            &anchors,
            &half,
            &anchors,
            &panel(),
            &RepelParams {
                max_labels: 5,
                ..RepelParams::default()
            },
        );
        assert!(lay.capped);
        assert_eq!(overlapping(&lay), 0);
    }

    #[test]
    fn non_finite_params_do_not_hang_or_panic() {
        let anchors = vec![(10.0, 10.0), (10.0, 10.0)];
        let half = vec![(5.0, 3.0); 2];
        let lay = layout(
            &anchors,
            &anchors,
            &half,
            &[(f64::NAN, 1.0)],
            &panel(),
            &RepelParams {
                force: f64::NAN,
                force_pull: f64::INFINITY,
                box_padding: f64::NAN,
                ..RepelParams::default()
            },
        );
        for b in &lay.boxes {
            assert!(b.cx.is_finite() && b.cy.is_finite());
        }
    }
}
