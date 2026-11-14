//! `StatPareto`: sum `y` per category, sort descending and accumulate — the
//! data behind a Pareto chart's bars and cumulative-percentage line.

use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Pareto stat behind [`GeomPareto`](crate::geom::pareto::GeomPareto).
///
/// Output columns, one row per distinct x (largest first when `sort`):
/// `x`, `y` (the category's summed value), `cum` (running total), `cum_pct`
/// (running share of the grand total, 0–100), `ymin` (0) and `ymax`
/// (= `cum`, so the y scale trains up to the grand total).
#[derive(Clone, Debug)]
pub struct StatPareto {
    /// Order categories by descending value (ties keep data order). With
    /// `false` the data order is kept.
    pub sort: bool,
    /// The grand total the running share is of (a top N of a longer list:
    /// the line ends at the top N's share, not 100 %); `None` is the sum
    /// of the rows. The y scale trains up to it.
    pub total: Option<f64>,
}

impl Default for StatPareto {
    fn default() -> Self {
        StatPareto {
            sort: true,
            total: None,
        }
    }
}

impl Stat for StatPareto {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let (Some(x_col), Some(y_col)) = (data.column("x"), data.column("y")) else {
            return DataFrame::new();
        };
        let mut cats: indexmap::IndexMap<String, (Value, f64)> = indexmap::IndexMap::new();
        for (x, y) in x_col.iter().zip(y_col) {
            let Some(v) = y.as_f64().filter(|v| v.is_finite()) else {
                continue;
            };
            cats.entry(x.key_str().into_owned())
                .or_insert_with(|| (x.clone(), 0.0))
                .1 += v;
        }
        let mut rows: Vec<(Value, f64)> = cats.into_values().collect();
        if self.sort {
            rows.sort_by(|a, b| b.1.total_cmp(&a.1));
        }
        let sum: f64 = rows.iter().map(|r| r.1).sum();
        let total = self
            .total
            .filter(|t| t.is_finite() && *t > 0.0)
            .unwrap_or(sum);
        let mut cum = 0.0;
        let (mut xs, mut ys, mut cums, mut pcts) = (vec![], vec![], vec![], vec![]);
        for (x, v) in rows {
            cum += v;
            xs.push(x);
            ys.push(Value::Float(v));
            cums.push(Value::Float(cum));
            pcts.push(if total != 0.0 {
                Value::Float(cum / total * 100.0)
            } else {
                Value::Na
            });
        }
        let n = xs.len();
        let mut out = DataFrame::new();
        out.add_column("x".into(), xs);
        out.add_column("y".into(), ys);
        out.add_column("cum".into(), cums.clone());
        out.add_column("cum_pct".into(), pcts);
        out.add_column("ymin".into(), vec![Value::Float(0.0); n]);
        // with a grand total beyond the rows, the scale reaches it (the
        // secondary axis's 100 %)
        let ymax = if total > sum {
            vec![Value::Float(total); n]
        } else {
            cums
        };
        out.add_column("ymax".into(), ymax);
        out
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    /// The ranking and running total span every category of the panel.
    fn panelwise(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "pareto"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_sorts_and_accumulates() {
        let mut d = DataFrame::new();
        d.add_column(
            "x".into(),
            ["a", "b", "c", "b"]
                .iter()
                .map(|s| Value::Str(s.to_string()))
                .collect(),
        );
        d.add_column(
            "y".into(),
            [10.0, 20.0, 50.0, 20.0]
                .iter()
                .map(|v| Value::Float(*v))
                .collect(),
        );
        let out = StatPareto::default().compute_group(&d, &ScaleSet::new());
        let keys: Vec<String> = out
            .column("x")
            .unwrap()
            .iter()
            .map(|v| v.key_str().into_owned())
            .collect();
        assert_eq!(keys, ["c", "b", "a"]);
        let f = |c: &str| -> Vec<f64> {
            out.column(c)
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect()
        };
        assert_eq!(f("y"), vec![50.0, 40.0, 10.0]);
        assert_eq!(f("cum"), vec![50.0, 90.0, 100.0]);
        assert_eq!(f("cum_pct"), vec![50.0, 90.0, 100.0]);
        // a top N of a longer list: the share of the grand total, and the
        // scale reaches it
        let out = StatPareto {
            total: Some(400.0),
            ..StatPareto::default()
        }
        .compute_group(&d, &ScaleSet::new());
        let f = |c: &str| -> Vec<f64> {
            out.column(c)
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect()
        };
        assert_eq!(f("cum_pct"), vec![12.5, 22.5, 25.0]);
        assert_eq!(f("ymax"), vec![400.0, 400.0, 400.0]);
    }
}
