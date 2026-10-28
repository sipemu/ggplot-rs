//! `StatTreemap`: squarified treemap layout (Bruls, Huizing & van Wijk).
//!
//! Each input row with a positive `weight` becomes one tile whose area is
//! proportional to that weight. With a `subgroup` mapping, groups are laid
//! out first (area = their summed weight) and each group's rows are tiled
//! inside its rectangle (inset by `group_padding`). The layout fills
//! `[0, aspect] × [0, 1]`.
//!
//! Output: one row per tile (`part = "tile"`, every input column carried
//! through, `xmin`/`xmax`/`ymin`/`ymax`, `area` = weight) followed — when
//! subgroups are mapped — by one row per group (`part = "group"`, its
//! rectangle, `subgroup`, `area` = group total; other columns `Na`).

use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Rectangle `(x, y, w, h)` with `(x, y)` its lower-left corner.
pub(crate) type Rect = (f64, f64, f64, f64);

/// Squarified treemap stat behind [`GeomTreemap`](crate::geom::treemap::GeomTreemap).
#[derive(Clone, Debug)]
pub struct StatTreemap {
    /// Layout width / height (match the panel's so tiles come out square).
    pub aspect: f64,
    /// Inset of each group's tiles inside the group rectangle, in layout units.
    pub group_padding: f64,
}

impl Default for StatTreemap {
    fn default() -> Self {
        StatTreemap {
            aspect: 1.6,
            group_padding: 0.006,
        }
    }
}

/// Worst aspect ratio of a row of areas laid along a side of length `side`.
fn worst(row: &[f64], side: f64) -> f64 {
    let sum: f64 = row.iter().sum();
    let (mx, mn) = row
        .iter()
        .fold((f64::MIN, f64::MAX), |(a, b), &v| (a.max(v), b.min(v)));
    let (s2, sum2) = (side * side, sum * sum);
    f64::max(s2 * mx / sum2, sum2 / (s2 * mn))
}

/// Squarified layout of `values` (any order; positive) inside `rect`.
/// Returns one rectangle per value, in input order. Rows are laid along the
/// shorter side, starting at the top-left.
pub(crate) fn squarify(values: &[f64], rect: Rect) -> Vec<Rect> {
    let mut out = vec![(0.0, 0.0, 0.0, 0.0); values.len()];
    let total: f64 = values.iter().sum();
    let (_, _, w0, h0) = rect;
    if values.is_empty() || total <= 0.0 || w0 <= 0.0 || h0 <= 0.0 {
        return out;
    }
    let scale = w0 * h0 / total;
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|&a, &b| values[b].total_cmp(&values[a]));
    let areas: Vec<f64> = order.iter().map(|&i| values[i] * scale).collect();

    let (mut x, y, mut w, mut h) = rect;
    let mut start = 0;
    while start < areas.len() {
        let side = w.min(h);
        let mut end = start + 1;
        while end < areas.len()
            && worst(&areas[start..=end], side) <= worst(&areas[start..end], side)
        {
            end += 1;
        }
        let row = &areas[start..end];
        let sum: f64 = row.iter().sum();
        if w >= h {
            // Vertical strip on the left, filled top to bottom.
            let sw = if h > 0.0 { sum / h } else { 0.0 };
            let mut top = y + h;
            for (k, a) in row.iter().enumerate() {
                let th = if sw > 0.0 { a / sw } else { 0.0 };
                out[order[start + k]] = (x, top - th, sw, th);
                top -= th;
            }
            x += sw;
            w -= sw;
        } else {
            // Horizontal strip along the top, filled left to right.
            let sh = if w > 0.0 { sum / w } else { 0.0 };
            let mut left = x;
            for (k, a) in row.iter().enumerate() {
                let tw = if sh > 0.0 { a / sh } else { 0.0 };
                out[order[start + k]] = (left, y + h - sh, tw, sh);
                left += tw;
            }
            h -= sh;
        }
        start = end;
    }
    out
}

impl Stat for StatTreemap {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let Some(weight) = data.column("weight") else {
            return DataFrame::new();
        };
        let rows: Vec<usize> = (0..data.nrows())
            .filter(|&i| weight[i].as_f64().is_some_and(|w| w.is_finite() && w > 0.0))
            .collect();
        if rows.is_empty() {
            return DataFrame::new();
        }
        let w = |i: usize| weight[i].as_f64().unwrap_or(0.0);
        let full: Rect = (0.0, 0.0, self.aspect.max(1e-6), 1.0);
        let mut tile = vec![(0.0, 0.0, 0.0, 0.0); data.nrows()];
        let mut groups: Vec<(Value, f64, Rect)> = Vec::new();

        if let Some(sub) = data.column("subgroup") {
            let mut members: indexmap::IndexMap<String, (Value, Vec<usize>)> =
                indexmap::IndexMap::new();
            for &i in &rows {
                members
                    .entry(sub[i].key_str().into_owned())
                    .or_insert_with(|| (sub[i].clone(), Vec::new()))
                    .1
                    .push(i);
            }
            let totals: Vec<f64> = members
                .values()
                .map(|(_, m)| m.iter().map(|&i| w(i)).sum())
                .collect();
            let rects = squarify(&totals, full);
            let p = self.group_padding;
            for (((val, m), total), r) in members.into_values().zip(&totals).zip(rects) {
                let inner = (
                    r.0 + p,
                    r.1 + p,
                    (r.2 - 2.0 * p).max(0.0),
                    (r.3 - 2.0 * p).max(0.0),
                );
                let vals: Vec<f64> = m.iter().map(|&i| w(i)).collect();
                for (&i, t) in m.iter().zip(squarify(&vals, inner)) {
                    tile[i] = t;
                }
                groups.push((val, *total, r));
            }
        } else {
            let vals: Vec<f64> = rows.iter().map(|&i| w(i)).collect();
            for (&i, t) in rows.iter().zip(squarify(&vals, full)) {
                tile[i] = t;
            }
        }

