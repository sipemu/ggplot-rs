use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Gaussian kernel density estimation with Silverman bandwidth.
pub struct StatDensity {
    pub n_points: usize,
}

impl Default for StatDensity {
    fn default() -> Self {
        StatDensity { n_points: 512 }
    }
}

impl Stat for StatDensity {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let x_col = match data.column("x") {
            Some(c) => c,
            None => return DataFrame::new(),
        };

        let values: Vec<f64> = x_col.iter().filter_map(|v| v.as_f64()).collect();
        if values.len() < 2 {
            return DataFrame::new();
        }

        let n = values.len() as f64;
        // R's bw.nrd0 (positive even for zero-spread data).
        let bandwidth = super::bw_nrd0(&values);

        let x_min = values.iter().cloned().fold(f64::INFINITY, f64::min) - 3.0 * bandwidth;
        let x_max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max) + 3.0 * bandwidth;
        let step = (x_max - x_min) / (self.n_points - 1) as f64;

        let mut x_vals = Vec::with_capacity(self.n_points);
        let mut y_vals = Vec::with_capacity(self.n_points);

        for i in 0..self.n_points {
            let x = x_min + i as f64 * step;
            let density: f64 = values
                .iter()
                .map(|xi| gaussian_kernel((x - xi) / bandwidth))
                .sum::<f64>()
                / (n * bandwidth);

            x_vals.push(Value::Float(x));
            y_vals.push(Value::Float(density));
        }

        let mut result = DataFrame::new();
        result.add_column("x".to_string(), x_vals);
        result.add_column("y".to_string(), y_vals);

        // Carry over grouping columns
        for col_name in &["color", "fill", "group"] {
            if let Some(col) = data.column(col_name) {
                if let Some(first) = col.first() {
                    result.add_column(col_name.to_string(), vec![first.clone(); self.n_points]);
                }
            }
        }

        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X]
    }

    fn name(&self) -> &str {
        "density"
    }
}

fn gaussian_kernel(x: f64) -> f64 {
    (-(x * x) / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt()
}
