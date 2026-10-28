//! `StatTree`: hierarchical node-link layouts (tidy tree / dendrogram /
//! org chart).
//!
//! Input: parent → child links (`from` = parent, `to` = child). Nodes with
//! no parent are roots (several roots lay out side by side). Leaves are
//! spaced one unit apart in depth-first order and every parent is centred
//! over its children, so subtrees never overlap. A node's level is its depth
//! (root 0) unless `height` names a column holding each link's *parent*
//! height (dendrograms: leaves at 0, merges at their merge height).
//!
//! Output: one row per node (`part = "node"`, `x`/`y`, `node`, `depth`,
//! `leaf`) then one per link (`part = "link"`, `x`/`y` = parent, `xend`/
//! `yend` = child, `from`, `to`), already oriented (top-down, left-right or
//! radial), plus `xmin`/`xmax`/`ymin`/`ymax` framing the layout.

use std::collections::HashMap;
use std::f64::consts::TAU;

use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Tree orientation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TreeOrientation {
    /// Root on top, children below.
    #[default]
    TopDown,
    /// Root on the left, children to the right.
    LeftRight,
    /// Root in the centre, levels as rings.
    Radial,
}

/// Tree layout stat behind [`GeomTree`](crate::geom::tree::GeomTree).
#[derive(Clone, Debug, Default)]
pub struct StatTree {
    pub orientation: TreeOrientation,
    /// Put every leaf on the deepest level (dendrogram style).
    pub align_leaves: bool,
    /// Column holding the parent's height on each link (dendrograms).
    pub height: Option<String>,
}

impl Stat for StatTree {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let (Some(from), Some(to)) = (data.column("from"), data.column("to")) else {
            return DataFrame::new();
        };
        let hcol = self.height.as_deref().and_then(|h| data.column(h));
        let mut index: HashMap<String, usize> = HashMap::new();
        let mut names: Vec<String> = Vec::new();
        let mut id = |v: &Value| -> usize {
            let key = v.key_str().into_owned();
            *index.entry(key.clone()).or_insert_with(|| {
                names.push(key);
                names.len() - 1
            })
        };
        let mut links: Vec<(usize, usize)> = Vec::new();
        let mut height: HashMap<usize, f64> = HashMap::new();
        for i in 0..data.nrows() {
            if from[i].is_na() || to[i].is_na() {
                continue;
            }
            let (p, c) = (id(&from[i]), id(&to[i]));
            if p != c {
                links.push((p, c));
                if let Some(h) = hcol.and_then(|col| col[i].as_f64()) {
                    height.insert(p, h);
                }
            }
        }
        let n = names.len();
        if n == 0 {
            return DataFrame::new();
        }
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut has_parent = vec![false; n];
        for &(p, c) in &links {
            if !has_parent[c] {
                children[p].push(c);
                has_parent[c] = true;
            }
        }
        let roots: Vec<usize> = (0..n).filter(|&i| !has_parent[i]).collect();
        // A cycle has no root: start from the first node.
        let roots = if roots.is_empty() { vec![0] } else { roots };

