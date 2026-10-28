//! `StatMarimekko`: variable-width column layouts — variwide charts and
//! Marimekko (mosaic) charts.
//!
//! Columns are the distinct `x` values in data order; a column's width is
//! its share of the summed `weight` (1 per row when unmapped). Columns sit
//! side by side over `x ∈ [0, 1]` with an optional gap between them.
//!
//! * Variwide: each column is one bar of height `y` (the first row's).
//! * Mosaic: each column is split by `fill` into stacked segments whose
//!   heights are their share of the column (`y ∈ [0, 1]`).
//!
//! Output: one row per bar / segment with `xmin`/`xmax`/`ymin`/`ymax`,
//! `x` (the column centre), `y` (the bar top / segment share), `column`
//! (the category), `value` (summed weight), `share` (of the column in a
//! mosaic, of the total in a variwide) and the carried `fill`.

use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Distinct `x` keys in data order with their summed weights.
pub(crate) fn column_totals(x: &[Value], weight: Option<&[Value]>) -> Vec<(String, f64)> {
    let mut cols: indexmap::IndexMap<String, f64> = indexmap::IndexMap::new();
    for (i, v) in x.iter().enumerate() {
        if v.is_na() {
            continue;
        }
        let w = weight.map(|c| c[i].as_f64().unwrap_or(0.0)).unwrap_or(1.0);
        *cols.entry(v.key_str().into_owned()).or_insert(0.0) += w.max(0.0);
    }
    cols.into_iter().collect()
}

/// `[xmin, xmax]` per column over `[0, 1]`, widths ∝ `widths`, separated by
/// `gap` (a fraction of the full width per gap).
pub(crate) fn column_spans(widths: &[f64], gap: f64) -> Vec<(f64, f64)> {
    let k = widths.len();
    let total: f64 = widths.iter().sum();
    if k == 0 || total <= 0.0 {
        return vec![(0.0, 0.0); k];
    }
    let gap = gap.clamp(0.0, 0.5 / k.max(1) as f64);
    let avail = 1.0 - gap * (k - 1) as f64;
    let mut x = 0.0;
    widths
        .iter()
        .map(|w| {
            let span = (x, x + avail * w / total);
            x = span.1 + gap;
            span
        })
        .collect()
}

/// One output bar / segment: (row of origin, column index, ymin, ymax,
/// value, share).
type Cell = (usize, usize, f64, f64, f64, f64);

/// Variwide / Marimekko stat behind [`GeomMarimekko`](crate::geom::marimekko::GeomMarimekko).
#[derive(Clone, Debug, Default)]
pub struct StatMarimekko {
    /// `true` for a mosaic (segments by `fill`), `false` for a variwide.
    pub mosaic: bool,
    /// Gap between columns as a fraction of the full width.
    pub gap: f64,
}

impl Stat for StatMarimekko {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let Some(x) = data.column("x") else {
            return DataFrame::new();
        };
        let weight = data.column("weight");
        let totals = column_totals(x, weight);
        let widths: Vec<f64> = totals.iter().map(|c| c.1).collect();
        let spans = column_spans(&widths, self.gap);
        let grand: f64 = widths.iter().sum();
        let w = |i: usize| {
            weight
                .map(|c| c[i].as_f64().unwrap_or(0.0))
                .unwrap_or(1.0)
                .max(0.0)
        };

        let mut rows: Vec<Cell> = Vec::new();
        for (k, (key, total)) in totals.iter().enumerate() {
            let members: Vec<usize> = (0..data.nrows())
                .filter(|&i| !x[i].is_na() && x[i].key_str() == key.as_str())
                .collect();
            if self.mosaic {
                let fill = data.column("fill");
                let mut segs: indexmap::IndexMap<String, (usize, f64)> = indexmap::IndexMap::new();
                for &i in &members {
                    let f = fill
                        .map(|c| c[i].key_str().into_owned())
                        .unwrap_or_default();
                    segs.entry(f).or_insert((i, 0.0)).1 += w(i);
                }
                let mut y = 0.0;
                for (first, v) in segs.into_values() {
                    let share = if *total > 0.0 { v / total } else { 0.0 };
                    rows.push((first, k, y, y + share, v, share));
                    y += share;
                }
            } else if let Some(&first) = members.first() {
                let h = data
                    .column("y")
                    .and_then(|c| c[first].as_f64())
                    .unwrap_or(f64::NAN);
                let share = if grand > 0.0 { total / grand } else { 0.0 };
                rows.push((first, k, 0.0_f64.min(h), 0.0_f64.max(h), *total, share));
            }
        }

