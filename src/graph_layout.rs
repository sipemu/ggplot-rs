//! Layered (Sugiyama-style) layout of a directed graph of boxes — nodes in
//! layers (stages) along one axis, ordered across it to keep crossings low.
//!
//! The steps: cycles broken by a DFS in input order; ranks by longest path
//! (a pinned layer wins; sources pulled up to just before their nearest
//! successor); a dummy vertex per layer a long edge passes; barycenter sweeps
//! plus adjacent transposition, the fewest crossings kept (counted per layer
//! pair with a Fenwick tree); a weighted least-squares (isotonic) placement
//! along each layer; ports spread along a box's side; optional swimlanes as
//! bands. A graph whose every node is pinned in both axes is drawn on a grid
//! where it is placed, with straight edges.
//!
//! Node sizes come from the caller (the box contents are the drawing's
//! business); the result is in px with the origin at the top-left margin.
//! Moved here from anofox-visualization's `::DIAGRAM` (sipemu/ggplot-rs#497,
//! DataZooDE/anofox-visualization#1153).

/// Which way the layers run.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Direction {
    /// Layers left to right (the default).
    #[default]
    LeftRight,
    /// Layers top to bottom.
    TopBottom,
}

/// A node: its box size, its swimlane, and optional pins (a layer, and a
/// position across it; both pinned on every node → the grid layout).
#[derive(Clone, Debug, Default)]
pub struct GraphNode {
    pub w: f64,
    pub h: f64,
    /// Index of the node's swimlane (< [`LayeredParams::lanes`]), if any.
    pub lane: Option<usize>,
    /// Pinned layer and pinned position (grid units in the explicit layout).
    pub pin: (Option<f64>, Option<f64>),
}

/// A directed edge; `label_w` is the width (px) of its label, if it has one:
/// left to right, the gap between two layers is at least as wide as the
/// labels that cross it.
#[derive(Clone, Debug, Default)]
pub struct GraphEdge {
    pub from: usize,
    pub to: usize,
    pub label_w: Option<f64>,
}

/// Spacing and options; the defaults are anofox-visualization's diagram.
#[derive(Clone, Debug)]
pub struct LayeredParams {
    pub direction: Direction,
    /// Number of swimlanes (nodes carry the index).
    pub lanes: usize,
    pub margin: f64,
    /// Arrowhead length (an edge label's gap leaves room for two).
    pub arrow: f64,
    /// A self-loop's reach beyond its box.
    pub loop_size: f64,
    /// Extra space between swimlanes, and a lane's label strip.
    pub lane_gap: f64,
    pub lane_head: f64,
}

impl Default for LayeredParams {
    fn default() -> Self {
        LayeredParams {
            direction: Direction::LeftRight,
            lanes: 0,
            margin: 10.0,
            arrow: 7.0,
            loop_size: 16.0,
            lane_gap: 22.0,
            lane_head: 16.0,
        }
    }
}

/// A placed node: centre, size, layer, and rank in reading order (layer by
/// layer, then along it).
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedNode {
    pub cx: f64,
    pub cy: f64,
    pub w: f64,
    pub h: f64,
    pub layer: usize,
    pub order: usize,
}

/// The laid-out graph.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphLayout {
    pub w: f64,
    pub h: f64,
    pub nodes: Vec<PlacedNode>,
    /// Per edge (input order): its points, source → target.
    pub edges: Vec<Vec<(f64, f64)>>,
    /// Edges drawn against the layer order (they close a cycle).
    pub back: Vec<bool>,
    pub layers: usize,
    /// Swimlane bands: (lane, x0, y0, x1, y1).
    pub bands: Vec<(usize, f64, f64, f64, f64)>,
}

/// Lay out `nodes` and `edges` in layers (see the module docs).
pub fn layered(nodes: &[GraphNode], edges: &[GraphEdge], p: &LayeredParams) -> GraphLayout {
    let explicit =
        !nodes.is_empty() && nodes.iter().all(|n| n.pin.0.is_some() && n.pin.1.is_some());
    if explicit {
        layout_explicit(nodes, edges, p)
    } else {
        layout(nodes, edges, p)
    }
}

/// A DAG edge: `(u, v, edge index, reversed)`.
type DagEdge = (usize, usize, usize, bool);

