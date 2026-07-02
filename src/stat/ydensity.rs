use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Kernel density estimation on Y per group (for violin plots).
/// Outputs: x (group value), y (eval points), violinwidth (density normalized to
/// [0, 1]). The geom mirrors `violinwidth` around the group's x slot.
pub struct StatYDensity {
    pub n_points: usize,
}

impl Default for StatYDensity {
    fn default() -> Self {
        StatYDensity { n_points: 128 }
    }
}

impl Stat for StatYDensity {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let x_col = data.column("x");
        let y_col = match data.column("y") {
            Some(c) => c,
            None => return DataFrame::new(),
        };

        let values: Vec<f64> = y_col.iter().filter_map(|v| v.as_f64()).collect();
        if values.len() < 2 {
            return DataFrame::new();
        }

        // Keep the group's x *value* as-is (e.g. the discrete label "A"). The geom
        // maps it through the X scale, exactly like boxplot — converting to f64 here
        // would collapse every discrete group to 0.0.
        let group_x = x_col
            .and_then(|c| c.first())
            .cloned()
            .unwrap_or(Value::Float(0.0));

        let n = values.len() as f64;
        let mean = values.iter().sum::<f64>() / n;
        let var = values.iter().map(|y| (y - mean).powi(2)).sum::<f64>() / (n - 1.0);
        let sd = var.sqrt();

        // Silverman's rule of thumb
        let iqr_val = iqr(&values);
        let bandwidth = 0.9 * sd.min(iqr_val / 1.34) * n.powf(-0.2);
        let bandwidth = if bandwidth > 0.0 { bandwidth } else { sd * 0.5 };

        let y_min = values.iter().cloned().fold(f64::INFINITY, f64::min) - 3.0 * bandwidth;
        let y_max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max) + 3.0 * bandwidth;
        let step = (y_max - y_min) / (self.n_points - 1) as f64;

        let mut x_vals = Vec::with_capacity(self.n_points);
        let mut y_vals = Vec::with_capacity(self.n_points);

        // Compute density at each evaluation point
        let mut densities = Vec::with_capacity(self.n_points);
        let mut max_density: f64 = 0.0;
        for i in 0..self.n_points {
            let y = y_min + i as f64 * step;
            let density: f64 = values
                .iter()
                .map(|yi| gaussian_kernel((y - yi) / bandwidth))
                .sum::<f64>()
                / (n * bandwidth);
            densities.push((y, density));
            if density > max_density {
                max_density = density;
            }
        }

        // Normalize density to [0, 1] (peak = 1). The geom scales this by the
        // per-group slot half-width, so the widest point fills the group's slot.
        let scale = if max_density > 0.0 {
            1.0 / max_density
        } else {
            1.0
        };

        let mut width_vals = Vec::with_capacity(self.n_points);
        for (y, density) in &densities {
            x_vals.push(group_x.clone());
            y_vals.push(Value::Float(*y));
            width_vals.push(Value::Float(density * scale));
        }

        let mut result = DataFrame::new();
        result.add_column("x".to_string(), x_vals);
        result.add_column("y".to_string(), y_vals);
        result.add_column("violinwidth".to_string(), width_vals);

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
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn name(&self) -> &str {
        "ydensity"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ydensity_basic() {
        let mut data = DataFrame::new();
        data.add_column("x".to_string(), vec![Value::Float(1.0); 50]);
        let y_vals: Vec<Value> = (0..50).map(|i| Value::Float(i as f64)).collect();
        data.add_column("y".to_string(), y_vals);

        let stat = StatYDensity::default();
        let scales = ScaleSet::new();
        let result = stat.compute_group(&data, &scales);

        assert!(result.nrows() > 0);
        assert!(result.column("x").is_some());
        assert!(result.column("y").is_some());
        assert!(result.column("violinwidth").is_some());
        // Normalized width peaks at 1.0.
        let max_w = result
            .column("violinwidth")
            .unwrap()
            .iter()
            .filter_map(|v| v.as_f64())
            .fold(0.0_f64, f64::max);
        assert!(
            (max_w - 1.0).abs() < 1e-9,
            "peak width should be 1.0, got {max_w}"
        );
    }
}