        let f =
            |g: &dyn Fn(&Cell) -> f64| rows.iter().map(|r| Value::Float(g(r))).collect::<Vec<_>>();
        let mut out = DataFrame::new();
        out.add_column("xmin".into(), f(&|r| spans[r.1].0));
        out.add_column("xmax".into(), f(&|r| spans[r.1].1));
        out.add_column("ymin".into(), f(&|r| r.2));
        out.add_column("ymax".into(), f(&|r| r.3));
        out.add_column("x".into(), f(&|r| (spans[r.1].0 + spans[r.1].1) / 2.0));
        out.add_column(
            "y".into(),
            // Variwide bars span [min(0, h), max(0, h)], so ymin + ymax = h.
            f(&|r| if self.mosaic { r.5 } else { r.2 + r.3 }),
        );
        out.add_column(
            "column".into(),
            rows.iter()
                .map(|r| Value::Str(totals[r.1].0.clone()))
                .collect(),
        );
        out.add_column("value".into(), f(&|r| r.4));
        out.add_column("share".into(), f(&|r| r.5));
        for name in data.column_names() {
            if out.has_column(name) || name == "weight" {
                continue;
            }
            let col = data.column(name).unwrap_or_default();
            out.add_column(
                name.to_string(),
                rows.iter().map(|r| col[r.0].clone()).collect(),
            );
        }
        out
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X]
    }

    /// Column widths are shares of the panel's total.
    fn panelwise(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "marimekko"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<Value> {
        v.iter().map(|x| Value::Str(x.to_string())).collect()
    }
    fn fl(v: &[f64]) -> Vec<Value> {
        v.iter().map(|&x| Value::Float(x)).collect()
    }
    fn col(d: &DataFrame, c: &str) -> Vec<f64> {
        d.column(c)
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect()
    }

    #[test]
    fn spans_are_proportional_and_gapped() {
        assert_eq!(
            column_spans(&[1.0, 3.0], 0.0),
            vec![(0.0, 0.25), (0.25, 1.0)]
        );
        let g = column_spans(&[1.0, 1.0], 0.1);
        assert!((g[0].1 - 0.45).abs() < 1e-12 && (g[1].0 - 0.55).abs() < 1e-12);
    }

    #[test]
    fn variwide_and_mosaic() {
        let mut d = DataFrame::new();
        d.add_column("x".into(), s(&["A", "B"]));
        d.add_column("y".into(), fl(&[10.0, 4.0]));
        d.add_column("weight".into(), fl(&[1.0, 3.0]));
        let out = StatMarimekko::default().compute_group(&d, &ScaleSet::new());
        assert_eq!(col(&out, "xmax"), vec![0.25, 1.0]);
        assert_eq!(col(&out, "ymax"), vec![10.0, 4.0]);

        let mut m = DataFrame::new();
        m.add_column("x".into(), s(&["A", "A", "B", "B", "B"]));
        m.add_column("fill".into(), s(&["p", "q", "p", "q", "p"]));
        m.add_column("weight".into(), fl(&[1.0, 3.0, 2.0, 4.0, 2.0]));
        let out = StatMarimekko {
            mosaic: true,
            gap: 0.0,
        }
        .compute_group(&m, &ScaleSet::new());
        // A: 4 of 12 → [0, 1/3]; segments p 0.25, q 0.75. B: p 0.5, q 0.5.
        assert_eq!(out.nrows(), 4);
        let (x1, y0, y1) = (col(&out, "xmax"), col(&out, "ymin"), col(&out, "ymax"));
        assert!((x1[0] - 1.0 / 3.0).abs() < 1e-12);
        assert_eq!((y0[1], y1[1]), (0.25, 1.0));
        assert_eq!((y0[3], y1[3]), (0.5, 1.0));
    }
}