        let n = rows.len() + groups.len();
        let mut out = DataFrame::new();
        let mut part = vec![Value::Str("tile".into()); rows.len()];
        part.extend(std::iter::repeat_n(
            Value::Str("group".into()),
            groups.len(),
        ));
        out.add_column("part".into(), part);
        let edge = |f: fn(&Rect) -> f64| -> Vec<Value> {
            rows.iter()
                .map(|&i| Value::Float(f(&tile[i])))
                .chain(groups.iter().map(|g| Value::Float(f(&g.2))))
                .collect()
        };
        out.add_column("xmin".into(), edge(|r| r.0));
        out.add_column("xmax".into(), edge(|r| r.0 + r.2));
        out.add_column("ymin".into(), edge(|r| r.1));
        out.add_column("ymax".into(), edge(|r| r.1 + r.3));
        out.add_column(
            "area".into(),
            rows.iter()
                .map(|&i| Value::Float(w(i)))
                .chain(groups.iter().map(|g| Value::Float(g.1)))
                .collect(),
        );
        for name in data.column_names() {
            if out.has_column(name) {
                continue;
            }
            let col = data.column(name).unwrap_or_default();
            let mut vals: Vec<Value> = rows.iter().map(|&i| col[i].clone()).collect();
            if name == "subgroup" {
                vals.extend(groups.iter().map(|g| g.0.clone()));
            } else {
                vals.extend(std::iter::repeat_n(Value::Na, groups.len()));
            }
            debug_assert_eq!(vals.len(), n);
            out.add_column(name.to_string(), vals);
        }
        out
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Weight]
    }

    /// Tiles share one rectangle: every row of the panel is laid out at once.
    fn panelwise(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "treemap"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn squarify_tiles_exactly_and_keeps_areas() {
        let vals = [6.0, 6.0, 4.0, 3.0, 2.0, 2.0, 1.0];
        let rect = (0.0, 0.0, 6.0, 4.0);
        let r = squarify(&vals, rect);
        let total: f64 = r.iter().map(|t| t.2 * t.3).sum();
        assert!((total - 24.0).abs() < 1e-9);
        for (v, t) in vals.iter().zip(&r) {
            assert!((t.2 * t.3 - v).abs() < 1e-9, "{t:?}");
            assert!(t.0 >= -1e-9 && t.0 + t.2 <= 6.0 + 1e-9);
            assert!(t.1 >= -1e-9 && t.1 + t.3 <= 4.0 + 1e-9);
        }
        // Bruls et al.'s example: worst aspect ratio stays modest.
        let worst = r
            .iter()
            .map(|t| (t.2 / t.3).max(t.3 / t.2))
            .fold(0.0, f64::max);
        assert!(worst < 3.0, "{worst}");
        // No two tiles overlap.
        for i in 0..r.len() {
            for j in i + 1..r.len() {
                let (a, b) = (r[i], r[j]);
                let ox = (a.0 + a.2).min(b.0 + b.2) - a.0.max(b.0);
                let oy = (a.1 + a.3).min(b.1 + b.3) - a.1.max(b.1);
                assert!(ox <= 1e-9 || oy <= 1e-9, "{a:?} overlaps {b:?}");
            }
        }
    }

    #[test]
    fn subgroups_nest_tiles_inside_group_rects() {
        let mut d = DataFrame::new();
        let s = |v: &[&str]| v.iter().map(|x| Value::Str(x.to_string())).collect();
        d.add_column("label".into(), s(&["a", "b", "c", "d", "e"]));
        d.add_column("subgroup".into(), s(&["G1", "G1", "G2", "G2", "G2"]));
        d.add_column(
            "weight".into(),
            [5.0, 3.0, 4.0, 2.0, 2.0]
                .iter()
                .map(|&v| Value::Float(v))
                .collect(),
        );
        let out = StatTreemap::default().compute_group(&d, &ScaleSet::new());
        assert_eq!(out.nrows(), 5 + 2);
        let f = |c: &str| -> Vec<f64> {
            out.column(c)
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect()
        };
        let (x0, x1, y0, y1) = (f("xmin"), f("xmax"), f("ymin"), f("ymax"));
        let sub = |i: usize| out.column("subgroup").unwrap()[i].key_str().into_owned();
        for t in 0..5 {
            let g = if sub(t) == "G1" { 5 } else { 6 };
            assert!(x0[t] >= x0[g] && x1[t] <= x1[g] && y0[t] >= y0[g] && y1[t] <= y1[g]);
        }
        // G1 holds 8 of 16 units: half of the 1.6 × 1 layout.
        let ga = (x1[5] - x0[5]) * (y1[5] - y0[5]);
        assert!((ga - 0.8).abs() < 1e-9, "{ga}");
        assert_eq!(out.column("label").unwrap()[5], Value::Na);
    }
}