/// The cycle-free orientation (a DFS in input order reverses every edge
/// that closes a cycle), and the self-loops (edge indices).
fn break_cycles(n: usize, edges: &[GraphEdge]) -> (Vec<DagEdge>, Vec<usize>) {
    let mut out: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n];
    let mut loops = Vec::new();
    for (k, e) in edges.iter().enumerate() {
        if e.from == e.to {
            loops.push(k);
        } else {
            out[e.from].push((e.to, k));
        }
    }
    let mut state = vec![0u8; n]; // 0 new, 1 on the DFS stack, 2 done
    let mut rev = vec![false; edges.len()];
    for s in 0..n {
        if state[s] != 0 {
            continue;
        }
        state[s] = 1;
        let mut stack = vec![(s, 0usize)];
        while let Some(top) = stack.last_mut() {
            let v = top.0;
            if top.1 < out[v].len() {
                let (w, k) = out[v][top.1];
                top.1 += 1;
                match state[w] {
                    0 => {
                        state[w] = 1;
                        stack.push((w, 0));
                    }
                    1 => rev[k] = true,
                    _ => {}
                }
            } else {
                state[v] = 2;
                stack.pop();
            }
        }
    }
    let dag = edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.from != e.to)
        .map(|(k, e)| {
            if rev[k] {
                (e.to, e.from, k, true)
            } else {
                (e.from, e.to, k, false)
            }
        })
        .collect();
    (dag, loops)
}

/// `keys` ordered by `(primary, secondary, index)`, as the indices.
fn ordered(mut keys: Vec<(f64, f64, usize)>) -> Vec<usize> {
    keys.sort_unstable_by(|a, b| {
        a.0.total_cmp(&b.0)
            .then(a.1.total_cmp(&b.1))
            .then(a.2.cmp(&b.2))
    });
    keys.into_iter().map(|k| k.2).collect()
}

/// Crossings between two adjacent layers (Barth–Mutzel–Jünger: the edges by
/// their upper end, inversions of the lower ends counted with a Fenwick tree).
fn layer_crossings(upper: &[usize], succ: &[Vec<usize>], pos: &[usize], lower_len: usize) -> u64 {
    let mut ends: Vec<(usize, usize)> = Vec::new();
    for &u in upper {
        for &v in &succ[u] {
            ends.push((pos[u], pos[v]));
        }
    }
    let lower: Vec<usize> = ordered(
        ends.iter()
            .enumerate()
            .map(|(i, &(u, v))| (u as f64, v as f64, i))
            .collect(),
    )
    .into_iter()
    .map(|i| ends[i].1)
    .collect();
    let mut tree = vec![0u64; lower_len + 1];
    let mut c = 0u64;
    for (seen, &p) in lower.iter().enumerate() {
        let mut i = p + 1;
        let mut le = 0u64;
        while i > 0 {
            le += tree[i];
            i -= 1 << i.trailing_zeros();
        }
        c += seen as u64 - le;
        let mut i = p + 1;
        while i <= lower_len {
            tree[i] += 1;
            i += 1 << i.trailing_zeros();
        }
    }
    c
}

/// Isotonic placement: positions as close as possible (weighted least
/// squares) to `want`, in the given order, at least `gap[i]` after the
/// previous one.
fn place(want: &[f64], weight: &[f64], gap: &[f64]) -> Vec<f64> {
    let n = want.len();
    let mut off = vec![0.0; n];
    for i in 1..n {
        off[i] = off[i - 1] + gap[i];
    }
    let mut blocks: Vec<(f64, f64, usize)> = Vec::new();
    for i in 0..n {
        blocks.push((weight[i] * (want[i] - off[i]), weight[i], 1));
        while blocks.len() >= 2 {
            let (b, a) = (blocks[blocks.len() - 1], blocks[blocks.len() - 2]);
            if a.0 / a.1 > b.0 / b.1 {
                blocks.pop();
                let last = blocks.last_mut().expect("two blocks");
                *last = (a.0 + b.0, a.1 + b.1, a.2 + b.2);
            } else {
                break;
            }
        }
    }
    let mut out = Vec::with_capacity(n);
    for (s, w, c) in blocks {
        for _ in 0..c {
            let i = out.len();
            out.push(s / w + off[i]);
        }
    }
    out
}

/// Where a line from a box's centre towards `to` leaves the box.
fn exit(p: &PlacedNode, to: (f64, f64)) -> (f64, f64) {
    let (dx, dy) = (to.0 - p.cx, to.1 - p.cy);
    let sx = if dx != 0.0 {
        p.w / 2.0 / dx.abs()
    } else {
        f64::INFINITY
    };
    let sy = if dy != 0.0 {
        p.h / 2.0 / dy.abs()
    } else {
        f64::INFINITY
    };
    let k = sx.min(sy);
    if k.is_finite() {
        (p.cx + dx * k, p.cy + dy * k)
    } else {
        (p.cx, p.cy)
    }
}

