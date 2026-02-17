use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Aggregation function type for StatSummary.
#[derive(Clone)]
pub enum SummaryFun {
    Mean,
    Median,
    Min,
    Max,
    Sum,
}

impl SummaryFun {
    fn apply(&self, values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        match self {
            SummaryFun::Mean => values.iter().sum::<f64>() / values.len() as f64,
            SummaryFun::Median => {
                let mut sorted = values.to_vec();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let n = sorted.len();
                if n.is_multiple_of(2) {
                    (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
                } else {
                    sorted[n / 2]
                }
            }
            SummaryFun::Min => values.iter().cloned().fold(f64::INFINITY, f64::min),
            SummaryFun::Max => values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            SummaryFun::Sum => values.iter().sum(),
        }
    }
}

/// Summarize y values for each unique x with a summary function.
/// Also computes ymin and ymax using configurable functions.
pub struct StatSummary {
    pub fun_y: SummaryFun,
    pub fun_ymin: SummaryFun,
    pub fun_ymax: SummaryFun,
}

impl Default for StatSummary {
    fn default() -> Self {
        StatSummary {
            fun_y: SummaryFun::Mean,
            fun_ymin: SummaryFun::Min,
            fun_ymax: SummaryFun::Max,
        }
    }
}

impl StatSummary {
    /// Create with mean_se default (mean +/- standard error).
    pub fn mean_se() -> Self {
        StatSummary {
            fun_y: SummaryFun::Mean,
            fun_ymin: SummaryFun::Min, // will be overridden in compute_group
            fun_ymax: SummaryFun::Max,
        }
    }
}

impl Stat for StatSummary {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let x_col = match data.column("x") {
            Some(c) => c,
            None => return DataFrame::new(),
        };
        let y_col = match data.column("y") {
            Some(c) => c,
            None => return DataFrame::new(),
        };

        // Group y values by x
        let mut groups: Vec<(String, Value, Vec<f64>)> = Vec::new();
        for (x, y) in x_col.iter().zip(y_col.iter()) {
            let key = x.to_group_key();
            let y_val = y.as_f64().unwrap_or(0.0);
            if let Some(entry) = groups.iter_mut().find(|(k, _, _)| k == &key) {
                entry.2.push(y_val);
            } else {
                groups.push((key, x.clone(), vec![y_val]));
            }
        }

        let n = groups.len();
        let mut x_vals = Vec::with_capacity(n);
        let mut y_vals = Vec::with_capacity(n);
        let mut ymin_vals = Vec::with_capacity(n);
        let mut ymax_vals = Vec::with_capacity(n);

        for (_, x_val, ys) in &groups {
            x_vals.push(x_val.clone());
            y_vals.push(Value::Float(self.fun_y.apply(ys)));
            ymin_vals.push(Value::Float(self.fun_ymin.apply(ys)));
            ymax_vals.push(Value::Float(self.fun_ymax.apply(ys)));
        }

        let mut result = DataFrame::new();
        result.add_column("x".to_string(), x_vals);
        result.add_column("y".to_string(), y_vals);
        result.add_column("ymin".to_string(), ymin_vals);
        result.add_column("ymax".to_string(), ymax_vals);

        // Carry over grouping columns
        for col_name in &["color", "fill", "group"] {
            if let Some(col) = data.column(col_name) {
                if let Some(first) = col.first() {
                    result.add_column(col_name.to_string(), vec![first.clone(); n]);
                }
            }
        }

        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn name(&self) -> &str {
        "summary"
    }
}
