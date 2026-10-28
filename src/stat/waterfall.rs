//! `StatWaterfall`: running totals for a waterfall (bridge) chart.
//!
//! Walks the panel's rows in data order, accumulating `y`. Each step becomes a
//! bar floating from the previous running total (`ymin`/`ymax`); rows whose x
//! level is listed in `totals` instead draw from 0 to the running total (their
//! own `y` is ignored), and `final_total` appends one closing total bar.

use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Legend / `direction` label of a rising step.
pub const WATERFALL_INCREASE: &str = "Increase";
/// Legend / `direction` label of a falling step.
pub const WATERFALL_DECREASE: &str = "Decrease";
/// Legend / `direction` label of a total bar.
pub const WATERFALL_TOTAL: &str = "Total";

/// Cumulative-sum stat behind [`GeomWaterfall`](crate::geom::waterfall::GeomWaterfall).
///
/// Output columns: `x`, `y` (running total after the step), `ymin`/`ymax`
/// (bar extent), `start` (running total before the step), `delta` (the step's
/// own value; the total for total bars) and `direction`
/// ([`WATERFALL_INCREASE`] / [`WATERFALL_DECREASE`] / [`WATERFALL_TOTAL`]).
/// Every other input column is carried through row by row (`Na` on an
/// appended final total).
#[derive(Clone, Debug, Default)]
pub struct StatWaterfall {
    /// x levels drawn as total bars (0 → running total); their `y` is ignored.
    pub totals: Vec<String>,
    /// Append a closing total bar with this x label.
    pub final_total: Option<String>,
}

impl Stat for StatWaterfall {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let (Some(x_col), Some(y_col)) = (data.column("x"), data.column("y")) else {
            return DataFrame::new();
        };
        let n = data.nrows();
        let mut cum = 0.0;
        let mut out: Vec<(Value, [f64; 5], &'static str)> = Vec::with_capacity(n + 1);
        let mut kept = Vec::with_capacity(n);
        for i in 0..n {
            let is_total = self.totals.iter().any(|t| *t == x_col[i].key_str());
            if is_total {
                out.push((x_col[i].clone(), [0.0, cum, 0.0, cum, cum], WATERFALL_TOTAL));
            } else {
                let Some(d) = y_col[i].as_f64().filter(|d| d.is_finite()) else {
                    continue;
                };
                let start = cum;
                cum += d;
                let dir = if d >= 0.0 {
                    WATERFALL_INCREASE
                } else {
                    WATERFALL_DECREASE
                };
                out.push((
                    x_col[i].clone(),
                    [start, cum, start.min(cum), start.max(cum), d],
                    dir,
                ));
            }
            kept.push(Some(i));
        }
        if let Some(label) = &self.final_total {
            out.push((
                Value::Str(label.clone()),
                [0.0, cum, 0.0, cum, cum],
                WATERFALL_TOTAL,
            ));
            kept.push(None);
        }

        let mut result = DataFrame::new();
        let num = |k: usize| out.iter().map(|r| Value::Float(r.1[k])).collect::<Vec<_>>();
        result.add_column("x".into(), out.iter().map(|r| r.0.clone()).collect());
        result.add_column("y".into(), num(1));
        result.add_column("ymin".into(), num(2));
        result.add_column("ymax".into(), num(3));
        result.add_column("start".into(), num(0));
        result.add_column("delta".into(), num(4));
        result.add_column(
            "direction".into(),
            out.iter().map(|r| Value::Str(r.2.to_string())).collect(),
        );
        for name in data.column_names() {
            if result.has_column(name) {
                continue;
            }
            let col = data.column(name).unwrap_or_default();
            let vals = kept
                .iter()
                .map(|k| k.map(|i| col[i].clone()).unwrap_or(Value::Na))
                .collect();
            result.add_column(name.to_string(), vals);
        }
        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    /// The running total spans every step of the panel, not one aesthetic
    /// group (a discrete x would otherwise make each step its own group).
    fn panelwise(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "waterfall"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn df(xs: &[&str], ys: &[f64]) -> DataFrame {
        let mut d = DataFrame::new();
        d.add_column(
            "x".into(),
            xs.iter().map(|s| Value::Str(s.to_string())).collect(),
        );
        d.add_column("y".into(), ys.iter().map(|v| Value::Float(*v)).collect());
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
    fn steps_float_from_the_running_total() {
        let stat = StatWaterfall {
            totals: vec!["Sub".into()],
            final_total: Some("End".into()),
        };
        let out = stat.compute_group(
            &df(&["A", "B", "Sub", "C"], &[10.0, -4.0, 0.0, 3.0]),
            &ScaleSet::new(),
        );
        assert_eq!(out.nrows(), 5);
        assert_eq!(col(&out, "ymin"), vec![0.0, 6.0, 0.0, 6.0, 0.0]);
        assert_eq!(col(&out, "ymax"), vec![10.0, 10.0, 6.0, 9.0, 9.0]);
        assert_eq!(col(&out, "y"), vec![10.0, 6.0, 6.0, 9.0, 9.0]);
        assert_eq!(col(&out, "delta"), vec![10.0, -4.0, 6.0, 3.0, 9.0]);
        let dirs: Vec<String> = out
            .column("direction")
            .unwrap()
            .iter()
            .map(|v| v.key_str().into_owned())
            .collect();
        assert_eq!(dirs, ["Increase", "Decrease", "Total", "Increase", "Total"]);
        assert_eq!(out.column("x").unwrap()[4], Value::Str("End".into()));
    }
}
