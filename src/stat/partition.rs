//! `StatPartition`: hierarchical partition layout (icicle / sunburst).
//!
//! Each input row is a leaf path given by the `levels` columns (outermost
//! first) with a `weight` (1 when unmapped). Every prefix of a path is a
//! node whose value is the sum of its leaves. Nodes at depth `d` occupy
//! `y ∈ [d, d + 1]`; siblings split their parent's x span in
//! proportion to value (first-appearance order, or largest first with
//! `sort`). The root spans x ∈ [0, 1]. A missing (`Na`/empty) level ends a
//! path early, so ragged hierarchies work.
//!
//! Output: one row per node with `xmin`/`xmax`/`ymin`/`ymax`, `depth`
//! (0-based), `node` (its own level value), `path` (`"A / B / C"`), `root`
//! (the depth-0 ancestor, for colouring), `value` and `share` (of the total).

use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Partition stat behind [`GeomPartition`](crate::geom::partition::GeomPartition).
#[derive(Clone, Debug, Default)]
pub struct StatPartition {
    /// Hierarchy columns, outermost level first.
    pub levels: Vec<String>,
    /// Order siblings by descending value instead of first appearance.
    pub sort: bool,
    /// Grow downward from the top (an icicle with the root on top):
    /// depth `d` spans `y ∈ [-(d + 1), -d]`.
    pub downward: bool,
}

#[derive(Default)]
struct Node {
    name: String,
    value: f64,
    children: indexmap::IndexMap<String, Node>,
}

impl Stat for StatPartition {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let cols: Vec<&[Value]> = self.levels.iter().filter_map(|l| data.column(l)).collect();
        if cols.is_empty() || cols.len() != self.levels.len() {
            return DataFrame::new();
        }
        let weight = data.column("weight");
        let mut root = Node::default();
        for i in 0..data.nrows() {
            let w = match weight {
                Some(c) => match c[i].as_f64() {
                    Some(w) if w.is_finite() && w > 0.0 => w,
                    _ => continue,
                },
                None => 1.0,
            };
            root.value += w;
            let mut node = &mut root;
            for col in &cols {
                let key = col[i].key_str().into_owned();
                if col[i].is_na() || key.is_empty() {
                    break;
                }
                node = node.children.entry(key.clone()).or_insert_with(|| Node {
                    name: key,
                    ..Default::default()
                });
                node.value += w;
            }
        }
        if root.value <= 0.0 {
            return DataFrame::new();
        }

        struct Row {
            x: (f64, f64),
            depth: usize,
            node: String,
            path: String,
            root: String,
            value: f64,
        }
        fn walk(
            node: &Node,
            (x0, span): (f64, f64),
            depth: usize,
            path: &str,
            root: &str,
            sort: bool,
            out: &mut Vec<Row>,
        ) {
            let mut kids: Vec<&Node> = node.children.values().collect();
            if sort {
                kids.sort_by(|a, b| b.value.total_cmp(&a.value));
            }
            let mut x = x0;
            for k in kids {
                let w = if node.value > 0.0 {
                    span * k.value / node.value
                } else {
                    0.0
                };
                let p = if path.is_empty() {
                    k.name.clone()
                } else {
                    format!("{path} / {}", k.name)
                };
                let r = if depth == 0 { k.name.as_str() } else { root };
                out.push(Row {
                    x: (x, x + w),
                    depth,
                    node: k.name.clone(),
                    path: p.clone(),
                    root: r.to_string(),
                    value: k.value,
                });
                walk(k, (x, w), depth + 1, &p, r, sort, out);
                x += w;
            }
        }
        let mut rows = Vec::new();
        walk(&root, (0.0, 1.0), 0, "", "", self.sort, &mut rows);

        let total = root.value;
        let f = |g: &dyn Fn(&Row) -> f64| rows.iter().map(|r| Value::Float(g(r))).collect();
        let s = |g: &dyn Fn(&Row) -> String| rows.iter().map(|r| Value::Str(g(r))).collect();
        let mut out = DataFrame::new();
        out.add_column("xmin".into(), f(&|r| r.x.0));
        out.add_column("xmax".into(), f(&|r| r.x.1));
        let band = |r: &Row| {
            let (lo, hi) = (r.depth as f64, r.depth as f64 + 1.0);
            if self.downward {
                (-hi, -lo)
            } else {
                (lo, hi)
            }
        };
        out.add_column("ymin".into(), f(&|r| band(r).0));
        out.add_column("ymax".into(), f(&|r| band(r).1));
        out.add_column("depth".into(), f(&|r| r.depth as f64));
        out.add_column("node".into(), s(&|r| r.node.clone()));
        out.add_column("path".into(), s(&|r| r.path.clone()));
        out.add_column("root".into(), s(&|r| r.root.clone()));
        out.add_column("value".into(), f(&|r| r.value));
        out.add_column("share".into(), f(&|r| r.value / total));
        out
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        Vec::new()
    }

    /// The partition spans every row of the panel.
    fn panelwise(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "partition"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_spans_sum_to_parents() {
        let mut d = DataFrame::new();
        let s = |v: &[&str]| v.iter().map(|x| Value::Str(x.to_string())).collect();
        d.add_column("a".into(), s(&["X", "X", "X", "Y"]));
        d.add_column("b".into(), s(&["p", "p", "q", ""]));
        d.add_column(
            "weight".into(),
            [1.0, 2.0, 3.0, 4.0]
                .iter()
                .map(|&v| Value::Float(v))
                .collect(),
        );
        let out = StatPartition {
            levels: vec!["a".into(), "b".into()],
            sort: false,
            downward: false,
        }
        .compute_group(&d, &ScaleSet::new());
        // X, X/p, X/q, Y (Y's empty child ends its path).
        assert_eq!(out.nrows(), 4);
        let paths: Vec<String> = out
            .column("path")
            .unwrap()
            .iter()
            .map(|v| v.key_str().into_owned())
            .collect();
        assert_eq!(paths, ["X", "X / p", "X / q", "Y"]);
        let f = |c: &str| -> Vec<f64> {
            out.column(c)
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect()
        };
        assert_eq!(f("value"), vec![6.0, 3.0, 3.0, 4.0]);
        assert_eq!(f("xmin"), vec![0.0, 0.0, 0.3, 0.6]);
        assert_eq!(f("xmax"), vec![0.6, 0.3, 0.6, 1.0]);
        assert_eq!(f("ymin"), vec![0.0, 1.0, 1.0, 0.0]);
        assert_eq!(out.column("root").unwrap()[2], Value::Str("X".into()));
    }
}