        let mut x = vec![f64::NAN; n];
        let mut depth = vec![0usize; n];
        let mut next_leaf = 0.0;
        let mut seen = vec![false; n];
        fn place(
            v: usize,
            d: usize,
            children: &[Vec<usize>],
            x: &mut [f64],
            depth: &mut [usize],
            next_leaf: &mut f64,
            seen: &mut [bool],
        ) {
            seen[v] = true;
            depth[v] = d;
            let kids: Vec<usize> = children[v].iter().copied().filter(|&c| !seen[c]).collect();
            if kids.is_empty() {
                x[v] = *next_leaf;
                *next_leaf += 1.0;
                return;
            }
            for &c in &kids {
                place(c, d + 1, children, x, depth, next_leaf, seen);
            }
            x[v] = (x[kids[0]] + x[kids[kids.len() - 1]]) / 2.0;
        }
        for &r in &roots {
            if !seen[r] {
                place(
                    r,
                    0,
                    &children,
                    &mut x,
                    &mut depth,
                    &mut next_leaf,
                    &mut seen,
                );
                next_leaf += 0.5;
            }
        }
        let leaf: Vec<bool> = (0..n).map(|i| children[i].is_empty()).collect();
        let max_depth = depth.iter().copied().max().unwrap_or(0) as f64;
        let level = |i: usize| -> f64 {
            if hcol.is_some() {
                return height.get(&i).copied().unwrap_or(0.0);
            }
            if self.align_leaves && leaf[i] {
                max_depth
            } else {
                depth[i] as f64
            }
        };
        // Down-axis value: larger = further from the root.
        let max_h = (0..n).map(level).fold(0.0, f64::max);
        let down = |i: usize| {
            if hcol.is_some() {
                max_h - level(i)
            } else {
                level(i)
            }
        };
        let span = (next_leaf - 1.5).max(1.0);
        let reach = (0..n).map(down).fold(0.0, f64::max).max(1.0);
        let place_xy = |i: usize| -> (f64, f64) {
            let (along, d) = (x[i], down(i));
            match self.orientation {
                TreeOrientation::TopDown => (along, -d),
                TreeOrientation::LeftRight => (d, -along),
                TreeOrientation::Radial => {
                    // Leaves around the full circle; depth as radius.
                    let a = TAU * along / (span + 1.0);
                    let r = d / reach;
                    (r * a.sin(), r * a.cos())
                }
            }
        };
        let pos: Vec<(f64, f64)> = (0..n).map(place_xy).collect();
        let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for &(px, py) in &pos {
            x0 = x0.min(px);
            x1 = x1.max(px);
            y0 = y0.min(py);
            y1 = y1.max(py);
        }
        let pad = |lo: f64, hi: f64| 0.08 * (hi - lo).max(1e-9) + 0.3;
        let (px, py) = (pad(x0, x1), pad(y0, y1));

        let s = |v: &str| Value::Str(v.to_string());
        let tree_links: Vec<(usize, usize)> = (0..n)
            .flat_map(|p| children[p].iter().map(move |&c| (p, c)))
            .collect();
        let rows = n + tree_links.len();
        let (mut part, mut xs, mut ys, mut xe, mut ye) = (vec![], vec![], vec![], vec![], vec![]);
        let (mut node, mut f, mut t, mut dep, mut lf) = (vec![], vec![], vec![], vec![], vec![]);
        for i in 0..n {
            part.push(s("node"));
            xs.push(pos[i].0);
            ys.push(pos[i].1);
            xe.push(pos[i].0);
            ye.push(pos[i].1);
            node.push(s(&names[i]));
            f.push(s(&names[i]));
            t.push(s(&names[i]));
            dep.push(Value::Float(depth[i] as f64));
            lf.push(Value::Bool(leaf[i]));
        }
        for &(p, c) in &tree_links {
            part.push(s("link"));
            xs.push(pos[p].0);
            ys.push(pos[p].1);
            xe.push(pos[c].0);
            ye.push(pos[c].1);
            node.push(s(&names[c]));
            f.push(s(&names[p]));
            t.push(s(&names[c]));
            dep.push(Value::Float(depth[c] as f64));
            lf.push(Value::Bool(leaf[c]));
        }
        let fl = |v: Vec<f64>| v.into_iter().map(Value::Float).collect::<Vec<_>>();
        let mut out = DataFrame::new();
        out.add_column("part".into(), part);
        out.add_column("x".into(), fl(xs));
        out.add_column("y".into(), fl(ys));
        out.add_column("xend".into(), fl(xe));
        out.add_column("yend".into(), fl(ye));
        out.add_column("node".into(), node);
        out.add_column("from".into(), f);
        out.add_column("to".into(), t);
        out.add_column("depth".into(), dep);
        out.add_column("leaf".into(), lf);
        for (c, v) in [
            ("xmin", x0 - px),
            ("xmax", x1 + px),
            ("ymin", y0 - py),
            ("ymax", y1 + py),
        ] {
            out.add_column(c.into(), vec![Value::Float(v); rows]);
        }
        out
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::From, Aesthetic::To]
    }

    /// The layout needs the whole hierarchy at once.
    fn panelwise(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "tree"
    }
}

