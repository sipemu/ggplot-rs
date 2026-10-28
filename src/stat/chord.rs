//! `StatChord`: angular layout for a chord diagram / dependency wheel.
//!
//! Input: an edge list (`from`, `to`, optional `weight`, 1 when unmapped;
//! repeated pairs summed, self-loops dropped). Each node gets an arc on the
//! circle proportional to its total flow (in + out, Highcharts' dependency
//! wheel), separated by `pad_angle`; inside its arc a node lays out its
//! outgoing links first, then its incoming ones, each ordered around the
//! circle. Angles run clockwise from 12 o'clock, in radians.
//!
//! Output: one row per node (`part = "node"`, arc `a0..a1`) then one per link
//! (`part = "link"`, source band `a0..a1`, target band `b0..b1`), with `node`
//! (name; the source for a link), `from`, `to`, `value`, and constant
//! `xmin`/`xmax`/`ymin`/`ymax` framing the unit circle plus label room so
//! the position scales train on a square-ish extent.

use std::collections::HashMap;
use std::f64::consts::TAU;

use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Chord layout stat behind [`GeomChord`](crate::geom::chord::GeomChord).
#[derive(Clone, Debug)]
pub struct StatChord {
    /// Gap between neighbouring node arcs, in radians.
    pub pad_angle: f64,
    /// Half-width of the trained x extent (the circle has radius 1; the rest
    /// is room for labels).
    pub x_extent: f64,
    /// Half-height of the trained y extent.
    pub y_extent: f64,
}

impl Default for StatChord {
    fn default() -> Self {
        StatChord {
            pad_angle: 0.04,
            x_extent: 1.45,
            y_extent: 1.25,
        }
    }
}

impl Stat for StatChord {
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
        let links: Vec<(usize, usize, f64)> =
            pairs.into_iter().map(|((a, b), w)| (a, b, w)).collect();
        if n == 0 || links.is_empty() {
            return DataFrame::new();
        }
        let mut total = vec![0.0; n];
        for &(a, b, w) in &links {
            total[a] += w;
            total[b] += w;
        }
        let pad = self.pad_angle.clamp(0.0, TAU / n as f64 * 0.5);
        let k = (TAU - pad * n as f64) / total.iter().sum::<f64>();
        let mut start = vec![0.0; n];
        let mut a = 0.0;
        for i in 0..n {
            start[i] = a;
            a += total[i] * k + pad;
        }
        // Order a node's bands around the circle: outgoing (by target
        // position, clockwise from the node), then incoming (by source).
        let rel = |from: usize, other: usize| (other + n - from) % n;
        let mut src = vec![(0.0, 0.0); links.len()];
        let mut dst = vec![(0.0, 0.0); links.len()];
        for (node, &node_start) in start.iter().enumerate() {
            let mut cursor = node_start;
            let mut out: Vec<usize> = (0..links.len()).filter(|&l| links[l].0 == node).collect();
            out.sort_by_key(|&l| std::cmp::Reverse(rel(node, links[l].1)));
            for l in out {
                src[l] = (cursor, cursor + links[l].2 * k);
                cursor = src[l].1;
            }
            let mut inc: Vec<usize> = (0..links.len()).filter(|&l| links[l].1 == node).collect();
            inc.sort_by_key(|&l| std::cmp::Reverse(rel(node, links[l].0)));
            for l in inc {
                dst[l] = (cursor, cursor + links[l].2 * k);
                cursor = dst[l].1;
            }
        }