fn straight(pn: &[PlacedNode], u: usize, v: usize) -> Vec<(f64, f64)> {
    let (a, b) = (&pn[u], &pn[v]);
    vec![exit(a, (b.cx, b.cy)), exit(b, (a.cx, a.cy))]
}

/// A self-loop over (left to right) or beside (top to bottom) its box.
fn self_loop(p: &PlacedNode, lr: bool, reach: f64) -> Vec<(f64, f64)> {
    if lr {
        let top = p.cy - p.h / 2.0;
        vec![
            (p.cx + p.w / 4.0, top),
            (p.cx + p.w / 4.0, top - reach),
            (p.cx - p.w / 4.0, top - reach),
            (p.cx - p.w / 4.0, top),
        ]
    } else {
        let right = p.cx + p.w / 2.0;
        vec![
            (right, p.cy - p.h / 4.0),
            (right + reach, p.cy - p.h / 4.0),
            (right + reach, p.cy + p.h / 4.0),
            (right, p.cy + p.h / 4.0),
        ]
    }
}

/// Every node pinned in both axes: a grid of the widest / tallest box.
fn layout_explicit(nodes: &[GraphNode], edges: &[GraphEdge], p: &LayeredParams) -> GraphLayout {
    let sx = nodes.iter().map(|b| b.w).fold(0.0, f64::max) + 48.0;
    let sy = nodes.iter().map(|b| b.h).fold(0.0, f64::max) + 24.0;
    let pin = |v: usize| (nodes[v].pin.0.unwrap_or(0.0), nodes[v].pin.1.unwrap_or(0.0));
    let mut xs: Vec<f64> = (0..nodes.len()).map(|v| pin(v).0).collect();
    xs.sort_by(f64::total_cmp);
    xs.dedup();
    let reading = ordered((0..nodes.len()).map(|v| (pin(v).0, pin(v).1, v)).collect());
    let mut pn: Vec<PlacedNode> = nodes
        .iter()
        .enumerate()
        .map(|(v, b)| PlacedNode {
            cx: pin(v).0 * sx,
            cy: pin(v).1 * sy,
            w: b.w,
            h: b.h,
            layer: xs.iter().position(|&x| x == pin(v).0).unwrap_or(0),
            order: 0,
        })
        .collect();
    for (i, &v) in reading.iter().enumerate() {
        pn[v].order = i;
    }
    let lr = p.direction == Direction::LeftRight;
    let pts = edges
        .iter()
        .map(|e| {
            if e.from == e.to {
                self_loop(&pn[e.from], lr, p.loop_size)
            } else {
                straight(&pn, e.from, e.to)
            }
        })
        .collect();
    finish(
        pn,
        pts,
        vec![false; edges.len()],
        xs.len().max(1),
        Vec::new(),
        p.margin,
    )
}

