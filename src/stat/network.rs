//! `StatNetwork`: node-link layouts for network graphs.
//!
//! Input: an edge list (`from`, `to`, optional `weight`, 1 when unmapped;
//! repeated pairs summed, self-loops dropped). Nodes are laid out by a
//! seeded Fruchterman–Reingold force simulation (deterministic for a seed),
//! on a circle, or on a grid.
//!
//! Output: one row per node (`part = "node"`, `x`/`y`, `node`, `degree` =
//! summed incident weight, `component` = connected-component number) then
//! one per edge (`part = "edge"`, `x`/`y` → `xend`/`yend`, `from`, `to`,
//! `value`), plus constant `xmin`/`xmax`/`ymin`/`ymax` framing a square.

use std::collections::HashMap;
use std::f64::consts::TAU;

use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::rng::SplitMix64;
use crate::scale::ScaleSet;

use super::Stat;

/// Node placement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NetworkLayout {
    /// Fruchterman–Reingold forces from a seeded start.
    ForceDirected { iterations: usize, seed: u64 },
    /// Nodes evenly on a circle, in first-appearance order.
    Circle,
    /// Nodes on a square grid, row by row.
    Grid,
}

impl Default for NetworkLayout {
    fn default() -> Self {
        NetworkLayout::ForceDirected {
            iterations: 300,
            seed: 42,
        }
    }
}

/// Network layout stat behind [`GeomNetwork`](crate::geom::network::GeomNetwork).
#[derive(Clone, Debug, Default)]
pub struct StatNetwork {
    pub layout: NetworkLayout,
}

/// Fruchterman–Reingold in the unit square, with a weak pull to the centre
/// so disconnected components stay together.
pub(crate) fn force_directed(
    n: usize,
    edges: &[(usize, usize, f64)],
    iterations: usize,
    seed: u64,
) -> Vec<(f64, f64)> {
    let mut rng = SplitMix64::new(seed);
    let mut pos: Vec<(f64, f64)> = (0..n)
        .map(|_| (rng.range_f64(-0.5, 0.5), rng.range_f64(-0.5, 0.5)))
        .collect();
    if n < 2 {
        return vec![(0.0, 0.0); n];
    }
    let k = (1.0 / n as f64).sqrt();
    let iterations = iterations.max(1);
    for it in 0..iterations {
        let temp = 0.1 * (1.0 - it as f64 / iterations as f64) + 1e-4;
        let mut disp = vec![(0.0, 0.0); n];
        for i in 0..n {
            for j in i + 1..n {
                let (dx, dy) = (pos[i].0 - pos[j].0, pos[i].1 - pos[j].1);
                let d = dx.hypot(dy).max(1e-3);
                let f = k * k / d;
                let (fx, fy) = (dx / d * f, dy / d * f);
                disp[i].0 += fx;
                disp[i].1 += fy;
                disp[j].0 -= fx;
                disp[j].1 -= fy;
            }
        }
        for &(a, b, _) in edges {
            let (dx, dy) = (pos[a].0 - pos[b].0, pos[a].1 - pos[b].1);
            let d = dx.hypot(dy).max(1e-3);
            let f = d * d / k;
            let (fx, fy) = (dx / d * f, dy / d * f);
            disp[a].0 -= fx;
            disp[a].1 -= fy;
            disp[b].0 += fx;
            disp[b].1 += fy;
        }
        for i in 0..n {
            disp[i].0 -= 0.5 * k * pos[i].0;
            disp[i].1 -= 0.5 * k * pos[i].1;
            let len = disp[i].0.hypot(disp[i].1);
            if len > 0.0 {
                let step = len.min(temp);
                pos[i].0 += disp[i].0 / len * step;
                pos[i].1 += disp[i].1 / len * step;
            }
        }
    }
    pos
}