/// Parent → child links for a dendrogram from an R `hclust`-style merge
/// matrix: row `k` merges `a` and `b` (negative = leaf `-a`, 1-based;
/// positive = the cluster formed at row `a`) at `heights[k]`. Returns
/// columns `from`, `to` and `height` (the parent's merge height) ready for
/// `geom_dendrogram`; leaves are named by `labels` (1-based leaf `i` →
/// `labels[i - 1]`), clusters `"#k"`.
pub fn dendrogram_links(
    merge: &[(i64, i64)],
    heights: &[f64],
    labels: &[&str],
) -> Vec<(String, Vec<Value>)> {
    let name = |v: i64| -> String {
        if v < 0 {
            labels
                .get((-v - 1) as usize)
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("{}", -v))
        } else {
            format!("#{v}")
        }
    };
    let (mut from, mut to, mut h) = (vec![], vec![], vec![]);
    for (k, &(a, b)) in merge.iter().enumerate() {
        let parent = format!("#{}", k + 1);
        let hk = heights.get(k).copied().unwrap_or(0.0);
        for child in [a, b] {
            from.push(Value::Str(parent.clone()));
            to.push(Value::Str(name(child)));
            h.push(Value::Float(hk));
        }
    }
    vec![
        ("from".into(), from),
        ("to".into(), to),
        ("height".into(), h),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edges(e: &[(&str, &str)]) -> DataFrame {
        let mut d = DataFrame::new();
        d.add_column(
            "from".into(),
            e.iter().map(|r| Value::Str(r.0.into())).collect(),
        );
        d.add_column(
            "to".into(),
            e.iter().map(|r| Value::Str(r.1.into())).collect(),
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
    fn parents_centre_over_children_and_leaves_never_collide() {
        let d = edges(&[
            ("CEO", "CTO"),
            ("CEO", "CFO"),
            ("CTO", "Dev"),
            ("CTO", "Ops"),
            ("CFO", "Acc"),
        ]);
        let out = StatTree::default().compute_group(&d, &ScaleSet::new());
        assert_eq!(out.nrows(), 6 + 5);
        let (x, y) = (col(&out, "x"), col(&out, "y"));
        // Node order: CEO, CTO, CFO, Dev, Ops, Acc.
        assert_eq!(&x[3..6], &[0.0, 1.0, 2.0], "leaves one apart");
        assert_eq!(x[1], 0.5, "CTO centred over Dev/Ops");
        assert_eq!(x[0], (0.5 + 2.0) / 2.0);
        assert_eq!(&y[..3], &[0.0, -1.0, -1.0]);
    }

    #[test]
    fn dendrogram_heights_and_radial() {
        let links = dendrogram_links(&[(-1, -2), (-3, 1)], &[1.0, 2.5], &["a", "b", "c"]);
        let mut d = DataFrame::new();
        for (k, v) in links {
            d.add_column(k, v);
        }
        let out = StatTree {
            height: Some("height".into()),
            ..Default::default()
        }
        .compute_group(&d, &ScaleSet::new());
        let names: Vec<String> = out.column("node").unwrap()[..5]
            .iter()
            .map(|v| v.key_str().into_owned())
            .collect();
        let y = col(&out, "y");
        let at = |n: &str| y[names.iter().position(|m| m == n).unwrap()];
        // Root (#2, h 2.5) on top at 0; #1 (h 1) at -1.5; leaves at -2.5.
        assert_eq!(
            (at("#2"), at("#1"), at("a"), at("c")),
            (0.0, -1.5, -2.5, -2.5)
        );
        let radial = StatTree {
            orientation: TreeOrientation::Radial,
            ..Default::default()
        }
        .compute_group(&d, &ScaleSet::new());
        let (rx, ry) = (col(&radial, "x"), col(&radial, "y"));
        let root = names.iter().position(|m| m == "#2").unwrap();
        assert!(rx[root].hypot(ry[root]) < 1e-12, "root at the centre");
    }
}
