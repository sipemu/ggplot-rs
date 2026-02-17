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
        let mean = values.iter().sum::<f64>() / n;
        let var = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
        let sd = var.sqrt();

        // Silverman's rule of thumb
        let bandwidth = 0.9 * sd.min(iqr(&values) / 1.34) * n.powf(-0.2);
        let bandwidth = if bandwidth > 0.0 { bandwidth } else { sd * 0.5 };

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

fn iqr(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    quantile_type7(&sorted, 0.75) - quantile_type7(&sorted, 0.25)
}

/// R-compatible type-7 quantile interpolation (R's default `quantile()` method).
fn quantile_type7(sorted: &[f64], p: f64) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return 0.0;
    }
    if n == 1 {
        return sorted[0];
    }
    let h = (n - 1) as f64 * p;
    let lo = h.floor() as usize;
    let hi = (lo + 1).min(n - 1);
    let frac = h - lo as f64;
    sorted[lo] + frac * (sorted[hi] - sorted[lo])
}
