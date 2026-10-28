//! `StatVenn`: area-proportional Venn / Euler layouts.
//!
//! Input rows give set sizes the venn.js way (inclusive): `sets = "A"` is
//! the size of A, `sets = "A&B"` the size of A ∩ B, with the size in
//! `weight`. Each set becomes a circle of area ∝ its size; pairwise centre
//! distances are solved so circle overlaps match the pairwise
//! intersections (exact for two sets), then centres are placed exactly
//! (two or three sets) or by a deterministic stress-minimising descent
//! (four or more). Higher-order intersections are not fitted.
//!
//! Output: one row per set (`part = "circle"`, `x`/`y`/`r`, `set`) and one
//! per labelled region (`part = "label"`, `x`/`y` at the region's deepest
//! interior point found by grid search, `text` = the set name for a set's
//! own region or the size for an intersection row), plus `xmin`/`xmax`/
//! `ymin`/`ymax` framing the diagram as a square.

use std::f64::consts::PI;

use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Area of the lens where circles of radius `r1`, `r2` at distance `d` overlap.
pub(crate) fn lens_area(r1: f64, r2: f64, d: f64) -> f64 {
    if d >= r1 + r2 {
        return 0.0;
    }
    if d <= (r1 - r2).abs() {
        return PI * r1.min(r2).powi(2);
    }
    let a = r1
        * r1
        * ((d * d + r1 * r1 - r2 * r2) / (2.0 * d * r1))
            .clamp(-1.0, 1.0)
            .acos();
    let b = r2
        * r2
        * ((d * d + r2 * r2 - r1 * r1) / (2.0 * d * r2))
            .clamp(-1.0, 1.0)
            .acos();
    let c = 0.5
        * ((-d + r1 + r2) * (d + r1 - r2) * (d - r1 + r2) * (d + r1 + r2))
            .max(0.0)
            .sqrt();
    a + b - c
}

