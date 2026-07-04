use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// LOESS (locally estimated scatterplot smoothing) via local weighted polynomial regression.
pub struct StatLoess {
    /// Span parameter controlling smoothness (0, 1]. Smaller = more flexible.
    pub span: f64,
    /// Number of points to generate for the fitted curve.
    pub n_points: usize,
    /// Whether to compute confidence interval.
    pub se: bool,
}

impl Default for StatLoess {
    fn default() -> Self {
        StatLoess {
            span: 0.75,
            n_points: 80,
            se: true,
        }
    }
}

impl Stat for StatLoess {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let x_col = match data.column("x") {
            Some(c) => c,
            None => return DataFrame::new(),
        };
        let y_col = match data.column("y") {
            Some(c) => c,
            None => return DataFrame::new(),
        };

        let pairs: Vec<(f64, f64)> = x_col
            .iter()
            .zip(y_col.iter())
            .filter_map(|(x, y)| Some((x.as_f64()?, y.as_f64()?)))
            .collect();

        if pairs.len() < 3 {
            return DataFrame::new();
        }

        let n = pairs.len();
        let x_min = pairs.iter().map(|(x, _)| *x).fold(f64::INFINITY, f64::min);
        let x_max = pairs
            .iter()
            .map(|(x, _)| *x)
            .fold(f64::NEG_INFINITY, f64::max);
        let step = (x_max - x_min) / (self.n_points - 1).max(1) as f64;

        // Number of neighbors to use
        let k = ((self.span * n as f64).ceil() as usize).max(3).min(n);

        let mut x_vals = Vec::with_capacity(self.n_points);
        let mut y_vals = Vec::with_capacity(self.n_points);
        let mut ymin_vals = Vec::with_capacity(self.n_points);
        let mut ymax_vals = Vec::with_capacity(self.n_points);

        // Compute residual variance for SE estimation
        let residual_var = if self.se {
            let mut sse = 0.0;
            for &(xi, yi) in &pairs {
                let y_hat = local_regression(&pairs, xi, k);
                sse += (yi - y_hat).powi(2);
            }
            Some(sse / (n as f64 - 2.0).max(1.0))
        } else {
            None
        };

        for i in 0..self.n_points {
            let x = x_min + i as f64 * step;
            let y = local_regression(&pairs, x, k);
            x_vals.push(Value::Float(x));
            y_vals.push(Value::Float(y));

            if let Some(var) = residual_var {
                // Approximate SE using residual variance and effective degrees of freedom
                let se = var.sqrt() * (1.0 / k as f64 + 1.0 / n as f64).sqrt() * 1.5;
                let t_val = 1.96;
                ymin_vals.push(Value::Float(y - t_val * se));
                ymax_vals.push(Value::Float(y + t_val * se));
            }
        }

        let mut result = DataFrame::new();
        result.add_column("x".to_string(), x_vals);
        result.add_column("y".to_string(), y_vals);
        if !ymin_vals.is_empty() {
            result.add_column("ymin".to_string(), ymin_vals);
            result.add_column("ymax".to_string(), ymax_vals);
        }
        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn name(&self) -> &str {
        "loess"
    }
}

/// Fit a tricube-weighted local quadratic (degree 2, like R's loess) over the
/// `k` nearest neighbors and return the prediction at `x0`.
fn local_regression(pairs: &[(f64, f64)], x0: f64, k: usize) -> f64 {
    // Sort by distance to x0 and take k nearest
    let mut dists: Vec<(usize, f64)> = pairs
        .iter()
        .enumerate()
        .map(|(i, (x, _))| (i, (x - x0).abs()))
        .collect();
    dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let max_dist = dists[k - 1].1;
    let max_dist = if max_dist < f64::EPSILON {
        1.0
    } else {
        max_dist
    };

    // Tricube weight function
    let weights: Vec<(f64, f64, f64)> = dists[..k]
        .iter()
        .map(|(i, d)| {
            let u = d / max_dist;
            let u = u.min(1.0);
            let w = (1.0 - u * u * u).powi(3);
            (pairs[*i].0, pairs[*i].1, w)
        })
        .collect();

    let sum_w: f64 = weights.iter().map(|(_, _, w)| w).sum();
    if sum_w < f64::EPSILON {
        return pairs.iter().map(|(_, y)| y).sum::<f64>() / pairs.len() as f64;
    }
    let mean_y = weights.iter().map(|(_, y, w)| w * y).sum::<f64>() / sum_w;

    // Weighted local quadratic regression (R's loess default degree = 2),
    // centered at x0 so the prediction is just the intercept. Solve the 3×3
    // normal equations for [a, b, c] with t = x - x0; the fit at t=0 is `a`.
    let (mut s1, mut s2, mut s3, mut s4) = (0.0, 0.0, 0.0, 0.0);
    let (mut ty0, mut ty1, mut ty2) = (0.0, 0.0, 0.0);
    for &(x, y, w) in &weights {
        let t = x - x0;
        let (t2, t3, t4) = (t * t, t * t * t, t * t * t * t);
        s1 += w * t;
        s2 += w * t2;
        s3 += w * t3;
        s4 += w * t4;
        ty0 += w * y;
        ty1 += w * t * y;
        ty2 += w * t2 * y;
    }
    // Matrix M = [[s0,s1,s2],[s1,s2,s3],[s2,s3,s4]], RHS = [ty0,ty1,ty2].
    let s0 = sum_w;
    let det = s0 * (s2 * s4 - s3 * s3) - s1 * (s1 * s4 - s3 * s2) + s2 * (s1 * s3 - s2 * s2);
    if det.abs() < 1e-12 {
        // Singular (e.g. too few distinct x): fall back to weighted linear.
        let denom = s0 * s2 - s1 * s1;
        if denom.abs() < 1e-12 {
            return mean_y;
        }
        let b = (s0 * ty1 - s1 * ty0) / denom;
        let a = (ty0 - b * s1) / s0;
        return a;
    }
    // Cramer's rule for a (column 0 replaced by RHS).
    let det_a = ty0 * (s2 * s4 - s3 * s3) - s1 * (ty1 * s4 - s3 * ty2) + s2 * (ty1 * s3 - s2 * ty2);
    det_a / det
}