/// Force-directed layout per connected component (so components don't
/// drift apart), each scaled by its node count and packed in shelf rows.
fn layout_components(
    n: usize,
    edges: &[(usize, usize, f64)],
    comp: &[usize],
    iterations: usize,
    seed: u64,
) -> Vec<(f64, f64)> {
    let n_comp = comp.iter().copied().max().unwrap_or(0);
    let mut pos = vec![(0.0, 0.0); n];
    // (component, members, size) — larger components first.
    let mut parts: Vec<(Vec<usize>, f64)> = (1..=n_comp)
        .map(|c| {
            let members: Vec<usize> = (0..n).filter(|&i| comp[i] == c).collect();
            let size = (members.len() as f64).sqrt();
            (members, size)
        })
        .collect();
    parts.sort_by(|a, b| b.1.total_cmp(&a.1));
    let row_width = parts.iter().map(|p| p.1 * p.1).sum::<f64>().sqrt() * 1.3;
    let (mut x, mut y, mut row_h) = (0.0, 0.0, 0.0f64);
    for (k, (members, size)) in parts.iter().enumerate() {
        let size = *size;
        let local: std::collections::HashMap<usize, usize> =
            members.iter().enumerate().map(|(j, &i)| (i, j)).collect();
        let sub: Vec<(usize, usize, f64)> = edges
            .iter()
            .filter_map(|&(a, b, w)| Some((*local.get(&a)?, *local.get(&b)?, w)))
            .collect();
        let p = force_directed(members.len(), &sub, iterations, seed.wrapping_add(k as u64));
        // Normalise to a box of side `size` (a single node is a point).
        let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for &(px, py) in &p {
            x0 = x0.min(px);
            x1 = x1.max(px);
            y0 = y0.min(py);
            y1 = y1.max(py);
        }
        let span = (x1 - x0).max(y1 - y0);
        let scale = if span > 0.0 { size / span } else { 0.0 };
        let side = if span > 0.0 { size } else { 0.0 };
        if x > 0.0 && x + side > row_width {
            x = 0.0;
            y -= row_h + 0.6;
            row_h = 0.0;
        }
        for (&i, &(px, py)) in members.iter().zip(&p) {
            pos[i] = (x + (px - x0) * scale, y - (py - y0) * scale);
        }
        x += side + 0.6;
        row_h = row_h.max(side);
    }
    pos
}

/// Connected-component number (1-based, in first-appearance order) per node.
pub(crate) fn components(n: usize, edges: &[(usize, usize, f64)]) -> Vec<usize> {
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut [usize], mut x: usize) -> usize {
        while p[x] != x {
            p[x] = p[p[x]];
            x = p[x];
        }
        x
    }
    for &(a, b, _) in edges {
        let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
        if ra != rb {
            parent[ra.max(rb)] = ra.min(rb);
        }
    }
    let mut label: HashMap<usize, usize> = HashMap::new();
    (0..n)
        .map(|i| {
            let r = find(&mut parent, i);
            let next = label.len() + 1;
            *label.entry(r).or_insert(next)
        })
        .collect()
}

