//! `StatArcdiagram`: node positions and link spans for an arc diagram.
//!
//! Input: an edge list (`from`, `to`, optional `weight`, 1 when unmapped;
//! repeated pairs summed, self-loops dropped). Nodes sit on a line at
//! x = 0, 1, 2, … in first-appearance order (or by descending total flow);
//! each link spans from its source's x to its target's x.
//!
//! Output: one row per node (`part = "node"`, `x`, `y = 0`, `value` = total
//! in + out flow) then one per link (`part = "link"`, `x` → `xend`,
//! `value` = weight), plus `node` (name; the source for a link), `from`,
//! `to`, and `ymin`/`ymax` sized so the y scale holds every arc (above, or
//! below for leftward links when `directed_below`) and room for labels.

use std::collections::HashMap;

use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Arc-diagram layout stat behind [`GeomArcdiagram`](crate::geom::arcdiagram::GeomArcdiagram).
#[derive(Clone, Debug, Default)]
pub struct StatArcdiagram {
    /// Order nodes by descending total flow instead of first appearance.
    pub sort_by_value: bool,
    /// Draw links that run right-to-left below the line (shows direction).
    pub directed_below: bool,
}

impl Stat for StatArcdiagram {
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
        let mut total = vec![0.0; n];
        for (&(a, b), &w) in &pairs {
            total[a] += w;
            total[b] += w;
        }
        let mut order: Vec<usize> = (0..n).collect();
        if self.sort_by_value {
            order.sort_by(|&a, &b| total[b].total_cmp(&total[a]));
        }
        let mut pos = vec![0.0; n];
        for (slot, &k) in order.iter().enumerate() {
            pos[k] = slot as f64;
        }
        let span = |a: usize, b: usize| (pos[b] - pos[a]).abs() / 2.0;
        let up = pairs
            .keys()
            .filter(|&&(a, b)| !self.directed_below || pos[b] > pos[a])
            .map(|&(a, b)| span(a, b))
            .fold(0.0, f64::max);
        let down = pairs
            .keys()
            .filter(|&&(a, b)| self.directed_below && pos[b] < pos[a])
            .map(|&(a, b)| span(a, b))
            .fold(0.0, f64::max);
        // Room under the line for the node labels.
        let room = 0.12 * up.max(down).max(0.5);

        let s = |x: &str| Value::Str(x.to_string());
        let (mut part, mut x, mut xend, mut node, mut f, mut t, mut value) =
            (vec![], vec![], vec![], vec![], vec![], vec![], vec![]);
        for &k in &order {
            part.push(s("node"));
            x.push(pos[k]);
            xend.push(pos[k]);
            node.push(s(&names[k]));
            f.push(s(&names[k]));
            t.push(s(&names[k]));
            value.push(total[k]);
        }
        for (&(a, b), &w) in &pairs {
            part.push(s("link"));
            x.push(pos[a]);
            xend.push(pos[b]);
            node.push(s(&names[a]));
            f.push(s(&names[a]));
            t.push(s(&names[b]));
            value.push(w);
        }
        let rows = part.len();
        let floats = |v: Vec<f64>| v.into_iter().map(Value::Float).collect::<Vec<_>>();
        let mut out = DataFrame::new();
        out.add_column("part".into(), part);
        out.add_column("x".into(), floats(x));
        out.add_column("xend".into(), floats(xend));
        out.add_column("y".into(), vec![Value::Float(0.0); rows]);
        out.add_column("ymin".into(), vec![Value::Float(-(down + room)); rows]);
        out.add_column("ymax".into(), vec![Value::Float(up); rows]);
        out.add_column("node".into(), node);
        out.add_column("from".into(), f);
        out.add_column("to".into(), t);
        out.add_column("value".into(), floats(value));
        out
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::From, Aesthetic::To]
    }

    /// The layout needs every link of the panel at once.
    fn panelwise(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "arcdiagram"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edges(e: &[(&str, &str)]) -> DataFrame {
        let mut d = DataFrame::new();
        d.add_column(
            "from".into(),
            e.iter().map(|r| Value::Str(r.0.to_string())).collect(),
        );
        d.add_column(
            "to".into(),
            e.iter().map(|r| Value::Str(r.1.to_string())).collect(),
        );
        d
    }

    fn col(d: &DataFrame, c: &str) -> Vec<f64> {
        d.column(c)
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect()
    }

    #[test]
    fn positions_spans_and_degree() {
        let d = edges(&[("A", "B"), ("A", "C"), ("C", "A"), ("D", "A"), ("B", "B")]);
        let out = StatArcdiagram::default().compute_group(&d, &ScaleSet::new());
        assert_eq!(out.nrows(), 4 + 4, "self-loop dropped");
        assert_eq!(&col(&out, "x")[..4], &[0.0, 1.0, 2.0, 3.0]);
        assert_eq!(col(&out, "value")[0], 4.0, "A: 2 out + 2 in");
        // Tallest arc spans D(3) → A(0): radius 1.5.
        assert_eq!(col(&out, "ymax")[0], 1.5);
        assert!(col(&out, "ymin")[0] < 0.0);
    }

    #[test]
    fn sort_and_directed_below() {
        let d = edges(&[("B", "A"), ("C", "A"), ("C", "B")]);
        let out = StatArcdiagram {
            sort_by_value: true,
            directed_below: true,
        }
        .compute_group(&d, &ScaleSet::new());
        let names: Vec<String> = out.column("node").unwrap()[..3]
            .iter()
            .map(|v| v.key_str().into_owned())
            .collect();
        // Totals: A 2, B 2, C 2 → stable order of appearance (B, A, C).
        assert_eq!(names, ["B", "A", "C"]);
        // C(2) → A(1) and C(2) → B(0) run leftward: below the line.
        assert_eq!(col(&out, "ymin")[0], -(1.0 + 0.12));
        assert_eq!(col(&out, "ymax")[0], 0.5, "only B → A runs rightward");
    }
}