        let rows = n + links.len();
        let mut part = Vec::with_capacity(rows);
        let (mut a0, mut a1, mut b0, mut b1) = (vec![], vec![], vec![], vec![]);
        let (mut node, mut f, mut t, mut value) = (vec![], vec![], vec![], vec![]);
        let s = |x: &str| Value::Str(x.to_string());
        for i in 0..n {
            let (lo, hi) = (start[i], start[i] + total[i] * k);
            part.push(s("node"));
            a0.push(lo);
            a1.push(hi);
            b0.push(lo);
            b1.push(hi);
            node.push(s(&names[i]));
            f.push(s(&names[i]));
            t.push(s(&names[i]));
            value.push(total[i]);
        }
        for (l, &(x, y, w)) in links.iter().enumerate() {
            part.push(s("link"));
            a0.push(src[l].0);
            a1.push(src[l].1);
            b0.push(dst[l].0);
            b1.push(dst[l].1);
            node.push(s(&names[x]));
            f.push(s(&names[x]));
            t.push(s(&names[y]));
            value.push(w);
        }
        let floats = |v: Vec<f64>| v.into_iter().map(Value::Float).collect::<Vec<_>>();
        let mut outdf = DataFrame::new();
        outdf.add_column("part".into(), part);
        outdf.add_column("a0".into(), floats(a0));
        outdf.add_column("a1".into(), floats(a1));
        outdf.add_column("b0".into(), floats(b0));
        outdf.add_column("b1".into(), floats(b1));
        outdf.add_column("node".into(), node);
        outdf.add_column("from".into(), f);
        outdf.add_column("to".into(), t);
        outdf.add_column("value".into(), floats(value));
        for (c, v) in [
            ("xmin", -self.x_extent),
            ("xmax", self.x_extent),
            ("ymin", -self.y_extent),
            ("ymax", self.y_extent),
        ] {
            outdf.add_column(c.into(), vec![Value::Float(v); rows]);
        }
        outdf
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::From, Aesthetic::To]
    }

    /// The layout needs every link of the panel at once.
    fn panelwise(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "chord"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edges(e: &[(&str, &str, f64)]) -> DataFrame {
        let mut d = DataFrame::new();
        d.add_column(
            "from".into(),
            e.iter().map(|r| Value::Str(r.0.to_string())).collect(),
        );
        d.add_column(
            "to".into(),
            e.iter().map(|r| Value::Str(r.1.to_string())).collect(),
        );
        d.add_column(
            "weight".into(),
            e.iter().map(|r| Value::Float(r.2)).collect(),
        );
        d
    }

    fn f(d: &DataFrame, c: &str) -> Vec<f64> {
        d.column(c)
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect()
    }

    #[test]
    fn arcs_fill_the_circle_and_bands_nest_in_their_nodes() {
        let d = edges(&[
            ("A", "B", 10.0),
            ("B", "A", 5.0),
            ("A", "C", 3.0),
            ("C", "B", 2.0),
            ("A", "A", 9.0),
        ]);
        let stat = StatChord::default();
        let out = stat.compute_group(&d, &ScaleSet::new());
        assert_eq!(out.nrows(), 3 + 4, "self-loop dropped");
        let (a0, a1, b0, b1) = (f(&out, "a0"), f(&out, "a1"), f(&out, "b0"), f(&out, "b1"));
        let arcs: f64 = (0..3).map(|i| a1[i] - a0[i]).sum();
        assert!((arcs + 3.0 * stat.pad_angle - TAU).abs() < 1e-9);
        // A: out 13 + in 5 = 18 of 40 total flow (each link counted twice).
        let k = arcs / 40.0;
        assert!((a1[0] - a0[0] - 18.0 * k).abs() < 1e-9);
        let name = |c: &str, i: usize| out.column(c).unwrap()[i].key_str().into_owned();
        let node_of = |n: &str| (0..3).find(|&i| name("node", i) == n).unwrap();
        for l in 3..7 {
            let (s, t) = (node_of(&name("from", l)), node_of(&name("to", l)));
            assert!(a0[l] >= a0[s] - 1e-9 && a1[l] <= a1[s] + 1e-9);
            assert!(b0[l] >= a0[t] - 1e-9 && b1[l] <= a1[t] + 1e-9);
        }
    }
}