impl Stat for StatNetwork {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let (Some(from), Some(to)) = (data.column("from"), data.column("to")) else {
            return DataFrame::new();
        };
        let weight = data.column("weight");
        let mut index: HashMap<String, usize> = HashMap::new();
        let mut names: Vec<String> = Vec::new();
        let mut id = |v: &Value| -> Option<usize> {
            if v.is_na() {
                return None;
            }
            let key = v.key_str().into_owned();
            Some(*index.entry(key.clone()).or_insert_with(|| {
                names.push(key);
                names.len() - 1
            }))
        };
        let mut pairs: indexmap::IndexMap<(usize, usize), f64> = indexmap::IndexMap::new();
        for i in 0..data.nrows() {
            let (Some(a), Some(b)) = (id(&from[i]), id(&to[i])) else {
                continue;
            };
            let w = match weight {
                Some(c) => match c[i].as_f64() {
                    Some(w) if w.is_finite() && w > 0.0 => w,
                    _ => continue,
                },
                None => 1.0,
            };
            if a != b {
                *pairs.entry((a, b)).or_insert(0.0) += w;
            }
        }
        let n = names.len();
        if n == 0 {
            return DataFrame::new();
        }
        let edges: Vec<(usize, usize, f64)> =
            pairs.into_iter().map(|((a, b), w)| (a, b, w)).collect();
        let comp = components(n, &edges);
        let pos = match self.layout {
            NetworkLayout::ForceDirected { iterations, seed } => {
                layout_components(n, &edges, &comp, iterations, seed)
            }
            NetworkLayout::Circle => (0..n)
                .map(|i| {
                    let a = TAU * i as f64 / n as f64;
                    (0.5 * a.sin(), 0.5 * a.cos())
                })
                .collect(),
            NetworkLayout::Grid => {
                let cols = (n as f64).sqrt().ceil() as usize;
                (0..n)
                    .map(|i| ((i % cols) as f64, -((i / cols) as f64)))
                    .collect()
            }
        };
        let mut degree = vec![0.0; n];
        for &(a, b, w) in &edges {
            degree[a] += w;
            degree[b] += w;
        }
        // Square frame around the layout with a margin for nodes/labels.
        let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for &(x, y) in &pos {
            x0 = x0.min(x);
            x1 = x1.max(x);
            y0 = y0.min(y);
            y1 = y1.max(y);
        }
        let half = ((x1 - x0).max(y1 - y0) / 2.0).max(0.5) * 1.12;
        let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);

        let s = |x: &str| Value::Str(x.to_string());
        let rows = n + edges.len();
        let (mut part, mut x, mut y, mut xe, mut ye) = (vec![], vec![], vec![], vec![], vec![]);
        let (mut node, mut f, mut t, mut value, mut deg, mut cmp) =
            (vec![], vec![], vec![], vec![], vec![], vec![]);
        for i in 0..n {
            part.push(s("node"));
            x.push(pos[i].0);
            y.push(pos[i].1);
            xe.push(pos[i].0);
            ye.push(pos[i].1);
            node.push(s(&names[i]));
            f.push(s(&names[i]));
            t.push(s(&names[i]));
            value.push(Value::Float(degree[i]));
            deg.push(Value::Float(degree[i]));
            cmp.push(s(&comp[i].to_string()));
        }
        for &(a, b, w) in &edges {
            part.push(s("edge"));
            x.push(pos[a].0);
            y.push(pos[a].1);
            xe.push(pos[b].0);
            ye.push(pos[b].1);
            node.push(s(&names[a]));
            f.push(s(&names[a]));
            t.push(s(&names[b]));
            value.push(Value::Float(w));
            deg.push(Value::Float(degree[a]));
            cmp.push(s(&comp[a].to_string()));
        }
        let fl = |v: Vec<f64>| v.into_iter().map(Value::Float).collect::<Vec<_>>();
        let mut out = DataFrame::new();
        out.add_column("part".into(), part);
        out.add_column("x".into(), fl(x));
        out.add_column("y".into(), fl(y));
        out.add_column("xend".into(), fl(xe));
        out.add_column("yend".into(), fl(ye));
        out.add_column("node".into(), node);
        out.add_column("from".into(), f);
        out.add_column("to".into(), t);
        out.add_column("value".into(), value);
        out.add_column("degree".into(), deg);
        out.add_column("component".into(), cmp);
        for (c, v) in [
            ("xmin", cx - half),
            ("xmax", cx + half),
            ("ymin", cy - half),
            ("ymax", cy + half),
        ] {
            out.add_column(c.into(), vec![Value::Float(v); rows]);
        }
        out
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::From, Aesthetic::To]
    }

    /// The layout needs every edge of the panel at once.
    fn panelwise(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "network"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn force_layout_is_deterministic_and_separates_nodes() {
        let edges = vec![(0, 1, 1.0), (1, 2, 1.0), (2, 0, 1.0), (3, 4, 1.0)];
        let a = force_directed(5, &edges, 200, 7);
        let b = force_directed(5, &edges, 200, 7);
        assert_eq!(a, b, "same seed, same layout");
        assert_ne!(a, force_directed(5, &edges, 200, 8));
        for i in 0..5 {
            for j in i + 1..5 {
                let d = (a[i].0 - a[j].0).hypot(a[i].1 - a[j].1);
                assert!(d > 0.05, "nodes {i},{j} collapsed: {d}");
            }
        }
        // Linked nodes end up closer than the average pair.
        let dist = |i: usize, j: usize| (a[i].0 - a[j].0).hypot(a[i].1 - a[j].1);
        assert!(dist(3, 4) < dist(0, 3).max(dist(1, 4)));
    }

    #[test]
    fn components_and_degrees() {
        assert_eq!(
            components(5, &[(0, 1, 1.0), (3, 4, 1.0)]),
            vec![1, 1, 2, 3, 3]
        );
        let mut d = DataFrame::new();
        let s = |v: &[&str]| v.iter().map(|x| Value::Str(x.to_string())).collect();
        d.add_column("from".into(), s(&["a", "a", "b", "c"]));
        d.add_column("to".into(), s(&["b", "c", "c", "c"]));
        let out = StatNetwork {
            layout: NetworkLayout::Circle,
        }
        .compute_group(&d, &ScaleSet::new());
        assert_eq!(out.nrows(), 3 + 3, "self-loop dropped");
        let deg: Vec<f64> = out.column("degree").unwrap()[..3]
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        assert_eq!(deg, vec![2.0, 2.0, 2.0]);
    }
}
