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
    /// Penalized B-spline (P-spline) GAM smoother via anofox-regression —
    /// ggplot2's `method = "gam"`. λ is chosen by GCV.
    #[cfg(feature = "regression")]
    Gam,
}

/// GLM family for regression-backed smoothing (`SmoothMethod::Glm`), R's
/// `glm(family = …)`.
///
/// For every non-Gaussian family the confidence band is computed as ggplot2's
/// `predictdf.glm` does: the linear predictor and its standard error come from
/// `predict(type = "link", se.fit = TRUE)`, the interval
/// `η ± qnorm(0.975)·se(η)` is formed on the link scale and both ends are
/// mapped through the inverse link — so the band respects the response range
/// (probabilities stay in `[0, 1]`, means stay positive). Dispersion follows
/// R: fixed at 1 for binomial / Poisson / negative binomial, Pearson-estimated
/// for Gamma.
///
/// `Gaussian` keeps the ordinary-least-squares fit with its `t`-based interval
/// (identical to `method = "lm"`).
#[cfg(feature = "regression")]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum SmoothFamily {
    /// Ordinary least squares (identity link).
    #[default]
    Gaussian,
    /// Poisson regression (log link) for count responses.
    Poisson,
    /// Binomial regression for 0/1 (or proportion) responses —
    /// `binomial(link)`, R's default link is logit.
    Binomial(SmoothBinomialLink),
    /// Gamma regression for positive continuous responses — `Gamma(link)`;
    /// R's default link is the inverse.
    Gamma(SmoothGammaLink),
    /// Negative-binomial regression (log link, θ estimated by maximum
    /// likelihood) for over-dispersed counts — R's `MASS::glm.nb`.
    NegativeBinomial,
}

#[cfg(feature = "regression")]
impl SmoothFamily {
    /// `binomial("logit")` — logistic regression.
    pub fn binomial() -> Self {
        SmoothFamily::Binomial(SmoothBinomialLink::Logit)
    }

    /// `Gamma("inverse")` — R's canonical Gamma link.
    pub fn gamma() -> Self {
        SmoothFamily::Gamma(SmoothGammaLink::Inverse)
    }
}

/// Link function for [`SmoothFamily::Binomial`].
#[cfg(feature = "regression")]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SmoothBinomialLink {
    /// `log(μ / (1 − μ))` (canonical).
    #[default]
    Logit,
    /// `Φ⁻¹(μ)`.
    Probit,
    /// `log(−log(1 − μ))`.
    Cloglog,
}

