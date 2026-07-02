use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Smoothing method selection.
#[derive(Clone, Debug, Default)]
pub enum SmoothMethod {
    /// Linear regression (y = mx + b).
    #[default]
    Lm,
    /// LOESS with configurable span.
    Loess { span: f64 },
    /// Generalized linear model via anofox-regression (Gaussian or Poisson).
    #[cfg(feature = "regression")]
    Glm { family: SmoothFamily },
    /// Robust linear regression (Huber M-estimator) via anofox-regression.
    #[cfg(feature = "regression")]
    Rlm,
}

/// GLM family for regression-backed smoothing (`SmoothMethod::Glm`).
#[cfg(feature = "regression")]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum SmoothFamily {
    /// Ordinary least squares (identity link).
    #[default]
    Gaussian,
    /// Poisson regression (log link) for count responses.
    Poisson,
}

/// Smoothing statistic — supports both linear regression and LOESS.
pub struct StatSmooth {
    /// Number of points to generate for the fitted line.
    pub n_points: usize,
    /// Whether to compute confidence interval.
    pub se: bool,
    /// Smoothing method.
    pub method: SmoothMethod,
}

impl Default for StatSmooth {
    fn default() -> Self {
        StatSmooth {
            n_points: 80,
            se: true,
            method: SmoothMethod::Lm,
        }
    }
}

