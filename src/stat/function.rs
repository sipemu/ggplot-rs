use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Evaluate a function over the x range and produce (x, y) pairs.
pub struct StatFunction {
    pub fun: Box<dyn Fn(f64) -> f64 + Send + Sync>,
    pub n_points: usize,
}

impl StatFunction {
    pub fn new(fun: impl Fn(f64) -> f64 + Send + Sync + 'static) -> Self {
        StatFunction {
            fun: Box::new(fun),
            n_points: 101,
        }
    }

    pub fn with_n_points(mut self, n: usize) -> Self {
        self.n_points = n;
        self
    }
}

impl Stat for StatFunction {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        // Determine x range from data
        let x_col = match data.column("x") {
            Some(c) => c,
            None => return DataFrame::new(),
        };

        let values: Vec<f64> = x_col.iter().filter_map(|v| v.as_f64()).collect();
        if values.is_empty() {
            return DataFrame::new();
        }

        let x_min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let x_max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        if (x_max - x_min).abs() < f64::EPSILON {
            return DataFrame::new();
        }

        let step = (x_max - x_min) / (self.n_points - 1).max(1) as f64;

        let mut x_vals = Vec::with_capacity(self.n_points);
        let mut y_vals = Vec::with_capacity(self.n_points);

        for i in 0..self.n_points {
            let x = x_min + i as f64 * step;
            let y = (self.fun)(x);
            x_vals.push(Value::Float(x));
            y_vals.push(Value::Float(y));
        }

        let mut result = DataFrame::new();
        result.add_column("x".to_string(), x_vals);
        result.add_column("y".to_string(), y_vals);
        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X]
    }

    fn name(&self) -> &str {
        "function"
    }
}