fn layout(nodes: &[GraphNode], edges: &[GraphEdge], p: &LayeredParams) -> GraphLayout {
    let n = nodes.len();
    let (dag, loops) = break_cycles(n, edges);
    let lr = p.direction == Direction::LeftRight;
    let along = |b: &GraphNode| if lr { b.w } else { b.h };
    let across = |b: &GraphNode| if lr { b.h } else { b.w };
    let (rank_gap, node_gap) = if lr { (64.0, 20.0) } else { (48.0, 28.0) };
    let (lane_gap, lane_head) = (p.lane_gap, p.lane_head);

    // ranks: longest path in topological order (input order first), then
    // sources pulled up to just before their nearest successor
    let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut succs: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(u, v, _, _) in &dag {
        preds[v].push(u);
        succs[u].push(v);
    }
    let mut indeg: Vec<usize> = preds.iter().map(Vec::len).collect();
    let mut done = vec![false; n];
    let mut topo = Vec::with_capacity(n);
    while let Some(v) = (0..n).find(|&v| !done[v] && indeg[v] == 0) {
        done[v] = true;
        topo.push(v);
        for &w in &succs[v] {
            indeg[w] -= 1;
        }
    }
    // a pinned layer wins over the longest path
    let pinned = |v: usize| {
        nodes[v]
            .pin
            .0
            .map(|x| x.round().clamp(0.0, 1000.0) as usize)
    };
    let any_pin = (0..n).any(|v| pinned(v).is_some());
    let mut rank = vec![0usize; n];
    for &v in &topo {
        rank[v] =
            pinned(v).unwrap_or_else(|| preds[v].iter().map(|&q| rank[q] + 1).max().unwrap_or(0));
    }
    for &v in topo.iter().rev() {
        if preds[v].is_empty() && pinned(v).is_none() {
            if let Some(m) = succs[v].iter().map(|&s| rank[s]).min() {
                rank[v] = rank[v].max(m.saturating_sub(1));
            }
        }
    }
    // drop empty ranks (pinned layers keep their numbers)
    let mut used = vec![false; rank.iter().max().map_or(1, |m| m + 1)];
    for &r in &rank {
        used[r] = true;
    }
    let mut renumber = vec![0usize; used.len()];
    let mut nl = 0;
    for (r, u) in used.iter().enumerate() {
        renumber[r] = nl;
        nl += usize::from(*u || any_pin);
    }
    for r in &mut rank {
        *r = renumber[*r];
    }
    let nl = nl.max(1);
    // edges the pins make run within or against the layers: drawn straight
    let (dag, free): (Vec<DagEdge>, Vec<DagEdge>) =
        dag.into_iter().partition(|&(u, v, _, _)| rank[v] > rank[u]);

    // vertices: the nodes, then one dummy per layer an edge passes
    let mut vrank: Vec<usize> = rank.clone();
    let mut vsucc: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut vpred: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut chains: Vec<Vec<usize>> = Vec::with_capacity(dag.len());
    for &(u, v, _, _) in &dag {
        let mut chain = vec![u];
        let mut prev = u;
        for r in rank[u] + 1..rank[v] {
            let dmy = vrank.len();
            vrank.push(r);
            vsucc.push(Vec::new());
            vpred.push(Vec::new());
            vsucc[prev].push(dmy);
            vpred[dmy].push(prev);
            chain.push(dmy);
            prev = dmy;
        }
        vsucc[prev].push(v);
        vpred[v].push(prev);
        chain.push(v);
        chains.push(chain);
    }
    let nv = vrank.len();
    let dummy = |v: usize| v >= n;
    // a vertex's swimlane (unlaned nodes last; a dummy runs in its source's)
    let mut vlane: Vec<usize> = (0..n).map(|v| nodes[v].lane.unwrap_or(p.lanes)).collect();
    vlane.resize(nv, 0);
    for ch in &chains {
        for &dm in &ch[1..ch.len() - 1] {
            vlane[dm] = vlane[ch[0]];
        }
    }
    let laned = p.lanes > 0;

    // initial order: DFS from the vertices by (rank, index)
    let mut layers: Vec<Vec<usize>> = vec![Vec::new(); nl];
    let mut seen = vec![false; nv];
    for s in ordered((0..nv).map(|v| (vrank[v] as f64, 0.0, v)).collect()) {
        if seen[s] {
            continue;
        }
        let mut stack = vec![s];
        while let Some(v) = stack.pop() {
            if seen[v] {
                continue;
            }
            seen[v] = true;
            layers[vrank[v]].push(v);
            for &w in vsucc[v].iter().rev() {
                if !seen[w] {
                    stack.push(w);
                }
            }
        }
    }
    if laned {
        for l in &mut layers {
            *l = ordered(
                l.iter()
                    .enumerate()
                    .map(|(i, &v)| (vlane[v] as f64, i as f64, v))
                    .collect(),
            );
        }
    }
    let mut pos = vec![0usize; nv];
    let index = |layers: &[Vec<usize>], pos: &mut [usize]| {
        for l in layers {
            for (i, &v) in l.iter().enumerate() {
                pos[v] = i;
            }
        }
    };
    index(&layers, &mut pos);
    let crossings = |layers: &[Vec<usize>], pos: &[usize]| -> u64 {
        (0..layers.len().saturating_sub(1))
            .map(|r| layer_crossings(&layers[r], &vsucc, pos, layers[r + 1].len()))
            .sum()
    };

    // crossing reduction: barycenter sweeps + adjacent transposition, best kept
    let mut best = layers.clone();
    let mut best_c = crossings(&layers, &pos);
    for it in 0..8 {
        if best_c == 0 {
            break;
        }
        let down = it % 2 == 0;
        let rs: Vec<usize> = if down {
            (1..nl).collect()
        } else {
            (0..nl.saturating_sub(1)).rev().collect()
        };
        for r in rs {
            let nb = if down { &vpred } else { &vsucc };
            let keyed: Vec<(f64, f64, usize)> = layers[r]
                .iter()
                .map(|&v| {
                    let k = if nb[v].is_empty() {
                        pos[v] as f64
                    } else {
                        nb[v].iter().map(|&w| pos[w] as f64).sum::<f64>() / nb[v].len() as f64
                    };
                    (vlane[v] as f64 * 1e6 + k, pos[v] as f64, v)
                })
                .collect();
            layers[r] = ordered(keyed);
            for (i, &v) in layers[r].iter().enumerate() {
                pos[v] = i;
            }
        }
        for _ in 0..4 {
            let mut better = false;
            for layer in layers.iter_mut() {
                for i in 0..layer.len().saturating_sub(1) {
                    let (a, b) = (layer[i], layer[i + 1]);
                    if vlane[a] != vlane[b] {
                        continue;
                    }
                    let pair = |x: usize, y: usize| -> usize {
                        let c = |l: &[Vec<usize>]| {
                            l[x].iter()
                                .flat_map(|&q| l[y].iter().map(move |&r| (q, r)))
                                .filter(|&(q, r)| pos[q] > pos[r])
                                .count()
                        };
                        c(&vpred) + c(&vsucc)
                    };
                    if pair(b, a) < pair(a, b) {
                        layer.swap(i, i + 1);
                        pos[a] = i + 1;
                        pos[b] = i;
                        better = true;
                    }
                }
            }
            if !better {
                break;
            }
        }
        let c = crossings(&layers, &pos);
        if c < best_c {
            best_c = c;
            best = layers.clone();
        }
    }
    let layers = best;
    index(&layers, &mut pos);

    // positions across each layer
    let size = |v: usize| if dummy(v) { 0.0 } else { across(&nodes[v]) };
    let gap_between = |a: usize, b: usize| -> f64 {
        let g = match (dummy(a), dummy(b)) {
            (true, true) => 8.0,
            (false, false) => node_gap,
            _ => 12.0,
        };
        let lane = if vlane[a] != vlane[b] {
            lane_gap + lane_head
        } else {
            0.0
        };
        size(a) / 2.0 + g + lane + size(b) / 2.0
    };
    let mut y = vec![0.0f64; nv];
    for l in &layers {
        let mut at = 0.0;
        for (i, &v) in l.iter().enumerate() {
            if i > 0 {
                at += gap_between(l[i - 1], v);
            }
            y[v] = at;
        }
        let mid = at / 2.0;
        for &v in l {
            y[v] -= mid;
        }
    }
    let relax = |y: &mut Vec<f64>, r: usize, up: bool, down: bool| {
        let l = &layers[r];
        if l.is_empty() {
            return;
        }
        let want: Vec<f64> = l
            .iter()
            .map(|&v| {
                let mut nb: Vec<usize> = Vec::new();
                if up {
                    nb.extend(&vpred[v]);
                }
                if down {
                    nb.extend(&vsucc[v]);
                }
                if nb.is_empty() {
                    y[v]
                } else {
                    nb.iter().map(|&w| y[w]).sum::<f64>() / nb.len() as f64
                }
            })
            .collect();
        let weight: Vec<f64> = l
            .iter()
            .map(|&v| if dummy(v) { 2.0 } else { 1.0 })
            .collect();
        let gaps: Vec<f64> = (0..l.len())
            .map(|i| {
                if i == 0 {
                    0.0
                } else {
                    gap_between(l[i - 1], l[i])
                }
            })
            .collect();
        for (&v, q) in l.iter().zip(place(&want, &weight, &gaps)) {
            y[v] = q;
        }
    };
    for _ in 0..6 {
        for r in 1..nl {
            relax(&mut y, r, true, false);
        }
        for r in (0..nl.saturating_sub(1)).rev() {
            relax(&mut y, r, false, true);
        }
    }
    for r in 0..nl {
        relax(&mut y, r, true, true);
    }
    // swimlanes: each lane shifted clear of the one before (across all layers)
    let mut lane_span: Vec<(f64, f64)> = Vec::new();
    if laned {
        let nlanes = p.lanes + 1;
        let span = |y: &[f64], l: usize| {
            (0..nv)
                .filter(|&v| vlane[v] == l)
                .map(|v| (y[v] - size(v) / 2.0, y[v] + size(v) / 2.0))
                .fold(None, |acc: Option<(f64, f64)>, (a, b)| match acc {
                    None => Some((a, b)),
                    Some((x, z)) => Some((x.min(a), z.max(b))),
                })
        };
        let mut floor: Option<f64> = None;
        for l in 0..nlanes {
            let Some((lo, _)) = span(&y, l) else {
                lane_span.push((f64::NAN, f64::NAN));
                continue;
            };
            if let Some(f) = floor {
                let need = f + lane_gap + lane_head - lo;
                if need > 0.0 {
                    for v in 0..nv {
                        if vlane[v] == l {
                            y[v] += need;
                        }
                    }
                }
            }
            let (lo, hi) = span(&y, l).expect("lane has vertices");
            lane_span.push((lo, hi));
            floor = Some(hi);
        }
    }

    // positions along the layer axis: each layer as thick as its widest node;
    // left to right, a gap is at least as wide as the edge labels crossing it
    let mut thick = vec![0.0f64; nl];
    for v in 0..n {
        thick[rank[v]] = thick[rank[v]].max(along(&nodes[v]));
    }
    let mut gap: Vec<f64> = vec![rank_gap; nl];
    if lr {
        for e in edges {
            let (a, b) = (rank[e.from].min(rank[e.to]), rank[e.from].max(rank[e.to]));
            if b != a + 1 {
                continue;
            }
            if let Some(w) = e.label_w {
                gap[a] = gap[a].max(w + 2.0 * p.arrow + 18.0);
            }
        }
    }
    let mut start_at = vec![0.0f64; nl];
    for r in 1..nl {
        start_at[r] = start_at[r - 1] + thick[r - 1] + gap[r - 1];
    }
    let centre = |r: usize| start_at[r] + thick[r] / 2.0;
    let xy = |rk: f64, o: f64| if lr { (rk, o) } else { (o, rk) };

    let mut pn: Vec<PlacedNode> = Vec::with_capacity(n);
    for (v, b) in nodes.iter().enumerate() {
        let (cx, cy) = xy(centre(rank[v]), y[v]);
        pn.push(PlacedNode {
            cx,
            cy,
            w: b.w,
            h: b.h,
            layer: rank[v],
            order: 0,
        });
    }
    let reading = ordered((0..n).map(|v| (rank[v] as f64, y[v], v)).collect());
    for (i, &v) in reading.iter().enumerate() {
        pn[v].order = i;
    }

    // ports: a node's outgoing (incoming) chains spread along its side,
    // sorted by where they go (come from)
    let half_along = |v: usize| if lr { pn[v].w / 2.0 } else { pn[v].h / 2.0 };
    let side = |v: usize| if lr { pn[v].h } else { pn[v].w };
    let mut out_ports: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut in_ports: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (c, ch) in chains.iter().enumerate() {
        out_ports[ch[0]].push(c);
        in_ports[ch[ch.len() - 1]].push(c);
    }
    for ports in &mut out_ports {
        *ports = ordered(ports.iter().map(|&c| (y[chains[c][1]], 0.0, c)).collect());
    }
    for ports in &mut in_ports {
        *ports = ordered(
            ports
                .iter()
                .map(|&c| (y[chains[c][chains[c].len() - 2]], 0.0, c))
                .collect(),
        );
    }
    let spread = |list: &[usize], c: usize, v: usize| -> f64 {
        let m = list.len();
        if m <= 1 {
            return 0.0;
        }
        let k = list.iter().position(|&x| x == c).unwrap_or(0);
        let span = (side(v) * 0.6).min(14.0 * (m - 1) as f64);
        -span / 2.0 + span * k as f64 / (m - 1) as f64
    };
    let mut edge_pts: Vec<Vec<(f64, f64)>> = vec![Vec::new(); edges.len()];
    let mut back = vec![false; edges.len()];
    for (c, ch) in chains.iter().enumerate() {
        let (u, v, k, rev) = dag[c];
        let mut pts = Vec::with_capacity(ch.len() * 2);
        pts.push(xy(
            centre(rank[u]) + half_along(u),
            y[u] + spread(&out_ports[u], c, u),
        ));
        for &dm in &ch[1..ch.len() - 1] {
            let r = vrank[dm];
            pts.push(xy(start_at[r], y[dm]));
            pts.push(xy(start_at[r] + thick[r], y[dm]));
        }
        pts.push(xy(
            centre(rank[v]) - half_along(v),
            y[v] + spread(&in_ports[v], c, v),
        ));
        if rev {
            pts.reverse();
        }
        edge_pts[k] = pts;
        back[k] = rev;
    }
    for k in loops {
        edge_pts[k] = self_loop(&pn[edges[k].from], lr, p.loop_size);
    }
    for &(u, v, k, rev) in &free {
        let mut pts = straight(&pn, u, v);
        if rev {
            pts.reverse();
        }
        edge_pts[k] = pts;
        back[k] = rev;
    }
    // swimlane bands across every layer, with the lane's label strip
    let total = start_at[nl - 1] + thick[nl - 1];
    let bands = lane_span
        .iter()
        .enumerate()
        .filter(|(l, (lo, _))| *l < p.lanes && lo.is_finite())
        .map(|(l, &(lo, hi))| {
            let (a, b) = if lr {
                xy(-12.0, lo - lane_head - 6.0)
            } else {
                xy(-lane_head - 10.0, lo - 8.0)
            };
            let (c, e) = xy(total + 12.0, hi + 8.0);
            (l, a, b, c, e)
        })
        .collect();
    finish(pn, edge_pts, back, nl, bands, p.margin)
}