impl Stat for StatSmooth {
    fn compute_group(&self, data: &DataFrame, scales: &ScaleSet) -> DataFrame {
        match &self.method {
            SmoothMethod::Lm => self.compute_lm(data),
            SmoothMethod::Loess { span } => {
                let loess = super::loess::StatLoess {
                    span: *span,
                    n_points: self.n_points,
                    se: self.se,
                };
                loess.compute_group(data, scales)
            }
            #[cfg(feature = "regression")]
            SmoothMethod::Glm { family } => self.compute_glm(data, Some(*family)),
            #[cfg(feature = "regression")]
            SmoothMethod::Rlm => self.compute_glm(data, None),
        }
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn name(&self) -> &str {
        "smooth"
    }
}

impl StatSmooth {
    fn compute_lm(&self, data: &DataFrame) -> DataFrame {
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

        if pairs.len() < 2 {
            return DataFrame::new();
        }

        let n = pairs.len() as f64;
        let sum_x: f64 = pairs.iter().map(|(x, _)| x).sum();
        let sum_y: f64 = pairs.iter().map(|(_, y)| y).sum();
        let sum_xy: f64 = pairs.iter().map(|(x, y)| x * y).sum();
        let sum_xx: f64 = pairs.iter().map(|(x, _)| x * x).sum();

        let mean_x = sum_x / n;
        let mean_y = sum_y / n;

        let denom = sum_xx - sum_x * sum_x / n;
        let (slope, intercept) = if denom.abs() < f64::EPSILON {
            (0.0, mean_y)
        } else {
            let m = (sum_xy - sum_x * sum_y / n) / denom;
            let b = mean_y - m * mean_x;
            (m, b)
        };

        // Generate fitted values across x range
        let x_min = pairs.iter().map(|(x, _)| *x).fold(f64::INFINITY, f64::min);
        let x_max = pairs
            .iter()
            .map(|(x, _)| *x)
            .fold(f64::NEG_INFINITY, f64::max);

        let step = (x_max - x_min) / (self.n_points - 1).max(1) as f64;

        // Compute standard error of prediction if requested
        let se_values = if self.se && pairs.len() > 2 {
            let residuals: Vec<f64> = pairs
                .iter()
                .map(|(x, y)| y - (slope * x + intercept))
                .collect();
            let sse: f64 = residuals.iter().map(|r| r * r).sum();
            let mse = sse / (n - 2.0);
            Some((mse, sum_xx, mean_x, n))
        } else {
            None
        };

        let mut x_vals = Vec::with_capacity(self.n_points);
        let mut y_vals = Vec::with_capacity(self.n_points);
        let mut ymin_vals = Vec::with_capacity(self.n_points);
        let mut ymax_vals = Vec::with_capacity(self.n_points);

        for i in 0..self.n_points {
            let x = x_min + i as f64 * step;
            let y = slope * x + intercept;
            x_vals.push(Value::Float(x));
            y_vals.push(Value::Float(y));

            if let Some((mse, sum_xx, mean_x, n)) = se_values {
                let se_pred = (mse
                    * (1.0 / n + (x - mean_x).powi(2) / (sum_xx - n * mean_x * mean_x)))
                    .sqrt();
                // ~95% CI: t ≈ 1.96 for large n
                let t_val = 1.96;
                ymin_vals.push(Value::Float(y - t_val * se_pred));
                ymax_vals.push(Value::Float(y + t_val * se_pred));
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

    /// GLM / robust-linear smoothing backed by anofox-regression. `family = None`
    /// selects the robust (Huber) fit; `Some(..)` selects a GLM family. A
    /// confidence-interval ribbon (ymin/ymax) is emitted when `self.se` is set.
    #[cfg(feature = "regression")]
    fn compute_glm(&self, data: &DataFrame, family: Option<SmoothFamily>) -> DataFrame {
        use anofox_regression::solvers::{
            FittedRegressor, HuberRegressor, OlsRegressor, PoissonRegressor, Regressor,
        };
        use anofox_regression::{IntervalType, PoissonFamily, RegressionOptions};
        use faer::{Col, Mat};

        let (x_col, y_col) = match (data.column("x"), data.column("y")) {
            (Some(x), Some(y)) => (x, y),
            _ => return DataFrame::new(),
        };
        let pairs: Vec<(f64, f64)> = x_col
            .iter()
            .zip(y_col.iter())
            .filter_map(|(x, y)| Some((x.as_f64()?, y.as_f64()?)))
            .collect();
        if pairs.len() < 2 {
            return DataFrame::new();
        }

        let n = pairs.len();
        let x = Mat::from_fn(n, 1, |i, _| pairs[i].0);
        let y = Col::from_fn(n, |i| pairs[i].1);
        let x_min = pairs.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let x_max = pairs.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
        let steps = self.n_points.max(2);
        let grid = Mat::from_fn(steps, 1, |k, _| {
            x_min + (x_max - x_min) * k as f64 / (steps - 1) as f64
        });
        let interval = if self.se {
            Some(IntervalType::Confidence)
        } else {
            None
        };

        // Fit the requested model and predict (with interval) over the grid.
        let pred = match family {
            None => match HuberRegressor::new().fit(&x, &y) {
                Ok(f) => f.predict_with_interval(&grid, interval, 0.95),
                Err(_) => return DataFrame::new(),
            },
            Some(SmoothFamily::Gaussian) => {
                match OlsRegressor::new(RegressionOptions::default()).fit(&x, &y) {
                    Ok(f) => f.predict_with_interval(&grid, interval, 0.95),
                    Err(_) => return DataFrame::new(),
                }
            }
            Some(SmoothFamily::Poisson) => {
                let reg =
                    PoissonRegressor::new(RegressionOptions::default(), PoissonFamily::default());
                match reg.fit(&x, &y) {
                    Ok(f) => f.predict_with_interval(&grid, interval, 0.95),
                    Err(_) => return DataFrame::new(),
                }
            }
        };

        let mut x_vals = Vec::with_capacity(steps);
        let mut y_vals = Vec::with_capacity(steps);
        let mut ymin_vals = Vec::with_capacity(steps);
        let mut ymax_vals = Vec::with_capacity(steps);
        for k in 0..steps {
            x_vals.push(Value::Float(grid[(k, 0)]));
            y_vals.push(Value::Float(pred.fit[k]));
            if self.se {
                ymin_vals.push(Value::Float(pred.lower[k]));
                ymax_vals.push(Value::Float(pred.upper[k]));
            }
        }

        let mut result = DataFrame::new();
        result.add_column("x".to_string(), x_vals);
        result.add_column("y".to_string(), y_vals);
        if self.se {
            result.add_column("ymin".to_string(), ymin_vals);
            result.add_column("ymax".to_string(), ymax_vals);
        }
        for col_name in &["color", "fill", "group"] {
            if let Some(col) = data.column(col_name) {
                if let Some(first) = col.first() {
                    result.add_column(col_name.to_string(), vec![first.clone(); steps]);
                }
            }
        }
        result
    }
}
