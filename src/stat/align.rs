//! `stat_align` — the default stat of `geom_area` (ggplot2 ≥ 3.4).
//!
//! Stacking adds y values that share an x. Series sampled at different x
//! would then stack on gaps (a series above one that has no row at some x
//! dips there). `stat_align` interpolates every group onto one shared set of
//! x positions first, so stacked areas stay continuous (#172).
//!
//! As ggplot2's `StatAlign`:
//! * one group in the panel: the data passes through unchanged;
//! * the shared positions are every x of the panel plus each group's zero
//!   crossings, and a tiny step `adjust` either side of each (0.1 % of the x
//!   range, at most a third of the smallest gap between positions);
//! * each group is linearly interpolated at the positions inside its own x
//!   range, and padded with `y = 0` one `adjust` beyond both ends;
//! * a group with a single distinct x is dropped; within a group, repeated x
//!   keep their first and last row, the first moved one `adjust` left (a
//!   vertical step);
//! * every other column is carried from the group's first row.
use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Interpolates every group of a panel onto shared x positions (see the
/// module docs).
#[derive(Debug, Clone, Copy, Default)]
pub struct StatAlign;

impl Stat for StatAlign {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        align_panel(data)
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn panelwise(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "align"
    }
}

/// One group's rows: (x, y) sorted by x, and the row the other columns are
/// carried from.
struct Group {
    pts: Vec<(f64, f64)>,
    first: usize,
}

fn align_panel(data: &DataFrame) -> DataFrame {
    let (Some(xs), Some(ys)) = (data.column("x"), data.column("y")) else {
        return data.clone();
    };
    // A discrete x (or any value that isn't a number): nothing to align.
    let xf: Option<Vec<f64>> = xs.iter().map(Value::as_f64).collect();
    let yf: Option<Vec<f64>> = ys.iter().map(Value::as_f64).collect();
    let (Some(xf), Some(yf)) = (xf, yf) else {
        return data.clone();
    };
    let keys: Vec<String> = crate::build::detect_group_columns(data)
        .into_iter()
        .filter(|c| c != "x")
        .collect();
    // Rows of each group, in first-appearance order.
    let mut order: Vec<String> = Vec::new();
    let mut rows: std::collections::HashMap<String, Vec<usize>> = Default::default();
    for i in 0..data.nrows() {
        let k = keys
            .iter()
            .map(|c| {
                data.column(c)
                    .map_or(String::new(), |v| v[i].to_group_key())
            })
            .collect::<Vec<_>>()
            .join("\u{1f}");
        rows.entry(k.clone())
            .or_insert_with(|| {
                order.push(k.clone());
                Vec::new()
            })
            .push(i);
    }
    if order.len() < 2 {
        return data.clone();
    }
    let groups: Vec<Group> = order
        .iter()
        .map(|k| {
            let ix = &rows[k];
            let mut pts: Vec<(f64, f64)> = ix.iter().map(|&i| (xf[i], yf[i])).collect();
            pts.sort_by(|a, b| a.0.total_cmp(&b.0));
            Group { pts, first: ix[0] }
        })
        .collect();

    // The shared positions: every x, plus each group's zero crossings.
    let mut locs: Vec<f64> = xf.iter().copied().filter(|x| x.is_finite()).collect();
    for g in &groups {
        for w in g.pts.windows(2) {
            let ((x0, y0), (x1, y1)) = (w[0], w[1]);
            if (y0 < 0.0) != (y1 < 0.0) && y1 != y0 {
                locs.push(x0 - y0 * (x1 - x0) / (y1 - y0));
            }
        }
    }
    sort_dedup(&mut locs);
    if locs.len() < 2 {
        return data.clone();
    }
    let range = locs[locs.len() - 1] - locs[0];
    let min_gap = locs
        .windows(2)
        .map(|w| w[1] - w[0])
        .fold(f64::INFINITY, f64::min);
    let adjust = (range * 0.001).min(min_gap / 3.0);
    let mut all: Vec<f64> = locs
        .iter()
        .flat_map(|&l| [l - adjust, l, l + adjust])
        .collect();
    sort_dedup(&mut all);

    let datetime = xs.iter().all(Value::is_datetime);
    let as_x = |v: f64| {
        if datetime {
            Value::DateTime(v.round() as i64)
        } else {
            Value::Float(v)
        }
    };
    let names: Vec<String> = data
        .column_names()
        .into_iter()
        .filter(|n| *n != "x" && *n != "y")
        .map(str::to_string)
        .collect();
    let mut out_x: Vec<Value> = Vec::new();
    let mut out_y: Vec<Value> = Vec::new();
    let mut out_rest: Vec<Vec<Value>> = vec![Vec::new(); names.len()];
    for g in &groups {
        if g.pts.iter().all(|p| p.0 == g.pts[0].0) {
            continue; // a single distinct x: ggplot2 drops the group
        }
        let pts = dedup_steps(&g.pts, adjust);
        let (lo, hi) = (pts[0].0, pts[pts.len() - 1].0);
        let mut gx = vec![lo - adjust];
        let mut gy = vec![0.0];
        for &l in all.iter().filter(|&&l| l >= lo && l <= hi) {
            gx.push(l);
            gy.push(interpolate(&pts, l));
        }
        gx.push(hi + adjust);
        gy.push(0.0);
        for (x, y) in gx.into_iter().zip(gy) {
            out_x.push(as_x(x));
            out_y.push(Value::Float(y));
            for (c, n) in names.iter().enumerate() {
                out_rest[c].push(data.column(n).map_or(Value::Na, |v| v[g.first].clone()));
            }
        }
    }
    let mut out = DataFrame::new();
    out.add_column("x".into(), out_x);
    out.add_column("y".into(), out_y);
    for (n, v) in names.into_iter().zip(out_rest) {
        out.add_column(n, v);
    }
    out
}

fn sort_dedup(v: &mut Vec<f64>) {
    v.retain(|x| x.is_finite());
    v.sort_by(f64::total_cmp);
    v.dedup();
}

/// Repeated x within a group: keep the first and last row, the first moved
/// `adjust` to the left (ggplot2 draws a vertical step there).
fn dedup_steps(pts: &[(f64, f64)], adjust: f64) -> Vec<(f64, f64)> {
    let mut out = Vec::with_capacity(pts.len());
    let mut i = 0;
    while i < pts.len() {
        let mut j = i;
        while j + 1 < pts.len() && pts[j + 1].0 == pts[i].0 {
            j += 1;
        }
        if j == i {
            out.push(pts[i]);
        } else {
            out.push((pts[i].0 - adjust, pts[i].1));
            out.push(pts[j]);
        }
        i = j + 1;
    }
    out
}

/// Linear interpolation at `x` (inside the points' range; the points are
/// sorted by x with no ties).
fn interpolate(pts: &[(f64, f64)], x: f64) -> f64 {
    let k = pts.partition_point(|p| p.0 < x);
    if k < pts.len() && pts[k].0 == x {
        return pts[k].1;
    }
    if k == 0 || k == pts.len() {
        return f64::NAN; // outside (not reached: callers stay in range)
    }
    let ((x0, y0), (x1, y1)) = (pts[k - 1], pts[k]);
    y0 + (y1 - y0) * (x - x0) / (x1 - x0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel(rows: &[(f64, f64, &str)]) -> DataFrame {
        let mut d = DataFrame::new();
        d.add_column("x".into(), rows.iter().map(|r| Value::Float(r.0)).collect());
        d.add_column("y".into(), rows.iter().map(|r| Value::Float(r.1)).collect());
        d.add_column(
            "fill".into(),
            rows.iter().map(|r| Value::Str(r.2.into())).collect(),
        );
        d
    }
    fn col(d: &DataFrame, n: &str) -> Vec<f64> {
        d.column(n)
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect()
    }

    #[test]
    fn matches_ggplot2_on_a_gap() {
        // ggplot2 4.0.3, layer_data(ggplot(d, aes(x, y, fill = g)) +
        // geom_area()) before stacking: a = 1 at x 1..4, b = 2 at x 1, 3, 4.
        let d = panel(&[
            (1.0, 1.0, "a"),
            (2.0, 1.0, "a"),
            (3.0, 1.0, "a"),
            (4.0, 1.0, "a"),
            (1.0, 2.0, "b"),
            (3.0, 2.0, "b"),
            (4.0, 2.0, "b"),
        ]);
        let out = align_panel(&d);
        let x = col(&out, "x");
        let y = col(&out, "y");
        let want_x = [
            0.997, 1.0, 1.003, 1.997, 2.0, 2.003, 2.997, 3.0, 3.003, 3.997, 4.0, 4.003,
        ];
        assert_eq!(out.nrows(), 24);
        for (i, w) in want_x.iter().enumerate() {
            assert!((x[i] - w).abs() < 1e-9, "a x[{i}] = {}", x[i]);
            assert!((x[12 + i] - w).abs() < 1e-9, "b x[{i}] = {}", x[12 + i]);
        }
        // Padding rows are 0; b is 2 everywhere in between, including x = 2
        // where it has no row of its own.
        assert_eq!((y[0], y[11], y[12], y[23]), (0.0, 0.0, 0.0, 0.0));
        assert!(y[1..11].iter().all(|&v| v == 1.0));
        assert!(y[13..23].iter().all(|&v| v == 2.0));
        assert_eq!(out.column("fill").unwrap()[13].as_str(), Some("b"));
    }

    #[test]
    fn one_group_passes_through() {
        let d = panel(&[(1.0, 1.0, "a"), (3.0, 4.0, "a")]);
        let out = align_panel(&d);
        assert_eq!(col(&out, "x"), vec![1.0, 3.0]);
        assert_eq!(col(&out, "y"), vec![1.0, 4.0]);
    }

    #[test]
    fn interpolates_and_adds_zero_crossings() {
        // a crosses zero between x = 0 and 2 (at 1); b only has x 0 and 2.
        let d = panel(&[
            (0.0, -1.0, "a"),
            (2.0, 1.0, "a"),
            (0.0, 1.0, "b"),
            (2.0, 3.0, "b"),
        ]);
        let out = align_panel(&d);
        let (x, y) = (col(&out, "x"), col(&out, "y"));
        let at = |g: usize, want: f64| {
            let off = if g == 0 { 0 } else { out.nrows() / 2 };
            let i = (off..off + out.nrows() / 2)
                .find(|&i| (x[i] - want).abs() < 1e-12)
                .unwrap();
            y[i]
        };
        assert_eq!(at(0, 1.0), 0.0, "a's zero crossing is a shared position");
        assert_eq!(at(1, 1.0), 2.0, "b interpolated there");
    }

    #[test]
    fn a_single_x_group_is_dropped_and_steps_kept() {
        let d = panel(&[
            (1.0, 5.0, "a"),
            (1.0, 6.0, "a"),
            (0.0, 1.0, "b"),
            (2.0, 1.0, "b"),
        ]);
        let out = align_panel(&d);
        assert!(out
            .column("fill")
            .unwrap()
            .iter()
            .all(|v| v.as_str() == Some("b")));
        let steps = dedup_steps(&[(0.0, 1.0), (1.0, 2.0), (1.0, 3.0), (2.0, 1.0)], 0.1);
        assert_eq!(steps, vec![(0.0, 1.0), (0.9, 2.0), (1.0, 3.0), (2.0, 1.0)]);
    }

    #[test]
    fn datetimes_stay_datetimes() {
        let mut d = DataFrame::new();
        d.add_column(
            "x".into(),
            vec![
                Value::DateTime(0),
                Value::DateTime(86_400),
                Value::DateTime(0),
                Value::DateTime(172_800),
            ],
        );
        d.add_column("y".into(), vec![Value::Float(1.0); 4]);
        d.add_column(
            "fill".into(),
            vec![
                Value::Str("a".into()),
                Value::Str("a".into()),
                Value::Str("b".into()),
                Value::Str("b".into()),
            ],
        );
        let out = align_panel(&d);
        assert!(out.column("x").unwrap().iter().all(Value::is_datetime));
    }
}