/// Move a layout to the margin and round it to half pixels.
fn finish(
    mut pn: Vec<PlacedNode>,
    mut edge_pts: Vec<Vec<(f64, f64)>>,
    back: Vec<bool>,
    layers: usize,
    mut bands: Vec<(usize, f64, f64, f64, f64)>,
    margin: f64,
) -> GraphLayout {
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for q in &pn {
        x0 = x0.min(q.cx - q.w / 2.0);
        x1 = x1.max(q.cx + q.w / 2.0);
        y0 = y0.min(q.cy - q.h / 2.0);
        y1 = y1.max(q.cy + q.h / 2.0);
    }
    for &(px, py) in edge_pts.iter().flatten() {
        x0 = x0.min(px);
        x1 = x1.max(px);
        y0 = y0.min(py);
        y1 = y1.max(py);
    }
    for &(_, a, b, c, e) in &bands {
        x0 = x0.min(a.min(c));
        x1 = x1.max(a.max(c));
        y0 = y0.min(b.min(e));
        y1 = y1.max(b.max(e));
    }
    if pn.is_empty() {
        (x0, y0, x1, y1) = (0.0, 0.0, 0.0, 0.0);
    }
    let r = |v: f64| (v * 2.0).round() / 2.0;
    let (dx, dy) = (margin - x0, margin - y0);
    for q in &mut pn {
        q.cx = r(q.cx + dx);
        q.cy = r(q.cy + dy);
    }
    for pts in &mut edge_pts {
        for pt in pts.iter_mut() {
            *pt = (r(pt.0 + dx), r(pt.1 + dy));
        }
    }
    for b in &mut bands {
        *b = (b.0, r(b.1 + dx), r(b.2 + dy), r(b.3 + dx), r(b.4 + dy));
    }
    GraphLayout {
        w: r(x1 - x0 + 2.0 * margin),
        h: r(y1 - y0 + 2.0 * margin),
        nodes: pn,
        edges: edge_pts,
        back,
        layers,
        bands,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(w: f64, h: f64) -> GraphNode {
        GraphNode {
            w,
            h,
            ..Default::default()
        }
    }
    fn edge(from: usize, to: usize) -> GraphEdge {
        GraphEdge {
            from,
            to,
            label_w: None,
        }
    }

    #[test]
    fn a_chain_runs_left_to_right_in_layers() {
        let l = layered(
            &[node(100.0, 40.0), node(100.0, 40.0), node(100.0, 40.0)],
            &[edge(0, 1), edge(1, 2)],
            &LayeredParams::default(),
        );
        assert_eq!(l.layers, 3);
        assert!(l.nodes[0].cx < l.nodes[1].cx && l.nodes[1].cx < l.nodes[2].cx);
        assert_eq!(l.nodes[0].cy, l.nodes[1].cy);
        // an edge leaves its source's right side and enters its target's left
        assert_eq!(l.edges[0][0].0, l.nodes[0].cx + 50.0);
        assert_eq!(l.edges[0].last().unwrap().0, l.nodes[1].cx - 50.0);
    }

    #[test]
    fn a_cycle_is_broken_and_marked_back() {
        let l = layered(
            &[node(80.0, 30.0), node(80.0, 30.0)],
            &[edge(0, 1), edge(1, 0)],
            &LayeredParams::default(),
        );
        assert_eq!(l.back, vec![false, true]);
        assert_eq!(l.layers, 2);
    }

    #[test]
    fn crossings_are_removed_where_possible() {
        // a→d, b→c drawn in input order would cross; the layout uncrosses them
        let ns = vec![node(60.0, 30.0); 4];
        let l = layered(&ns, &[edge(0, 3), edge(1, 2)], &LayeredParams::default());
        let (a, b, c, d) = (&l.nodes[0], &l.nodes[1], &l.nodes[2], &l.nodes[3]);
        assert_eq!(a.cy < b.cy, d.cy < c.cy, "{a:?} {b:?} {c:?} {d:?}");
    }

    #[test]
    fn pins_and_the_explicit_grid() {
        let mut ns = vec![node(60.0, 30.0); 3];
        ns[2].pin.0 = Some(3.0);
        let l = layered(&ns, &[edge(0, 1)], &LayeredParams::default());
        assert_eq!(l.nodes[2].layer, 3, "a pinned layer keeps its number");
        let grid: Vec<GraphNode> = (0..4)
            .map(|i| GraphNode {
                w: 60.0,
                h: 30.0,
                pin: (Some((i % 2) as f64), Some((i / 2) as f64)),
                ..Default::default()
            })
            .collect();
        let g = layered(&grid, &[edge(0, 3)], &LayeredParams::default());
        assert_eq!(g.layers, 2);
        assert_eq!(g.nodes[0].cy, g.nodes[1].cy);
        assert_eq!(g.edges[0].len(), 2, "straight");
    }

    #[test]
    fn lanes_become_bands_and_top_bottom_swaps_the_axes() {
        let mut ns = vec![node(60.0, 30.0); 2];
        ns[0].lane = Some(0);
        ns[1].lane = Some(1);
        let l = layered(
            &ns,
            &[edge(0, 1)],
            &LayeredParams {
                lanes: 2,
                ..Default::default()
            },
        );
        assert_eq!(l.bands.len(), 2);
        let tb = layered(
            &ns[..1],
            &[],
            &LayeredParams {
                direction: Direction::TopBottom,
                ..Default::default()
            },
        );
        assert_eq!(tb.nodes.len(), 1);
        let t = layered(
            &[node(60.0, 30.0), node(60.0, 30.0)],
            &[edge(0, 1)],
            &LayeredParams {
                direction: Direction::TopBottom,
                ..Default::default()
            },
        );
        assert!(t.nodes[0].cy < t.nodes[1].cy && t.nodes[0].cx == t.nodes[1].cx);
    }

    #[test]
    fn edge_cases_loops_labels_ports_lanes_and_empty() {
        let p = LayeredParams::default();
        let empty = layered(&[], &[], &p);
        assert!(empty.nodes.is_empty() && empty.layers == 1);
        // self-loops left to right and top to bottom, in layers and on a grid
        for dir in [Direction::LeftRight, Direction::TopBottom] {
            let q = LayeredParams {
                direction: dir,
                ..Default::default()
            };
            let l = layered(
                &[node(60.0, 30.0), node(60.0, 30.0)],
                &[edge(0, 0), edge(0, 1)],
                &q,
            );
            assert_eq!(l.edges[0].len(), 4, "a self-loop");
            let g: Vec<GraphNode> = (0..2)
                .map(|i| GraphNode {
                    w: 60.0,
                    h: 30.0,
                    pin: (Some(i as f64), Some(0.0)),
                    ..Default::default()
                })
                .collect();
            let gl = layered(&g, &[edge(1, 1), edge(0, 1)], &q);
            assert_eq!(gl.edges[0].len(), 4);
        }
        // an edge label widens its gap; a long edge gets dummies (bends)
        let ns = vec![node(60.0, 30.0); 4];
        let plain = layered(
            &ns,
            &[edge(0, 1), edge(1, 2), edge(0, 2), edge(0, 3), edge(2, 3)],
            &p,
        );
        let labelled = layered(
            &ns,
            &[
                GraphEdge {
                    from: 0,
                    to: 1,
                    label_w: Some(300.0),
                },
                edge(1, 2),
                edge(0, 2),
                edge(0, 3),
                edge(2, 3),
            ],
            &p,
        );
        assert!(
            labelled.nodes[1].cx - labelled.nodes[0].cx > plain.nodes[1].cx - plain.nodes[0].cx
        );
        assert!(
            plain.edges[3].len() > 2,
            "a long edge bends through its dummies"
        );
        // several edges leave one node: their ports spread along its side
        let starts: Vec<f64> = [0, 2, 3].iter().map(|&k| plain.edges[k][0].1).collect();
        assert!(starts.windows(2).any(|w| w[0] != w[1]), "{starts:?}");
        // a pinned layer against the flow: a straight edge drawn back
        let mut pn = vec![node(60.0, 30.0); 3];
        pn[0].pin.0 = Some(2.0);
        pn[1].pin.0 = Some(0.0);
        let back = layered(&pn, &[edge(0, 1), edge(1, 2)], &p);
        assert_eq!(back.edges[0].len(), 2);
        // swimlanes top to bottom, an unlaned node, crossings between lanes
        let mut lanes = vec![node(60.0, 30.0); 5];
        for (i, n) in lanes.iter_mut().enumerate().take(4) {
            n.lane = Some(i % 2);
        }
        let q = LayeredParams {
            direction: Direction::TopBottom,
            lanes: 2,
            ..Default::default()
        };
        let tb = layered(
            &lanes,
            &[edge(0, 3), edge(1, 2), edge(2, 4), edge(3, 4)],
            &q,
        );
        assert_eq!(tb.bands.len(), 2);
        assert!(tb.bands.iter().all(|b| b.3 > b.1 && b.4 > b.2));
    }
}