/// Link function for [`SmoothFamily::Gamma`].
#[cfg(feature = "regression")]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SmoothGammaLink {
    /// `1 / μ` (R's canonical Gamma link).
    #[default]
    Inverse,
    /// `log(μ)`.
    Log,
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
            #[cfg(feature = "regression")]
            SmoothMethod::Gam => self.compute_gam(data),
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
                // 95% confidence interval for the mean response: R's
                // qt(0.975, n − 2), not the large-sample normal 1.96.
                let t_val = crate::stat::dist::qt(0.975, (n - 2.0).max(1.0));
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
            FittedRegressor, HuberRegressor, OlsRegressor, Regressor,
        };
        use anofox_regression::{IntervalType, RegressionOptions};
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
            Some(family) => match glm_link_prediction(family, &x, &y, &grid, self.se) {
                Some(p) => p,
                None => return DataFrame::new(),
            },
        };

        let mut x_vals = Vec::with_capacity(steps);
        let mut y_vals = Vec::with_capacity(steps);
        let mut ymin_vals = Vec::with_capacity(steps);
        let mut ymax_vals = Vec::with_capacity(steps);
        for k in 0..steps {
            x_vals.push(Value::Float(grid[(k, 0)]));
            y_vals.push(Value::Float(pred.fit[k]));
            if self.se {
                // An inverse link can swap the ends (e.g. Gamma's 1/η).
                let (a, b) = (pred.lower[k], pred.upper[k]);
                ymin_vals.push(Value::Float(a.min(b)));
                ymax_vals.push(Value::Float(a.max(b)));
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

    /// GAM smoothing via anofox-regression's penalized B-spline (P-spline) with
    /// GCV-selected smoothing — ggplot2's `method = "gam"`. Emits a t-based
    /// confidence ribbon (ymin/ymax) when `self.se` is set.
    #[cfg(feature = "regression")]
    fn compute_gam(&self, data: &DataFrame) -> DataFrame {
        use anofox_regression::solvers::{FittedRegressor, PSplineRegressor, Regressor};
        use anofox_regression::IntervalType;
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
        // The P-spline needs enough distinct support to build a basis; fall back
        // to a straight line for tiny groups.
        if pairs.len() < 6 {
            return self.compute_lm(data);
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

        let pred = match PSplineRegressor::new().fit(&x, &y) {
            Ok(f) => f.predict_with_interval(&grid, interval, 0.95),
            // Degenerate input (e.g. collinear x) — fall back to a line.
            Err(_) => return self.compute_lm(data),
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

/// IRLS convergence settings shared with the anofox-statistics DuckDB
/// extension's GLM aggregates (R's `glm.control(epsilon = 1e-8)`), so a fit in
/// SQL and the plotted smooth agree numerically.
#[cfg(feature = "regression")]
const GLM_TOL: f64 = 1e-8;
#[cfg(feature = "regression")]
const GLM_MAX_ITER: usize = 100;

/// Fitted mean and (optionally) a 95% confidence band for a non-Gaussian GLM
/// family over `grid`, computed like ggplot2's `predictdf.glm`: the band is
/// `linkinv(η ± qnorm(0.975)·se(η))`. `None` when the fit fails (degenerate
/// or out-of-range responses, e.g. a negative count).
#[cfg(feature = "regression")]
fn glm_link_prediction(
    family: SmoothFamily,
    x: &faer::Mat<f64>,
    y: &faer::Col<f64>,
    grid: &faer::Mat<f64>,
    se: bool,
) -> Option<anofox_regression::PredictionResult> {
    use anofox_regression::core::PredictionType;
    use anofox_regression::solvers::{
        BinomialRegressor, GammaRegressor, NegativeBinomialRegressor, PoissonRegressor, Regressor,
    };
    use anofox_regression::{BinomialLink, PredictionResult};
    use faer::Col;

    // Link-scale prediction (η and se(η)) plus the inverse link.
    let (link_pred, linkinv): (PredictionResult, Box<dyn Fn(f64) -> f64>) = match family {
        SmoothFamily::Gaussian => return None,
        SmoothFamily::Poisson => {
            let f = PoissonRegressor::log()
                .tolerance(GLM_TOL)
                .max_iterations(GLM_MAX_ITER)
                .build()
                .fit(x, y)
                .ok()?;
            (
                f.predict_with_se(grid, PredictionType::Link, None, 0.95),
                Box::new(f64::exp),
            )
        }
        SmoothFamily::NegativeBinomial => {
            let f = NegativeBinomialRegressor::builder()
                .tolerance(GLM_TOL)
                .max_iterations(GLM_MAX_ITER)
                .build()
                .fit(x, y)
                .ok()?;
            (
                f.predict_with_se(grid, PredictionType::Link, None, 0.95),
                Box::new(f64::exp),
            )
        }
        SmoothFamily::Binomial(link) => {
            let link = match link {
                SmoothBinomialLink::Logit => BinomialLink::Logit,
                SmoothBinomialLink::Probit => BinomialLink::Probit,
                SmoothBinomialLink::Cloglog => BinomialLink::Cloglog,
            };
            let f = BinomialRegressor::builder()
                .link(link)
                .tolerance(GLM_TOL)
                .max_iterations(GLM_MAX_ITER)
                .build()
                .fit(x, y)
                .ok()?;
            (
                f.predict_with_se(grid, PredictionType::Link, None, 0.95),
                Box::new(move |eta| link.link_inverse(eta)),
            )
        }
        SmoothFamily::Gamma(link) => {
            let (power, inv): (f64, Box<dyn Fn(f64) -> f64>) = match link {
                SmoothGammaLink::Inverse => (-1.0, Box::new(|eta: f64| 1.0 / eta)),
                SmoothGammaLink::Log => (0.0, Box::new(f64::exp)),
            };
            let f = GammaRegressor::builder()
                .link_power(power)
                .tolerance(GLM_TOL)
                .max_iterations(GLM_MAX_ITER)
                .build()
                .fit(x, y)
                .ok()?;
            (
                f.inner()
                    .predict_with_se(grid, PredictionType::Link, None, 0.95),
                inv,
            )
        }
    };

    let n = grid.nrows();
    let eta = &link_pred.fit;
    let fit = Col::from_fn(n, |k| linkinv(eta[k]));
    if !se {
        return Some(PredictionResult::with_intervals(
            fit,
            Col::zeros(n),
            Col::zeros(n),
            Col::zeros(n),
        ));
    }
    let z = crate::stat::dist::qnorm(0.975);
    let lower = Col::from_fn(n, |k| linkinv(eta[k] - z * link_pred.se[k]));
    let upper = Col::from_fn(n, |k| linkinv(eta[k] + z * link_pred.se[k]));
    Some(PredictionResult::with_intervals(
        fit,
        lower,
        upper,
        link_pred.se.clone(),
    ))
}