/// Centre distance giving an overlap of `overlap` (bisection; the lens area
/// falls monotonically with distance).
pub(crate) fn distance_for_overlap(r1: f64, r2: f64, overlap: f64) -> f64 {
    let full = PI * r1.min(r2).powi(2);
    if overlap <= 0.0 {
        return r1 + r2 + 0.05 * (r1 + r2);
    }
    if overlap >= full {
        return (r1 - r2).abs();
    }
    let (mut lo, mut hi) = ((r1 - r2).abs(), r1 + r2);
    for _ in 0..100 {
        let mid = (lo + hi) / 2.0;
        if lens_area(r1, r2, mid) > overlap {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

/// Venn layout stat behind [`GeomVenn`](crate::geom::venn::GeomVenn).
#[derive(Clone, Debug, Default)]
pub struct StatVenn;

impl Stat for StatVenn {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let Some(sets_col) = data.column("sets") else {
            return DataFrame::new();
        };
        let weight = data.column("weight");
        let parse = |v: &Value| -> Vec<String> {
            let mut s: Vec<String> = v
                .key_str()
                .split('&')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect();
            s.sort();
            s.dedup();
            s
        };
        // Sets in first-appearance order; sizes per combination.
        let mut names: Vec<String> = Vec::new();
        let mut combos: Vec<(Vec<String>, f64)> = Vec::new();
        for i in 0..data.nrows() {
            let combo = parse(&sets_col[i]);
            let size = match weight {
                Some(c) => c[i].as_f64().unwrap_or(f64::NAN),
                None => 1.0,
            };
            if combo.is_empty() || !size.is_finite() || size < 0.0 {
                continue;
            }
            for s in &combo {
                if !names.contains(s) {
                    names.push(s.clone());
                }
            }
            combos.push((combo, size));
        }
        let n = names.len();
        if n == 0 {
            return DataFrame::new();
        }
        let idx = |s: &str| names.iter().position(|m| m == s).unwrap();
        let mut size = vec![0.0; n];
        let mut pair = vec![vec![0.0; n]; n];
        for (combo, s) in &combos {
            match combo.as_slice() {
                [a] => size[idx(a)] = *s,
                [a, b] => {
                    let (i, j) = (idx(a), idx(b));
                    pair[i][j] = *s;
                    pair[j][i] = *s;
                }
                _ => {}
            }
        }
        // A set given only through intersections gets at least their size.
        for i in 0..n {
            let implied = (0..n).map(|j| pair[i][j]).fold(0.0, f64::max);
            size[i] = size[i].max(implied).max(1e-9);
        }
        let r: Vec<f64> = size.iter().map(|s| (s / PI).sqrt()).collect();
        let mut d = vec![vec![0.0; n]; n];
        for i in 0..n {
            for j in i + 1..n {
                let dij = distance_for_overlap(r[i], r[j], pair[i][j].min(size[i]).min(size[j]));
                d[i][j] = dij;
                d[j][i] = dij;
            }
        }
        let mut pos = vec![(0.0, 0.0); n];
        if n >= 2 {
            pos[1] = (d[0][1], 0.0);
        }
        if n >= 3 {
            // Third centre by the law of cosines (clamped when the three
            // distances can't form a triangle).
            let (a, b, c) = (d[0][1], d[0][2], d[1][2]);
            let x = if a > 0.0 {
                (a * a + b * b - c * c) / (2.0 * a)
            } else {
                0.0
            };
            pos[2] = (x, -(b * b - x * x).max(0.0).sqrt());
        }
        if n >= 4 {
            // Start the rest on a circle, then descend on the stress
            // Σ (|pi − pj| − dij)².
            let spread = d.iter().flatten().fold(0.0f64, |m, &v| m.max(v));
            for (k, p) in pos.iter_mut().enumerate().skip(3) {
                let a = 2.0 * PI * k as f64 / n as f64;
                *p = (spread * a.cos() / 2.0, spread * a.sin() / 2.0);
            }
            for it in 0..2000 {
                let step = 0.05 * (1.0 - it as f64 / 2000.0) + 0.002;
                let mut grad = vec![(0.0, 0.0); n];
                for i in 0..n {
                    for j in 0..n {
                        if i == j {
                            continue;
                        }
                        let (dx, dy) = (pos[i].0 - pos[j].0, pos[i].1 - pos[j].1);
                        let dist = dx.hypot(dy).max(1e-9);
                        let e = (dist - d[i][j]) / dist;
                        grad[i].0 += e * dx;
                        grad[i].1 += e * dy;
                    }
                }
                for i in 0..n {
                    pos[i].0 -= step * grad[i].0;
                    pos[i].1 -= step * grad[i].1;
                }
            }
        }
        // Centre the diagram.
        let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for k in 0..n {
            x0 = x0.min(pos[k].0 - r[k]);
            x1 = x1.max(pos[k].0 + r[k]);
            y0 = y0.min(pos[k].1 - r[k]);
            y1 = y1.max(pos[k].1 + r[k]);
        }
        let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        for p in pos.iter_mut() {
            p.0 -= cx;
            p.1 -= cy;
        }
        let half = ((x1 - x0).max(y1 - y0) / 2.0) * 1.08;

        // Region labels: deepest grid point inside exactly the combination's
        // sets (and outside the others).
        let inside = |p: (f64, f64), k: usize| (p.0 - pos[k].0).hypot(p.1 - pos[k].1) < r[k];
        let margin =
            |p: (f64, f64), k: usize| (r[k] - (p.0 - pos[k].0).hypot(p.1 - pos[k].1)).abs();
        let grid = 80;
        let mut labels: Vec<(f64, f64, String, usize)> = Vec::new();
        for (combo, s) in &combos {
            let members: Vec<usize> = combo.iter().map(|c| idx(c)).collect();
            let mut best: Option<((f64, f64), f64)> = None;
            for gx in 0..=grid {
                for gy in 0..=grid {
                    let p = (
                        -half + 2.0 * half * gx as f64 / grid as f64,
                        -half + 2.0 * half * gy as f64 / grid as f64,
                    );
                    if !(0..n).all(|k| inside(p, k) == members.contains(&k)) {
                        continue;
                    }
                    let m = (0..n).map(|k| margin(p, k)).fold(f64::INFINITY, f64::min);
                    if best.is_none_or(|b| m > b.1) {
                        best = Some((p, m));
                    }
                }
            }
            if let Some((p, _)) = best {
                let text = if members.len() == 1 {
                    names[members[0]].clone()
                } else {
                    crate::format::format_value(&Value::Float(*s))
                };
                labels.push((p.0, p.1, text, members[0]));
            }
        }

        let rows = n + labels.len();
        let s = |v: &str| Value::Str(v.to_string());
        let mut part = vec![s("circle"); n];
        part.extend(std::iter::repeat_n(s("label"), labels.len()));
        let xs: Vec<Value> = pos
            .iter()
            .map(|p| Value::Float(p.0))
            .chain(labels.iter().map(|l| Value::Float(l.0)))
            .collect();
        let ys: Vec<Value> = pos
            .iter()
            .map(|p| Value::Float(p.1))
            .chain(labels.iter().map(|l| Value::Float(l.1)))
            .collect();
        let rs: Vec<Value> = r
            .iter()
            .map(|&v| Value::Float(v))
            .chain(std::iter::repeat_n(Value::Float(0.0), labels.len()))
            .collect();
        // A label row carries its region's first set, so a fill mapped on
        // `set` never trains an NA level.
        let set: Vec<Value> = names
            .iter()
            .map(|nm| s(nm))
            .chain(labels.iter().map(|l| s(&names[l.3])))
            .collect();
        let text: Vec<Value> = names
            .iter()
            .map(|_| Value::Na)
            .chain(labels.iter().map(|l| s(&l.2)))
            .collect();
        let sizes: Vec<Value> = size
            .iter()
            .map(|&v| Value::Float(v))
            .chain(std::iter::repeat_n(Value::Na, labels.len()))
            .collect();
        let mut out = DataFrame::new();
        out.add_column("part".into(), part);
        out.add_column("x".into(), xs);
        out.add_column("y".into(), ys);
        out.add_column("r".into(), rs);
        out.add_column("set".into(), set);
        out.add_column("text".into(), text);
        out.add_column("size".into(), sizes);
        for (c, v) in [
            ("xmin", -half),
            ("xmax", half),
            ("ymin", -half),
            ("ymax", half),
        ] {
            out.add_column(c.into(), vec![Value::Float(v); rows]);
        }
        out
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Sets]
    }

    /// All sets of the panel form one diagram.
    fn panelwise(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "venn"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(rows: &[(&str, f64)]) -> DataFrame {
        let mut d = DataFrame::new();
        d.add_column(
            "sets".into(),
            rows.iter().map(|r| Value::Str(r.0.into())).collect(),
        );
        d.add_column(
            "weight".into(),
            rows.iter().map(|r| Value::Float(r.1)).collect(),
        );
        d
    }

    fn f(d: &DataFrame, c: &str) -> Vec<f64> {
        d.column(c)
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap_or(f64::NAN))
            .collect()
    }

    #[test]
    fn two_sets_overlap_exactly() {
        let out = StatVenn.compute_group(
            &data(&[("A", 100.0), ("B", 60.0), ("A&B", 25.0)]),
            &ScaleSet::new(),
        );
        let (x, y, r) = (f(&out, "x"), f(&out, "y"), f(&out, "r"));
        assert!((PI * r[0] * r[0] - 100.0).abs() < 1e-9);
        let d = (x[0] - x[1]).hypot(y[0] - y[1]);
        assert!((lens_area(r[0], r[1], d) - 25.0).abs() < 1e-6);
        // Labels: A, B and the intersection size.
        let texts: Vec<String> = out.column("text").unwrap()[2..]
            .iter()
            .map(|v| v.key_str().into_owned())
            .collect();
        assert_eq!(texts, ["A", "B", "25"]);
    }

    #[test]
    fn three_and_four_sets_fit_pairwise_overlaps() {
        let rows = [
            ("A", 50.0),
            ("B", 50.0),
            ("C", 50.0),
            ("A&B", 10.0),
            ("A&C", 10.0),
            ("B&C", 10.0),
        ];
        let out = StatVenn.compute_group(&data(&rows), &ScaleSet::new());
        let (x, y, r) = (f(&out, "x"), f(&out, "y"), f(&out, "r"));
        for (i, j) in [(0, 1), (0, 2), (1, 2)] {
            let d = (x[i] - x[j]).hypot(y[i] - y[j]);
            assert!((lens_area(r[i], r[j], d) - 10.0).abs() < 1e-6, "{i}{j}");
        }
        let four = [
            ("A", 40.0),
            ("B", 40.0),
            ("C", 40.0),
            ("D", 40.0),
            ("A&B", 8.0),
            ("B&C", 8.0),
            ("C&D", 8.0),
            ("A&D", 8.0),
        ];
        let out = StatVenn.compute_group(&data(&four), &ScaleSet::new());
        let (x, y, r) = (f(&out, "x"), f(&out, "y"), f(&out, "r"));
        for (i, j) in [(0, 1), (1, 2), (2, 3), (0, 3)] {
            let d = (x[i] - x[j]).hypot(y[i] - y[j]);
            let got = lens_area(r[i], r[j], d);
            assert!((got - 8.0).abs() < 3.0, "{i}{j}: {got}");
        }
    }
}
