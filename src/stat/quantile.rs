use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

use anofox_regression::{FittedRegressor, QuantileRegressor, Regressor};
use faer::{Col, Mat};

/// Quantile-regression line for a single quantile `tau` (R's `stat_quantile`),
/// backed by anofox-regression's `QuantileRegressor`. One fitted line per group.
pub struct StatQuantile {
    /// Quantile in (0, 1). Default 0.5 (median).
    pub tau: f64,
    /// Number of points along the fitted line. Default 80.
    pub n_points: usize,
}

impl Default for StatQuantile {
    fn default() -> Self {
        StatQuantile {
            tau: 0.5,
            n_points: 80,
        }
    }
}

impl StatQuantile {
    pub fn new(tau: f64) -> Self {
        StatQuantile {
            tau,
            ..Default::default()
        }
    }
}

impl Stat for StatQuantile {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let (x_col, y_col) = match (data.column("x"), data.column("y")) {
            (Some(x), Some(y)) => (x, y),
            _ => return DataFrame::new(),
        };
        let pts: Vec<(f64, f64)> = x_col
            .iter()
            .zip(y_col.iter())
            .filter_map(|(a, b)| Some((a.as_f64()?, b.as_f64()?)))
            .collect();
        if pts.len() < 2 {
            return DataFrame::new();
        }

        let n = pts.len();
        let x = Mat::from_fn(n, 1, |i, _| pts[i].0);
        let y = Col::from_fn(n, |i| pts[i].1);

        let model = QuantileRegressor::builder().tau(self.tau).build();
        let fitted = match model.fit(&x, &y) {
            Ok(f) => f,
            Err(_) => return DataFrame::new(),
        };
        let xmin = pts.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let xmax = pts.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
        let steps = self.n_points.max(2);
        let grid = Mat::from_fn(steps, 1, |k, _| {
            xmin + (xmax - xmin) * k as f64 / (steps - 1) as f64
        });
        let preds = fitted.predict(&grid);

        let mut x_vals = Vec::with_capacity(steps);
        let mut y_vals = Vec::with_capacity(steps);
        for k in 0..steps {
            x_vals.push(Value::Float(grid[(k, 0)]));
            y_vals.push(Value::Float(preds[k]));
        }

        let nrows = x_vals.len();
        let mut result = DataFrame::new();
        result.add_column("x".to_string(), x_vals);
        result.add_column("y".to_string(), y_vals);
        for col_name in &["color", "fill", "group"] {
            if let Some(col) = data.column(col_name) {
                if let Some(first) = col.first() {
                    result.add_column(col_name.to_string(), vec![first.clone(); nrows]);
                }
            }
        }
        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn name(&self) -> &str {
        "quantile"
    }
}
