use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Confidence ellipse for a 2-D point cloud (analogous to R's `stat_ellipse`).
///
/// Assumes a bivariate normal distribution: the ellipse is the covariance
/// eigen-decomposition scaled by the chi-square quantile for `level` (df = 2).
/// Emits `segments + 1` boundary points forming a closed path per group.
pub struct StatEllipse {
    /// Confidence level in (0, 1). Default 0.95.
    pub level: f64,
    /// Number of segments used to draw the ellipse. Default 51.
    pub segments: usize,
}

impl Default for StatEllipse {
    fn default() -> Self {
        StatEllipse {
            level: 0.95,
            segments: 51,
        }
    }
}

impl StatEllipse {
    pub fn new(level: f64) -> Self {
        StatEllipse {
            level,
            ..Default::default()
        }
    }
}

impl Stat for StatEllipse {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let (xs, ys) = match (data.column("x"), data.column("y")) {
            (Some(x), Some(y)) => (x, y),
            _ => return DataFrame::new(),
        };
        let pts: Vec<(f64, f64)> = xs
            .iter()
            .zip(ys.iter())
            .filter_map(|(a, b)| Some((a.as_f64()?, b.as_f64()?)))
            .collect();
        if pts.len() < 3 {
            return DataFrame::new();
        }

        let n = pts.len() as f64;
        let mx = pts.iter().map(|p| p.0).sum::<f64>() / n;
        let my = pts.iter().map(|p| p.1).sum::<f64>() / n;

        // Sample covariance (n - 1 denominator).
        let mut sxx = 0.0;
        let mut syy = 0.0;
        let mut sxy = 0.0;
        for &(x, y) in &pts {
            sxx += (x - mx) * (x - mx);
            syy += (y - my) * (y - my);
            sxy += (x - mx) * (y - my);
        }
        let d = n - 1.0;
        let (sxx, syy, sxy) = (sxx / d, syy / d, sxy / d);

        // Eigen-decomposition of the symmetric 2x2 [[sxx, sxy], [sxy, syy]].
        let trace = sxx + syy;
        let det = sxx * syy - sxy * sxy;
        let disc = ((trace * 0.5).powi(2) - det).max(0.0).sqrt();
        let l1 = (trace * 0.5 + disc).max(0.0);
        let l2 = (trace * 0.5 - disc).max(0.0);
        let (v1x, v1y) = if sxy.abs() > 1e-12 {
            let vx = l1 - syy;
            let vy = sxy;
            let norm = (vx * vx + vy * vy).sqrt();
            (vx / norm, vy / norm)
        } else if sxx >= syy {
            (1.0, 0.0)
        } else {
            (0.0, 1.0)
        };
        // Second axis is perpendicular to the first.
        let (v2x, v2y) = (-v1y, v1x);

        // Chi-square quantile with 2 dof has the closed form -2 ln(1 - level).
        let radius = (-2.0 * (1.0 - self.level).ln()).sqrt();
        let a = radius * l1.sqrt();
        let b = radius * l2.sqrt();

        let steps = self.segments.max(3);
        let mut x_vals = Vec::with_capacity(steps + 1);
        let mut y_vals = Vec::with_capacity(steps + 1);
        for i in 0..=steps {
            let theta = 2.0 * std::f64::consts::PI * (i as f64) / (steps as f64);
            let (c, s) = (theta.cos(), theta.sin());
            let px = mx + a * c * v1x + b * s * v2x;
            let py = my + a * c * v1y + b * s * v2y;
            x_vals.push(Value::Float(px));
            y_vals.push(Value::Float(py));
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
        "ellipse"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(pts: &[(f64, f64)]) -> DataFrame {
        let mut df = DataFrame::new();
        df.add_column("x".into(), pts.iter().map(|p| Value::Float(p.0)).collect());
        df.add_column("y".into(), pts.iter().map(|p| Value::Float(p.1)).collect());
        df
    }

    #[test]
    fn ellipse_of_circular_cloud_is_centered() {
        // A symmetric ring of points → ellipse centred at the mean.
        let pts: Vec<(f64, f64)> = (0..40)
            .map(|i| {
                let t = 2.0 * std::f64::consts::PI * i as f64 / 40.0;
                (5.0 + t.cos(), 3.0 + t.sin())
            })
            .collect();
        let out = StatEllipse::default().compute_group(&frame(&pts), &ScaleSet::new());
        assert_eq!(out.nrows(), StatEllipse::default().segments + 1);
        let xs: Vec<f64> = out
            .column("x")
            .unwrap()
            .iter()
            .filter_map(|v| v.as_f64())
            .collect();
        let ys: Vec<f64> = out
            .column("y")
            .unwrap()
            .iter()
            .filter_map(|v| v.as_f64())
            .collect();
        let cx = xs.iter().sum::<f64>() / xs.len() as f64;
        let cy = ys.iter().sum::<f64>() / ys.len() as f64;
        assert!((cx - 5.0).abs() < 0.2, "center x {cx}");
        assert!((cy - 3.0).abs() < 0.2, "center y {cy}");
        // Closed path: first point equals last.
        assert!((xs[0] - xs[xs.len() - 1]).abs() < 1e-9);
    }

    #[test]
    fn too_few_points_returns_empty() {
        let out = StatEllipse::default()
            .compute_group(&frame(&[(0.0, 0.0), (1.0, 1.0)]), &ScaleSet::new());
        assert_eq!(out.nrows(), 0);
    }

    #[test]
    fn higher_level_makes_larger_ellipse() {
        let pts: Vec<(f64, f64)> = (0..30)
            .map(|i| (i as f64, (i as f64 * 0.7).sin() * 3.0))
            .collect();
        let small = StatEllipse::new(0.5).compute_group(&frame(&pts), &ScaleSet::new());
        let big = StatEllipse::new(0.99).compute_group(&frame(&pts), &ScaleSet::new());
        let span = |df: &DataFrame| {
            let xs: Vec<f64> = df
                .column("x")
                .unwrap()
                .iter()
                .filter_map(|v| v.as_f64())
                .collect();
            xs.iter().cloned().fold(f64::MIN, f64::max)
                - xs.iter().cloned().fold(f64::MAX, f64::min)
        };
        assert!(span(&big) > span(&small));
    }
}
