//! Quantile–quantile statistics: `stat_qq`, `stat_qq_line` (ggplot2) and
//! `stat_qq_band` (qqplotr's pointwise / KS confidence envelopes) against a
//! choice of theoretical distribution.
//!
//! Plotting positions are R's `ppoints(n)` (`a = 3/8` for `n ≤ 10`, else
//! `1/2`); sample quantiles for the reference line use R's default type-7
//! interpolation. All values are validated against R (`stats::qqnorm`,
//! ggplot2 4.0 `stat_qq`/`stat_qq_line`, qqplotr 0.0.7 `stat_qq_band`) in
//! `tests/qq_dist_r.rs`.

use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::distribution as d;
use super::Stat;

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

/// R's `ppoints(n)`.
pub fn ppoints(n: usize) -> Vec<f64> {
    let a = if n <= 10 { 3.0 / 8.0 } else { 0.5 };
    (0..n)
        .map(|i| (i as f64 + 1.0 - a) / (n as f64 + 1.0 - 2.0 * a))
        .collect()
}

/// Theoretical distribution of a QQ plot, with its parameters (ggplot2's
/// `distribution` + `dparams`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum QQDistribution {
    /// Normal (`qnorm`), default `mean = 0, sd = 1`.
    Normal { mean: f64, sd: f64 },
    /// Student t (`qt`) with `df` degrees of freedom.
    StudentT { df: f64 },
    /// Exponential (`qexp`) with the given `rate`.
    Exponential { rate: f64 },
    /// Half-normal — `|Z|·sd`, the reference for half-normal plots of
    /// absolute residuals: quantile `sd · qnorm((1 + p) / 2)`.
    HalfNormal { sd: f64 },
}

impl Default for QQDistribution {
    fn default() -> Self {
        QQDistribution::normal()
    }
}

impl QQDistribution {
    /// Standard normal.
    pub fn normal() -> Self {
        QQDistribution::Normal { mean: 0.0, sd: 1.0 }
    }
    /// Student t with `df` degrees of freedom.
    pub fn t(df: f64) -> Self {
        QQDistribution::StudentT { df }
    }
    /// Exponential with rate 1.
    pub fn exponential() -> Self {
        QQDistribution::Exponential { rate: 1.0 }
    }
    /// Half-normal with `sd = 1`.
    pub fn half_normal() -> Self {
        QQDistribution::HalfNormal { sd: 1.0 }
    }

    /// Quantile function (`q*` in R). NaN for invalid parameters.
    pub fn quantile(&self, p: f64) -> f64 {
        match *self {
            QQDistribution::Normal { mean, sd } if sd > 0.0 => mean + sd * d::qnorm(p),
            QQDistribution::StudentT { df } if df > 0.0 => d::qt(p, df),
            QQDistribution::Exponential { rate } if rate > 0.0 => {
                if !(0.0..=1.0).contains(&p) {
                    f64::NAN
                } else {
                    -(-p).ln_1p() / rate
                }
            }
            QQDistribution::HalfNormal { sd } if sd > 0.0 => {
                if !(0.0..=1.0).contains(&p) {
                    f64::NAN
                } else {
                    sd * d::qnorm((1.0 + p) / 2.0)
                }
            }
            _ => f64::NAN,
        }
    }

    /// Density (`d*` in R) — the pointwise band's standard error uses it.
    pub fn density(&self, x: f64) -> f64 {
        match *self {
            QQDistribution::Normal { mean, sd } if sd > 0.0 => d::dnorm((x - mean) / sd) / sd,
            QQDistribution::StudentT { df } if df > 0.0 => d::dt(x, df),
            QQDistribution::Exponential { rate } if rate > 0.0 => {
                if x < 0.0 {
                    0.0
                } else {
                    rate * (-rate * x).exp()
                }
            }
            QQDistribution::HalfNormal { sd } if sd > 0.0 => {
                if x < 0.0 {
                    0.0
                } else {
                    2.0 * d::dnorm(x / sd) / sd
                }
            }
            _ => f64::NAN,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            QQDistribution::Normal { .. } => "norm",
            QQDistribution::StudentT { .. } => "t",
            QQDistribution::Exponential { .. } => "exp",
            QQDistribution::HalfNormal { .. } => "halfnorm",
        }
    }
}

/// The finite sample values of `y`, sorted.
fn sorted_sample(data: &DataFrame) -> Vec<f64> {
    let mut v: Vec<f64> = data
        .column("y")
        .map(|c| {
            c.iter()
                .filter_map(|v| v.as_f64())
                .filter(|v| v.is_finite())
                .collect()
        })
        .unwrap_or_default();
    v.sort_by(|a, b| a.total_cmp(b));
    v
}

/// Copy the group-identifying columns (first value) onto `n` output rows.
fn carry_groups(data: &DataFrame, out: &mut DataFrame, n: usize) {
    for col_name in ["color", "fill", "group", "linetype"] {
        if let Some(first) = data.column(col_name).and_then(|c| c.first()) {
            out.add_column(col_name.to_string(), vec![first.clone(); n]);
        }
    }
}

fn floats(v: impl IntoIterator<Item = f64>) -> Vec<Value> {
    v.into_iter().map(Value::Float).collect()
}

/// Slope / intercept of the line through the `line_p` quantiles (sample
/// type-7 vs theoretical), as ggplot2's `stat_qq_line`.
fn qq_line_coef(sorted: &[f64], dist: &QQDistribution, line_p: (f64, f64)) -> (f64, f64) {
    let (x1, x2) = (dist.quantile(line_p.0), dist.quantile(line_p.1));
    let (y1, y2) = (
        quantile_type7(sorted, line_p.0),
        quantile_type7(sorted, line_p.1),
    );
    let slope = (y2 - y1) / (x2 - x1);
    (slope, y1 - slope * x1)
}

/// `stat_qq` against any [`QQDistribution`]: `x` = theoretical quantiles at
/// `ppoints(n)`, `y` = the sorted sample (also as `theoretical` / `sample`).
#[derive(Clone, Debug, Default)]
pub struct StatQQDist {
    pub distribution: QQDistribution,
}

impl StatQQDist {
    pub fn new(distribution: QQDistribution) -> Self {
        StatQQDist { distribution }
    }
}

impl Stat for StatQQDist {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let values = sorted_sample(data);
        if values.is_empty() {
            return DataFrame::new();
        }
        let n = values.len();
        let theo: Vec<f64> = ppoints(n)
            .into_iter()
            .map(|p| self.distribution.quantile(p))
            .collect();
        let mut result = DataFrame::new();
        result.add_column("x".to_string(), floats(theo.iter().copied()));
        result.add_column("y".to_string(), floats(values.iter().copied()));
        result.add_column("theoretical".to_string(), floats(theo));
        result.add_column("sample".to_string(), floats(values));
        carry_groups(data, &mut result, n);
        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Y]
    }

    fn name(&self) -> &str {
        "qq"
    }
}

/// `stat_qq_line` against any [`QQDistribution`]: the line through the
/// `line_p` (default 1st/3rd quartile) points, drawn over the range of the
/// theoretical quantiles. Output `x`, `y` (2 rows) plus `slope`, `intercept`.
#[derive(Clone, Debug)]
pub struct StatQQLineDist {
    pub distribution: QQDistribution,
    /// Probabilities of the two quantiles the line passes through
    /// (ggplot2's `line.p`, default `(0.25, 0.75)`).
    pub line_p: (f64, f64),
}

impl Default for StatQQLineDist {
    fn default() -> Self {
        StatQQLineDist {
            distribution: QQDistribution::default(),
            line_p: (0.25, 0.75),
        }
    }
}

impl StatQQLineDist {
    pub fn new(distribution: QQDistribution) -> Self {
        StatQQLineDist {
            distribution,
            ..Default::default()
        }
    }
}

impl Stat for StatQQLineDist {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let values = sorted_sample(data);
        let n = values.len();
        if n < 2 {
            return DataFrame::new();
        }
        let (slope, intercept) = qq_line_coef(&values, &self.distribution, self.line_p);
        let pp = ppoints(n);
        let x_min = self.distribution.quantile(pp[0]);
        let x_max = self.distribution.quantile(pp[n - 1]);
        let mut result = DataFrame::new();
        result.add_column("x".to_string(), floats([x_min, x_max]));
        result.add_column(
            "y".to_string(),
            floats([intercept + slope * x_min, intercept + slope * x_max]),
        );
        result.add_column("slope".to_string(), floats([slope, slope]));
        result.add_column("intercept".to_string(), floats([intercept, intercept]));
        carry_groups(data, &mut result, 2);
        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Y]
    }

    fn name(&self) -> &str {
        "qq_line"
    }
}

/// Confidence-band construction for [`StatQQBand`] (qqplotr's `bandType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum QQBandType {
    /// Pointwise normal-theory band: `fitted ± z · slope/f(q) · √(p(1−p)/n)`.
    #[default]
    Pointwise,
    /// Simultaneous Kolmogorov–Smirnov (DKW) band: the line evaluated at the
    /// quantiles of `p ± ε`, `ε = √(ln(2/(1−level)) / (2n))`. Probabilities
    /// clamped to 0 / 1 give `±Inf` bounds, drawn to the panel edge.
    Ks,
}

/// `stat_qq_band` (qqplotr): a confidence envelope around the QQ reference
/// line. Output per sample point: `x` (theoretical quantile), `ymin`, `ymax`
/// and `y` (the line's fitted value).
#[derive(Clone, Debug)]
pub struct StatQQBand {
    pub distribution: QQDistribution,
    pub band: QQBandType,
    /// Confidence level (qqplotr's `conf`, default 0.95).
    pub level: f64,
    /// Quantile probabilities of the reference line (default quartiles).
    pub line_p: (f64, f64),
}

impl Default for StatQQBand {
    fn default() -> Self {
        StatQQBand {
            distribution: QQDistribution::default(),
            band: QQBandType::Pointwise,
            level: 0.95,
            line_p: (0.25, 0.75),
        }
    }
}

impl StatQQBand {
    pub fn new(distribution: QQDistribution) -> Self {
        StatQQBand {
            distribution,
            ..Default::default()
        }
    }
    /// Use the given band type.
    pub fn band(mut self, band: QQBandType) -> Self {
        self.band = band;
        self
    }
    /// Set the confidence level (e.g. 0.95).
    pub fn level(mut self, level: f64) -> Self {
        self.level = level;
        self
    }
}

impl Stat for StatQQBand {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let values = sorted_sample(data);
        let n = values.len();
        if n < 2 || !(self.level > 0.0 && self.level < 1.0) {
            return DataFrame::new();
        }
        let dist = &self.distribution;
        let (slope, intercept) = qq_line_coef(&values, dist, self.line_p);
        let probs = ppoints(n);
        let nf = n as f64;
        let mut xs = Vec::with_capacity(n);
        let mut fit = Vec::with_capacity(n);
        let mut lo = Vec::with_capacity(n);
        let mut hi = Vec::with_capacity(n);
        let z = d::qnorm(1.0 - (1.0 - self.level) / 2.0);
        let eps = ((2.0 / (1.0 - self.level)).ln() / (2.0 * nf)).sqrt();
        for &p in &probs {
            let q = dist.quantile(p);
            let fitted = intercept + slope * q;
            let (l, u) = match self.band {
                QQBandType::Pointwise => {
                    let se = slope / dist.density(q) * (p * (1.0 - p) / nf).sqrt();
                    (fitted - z * se, fitted + z * se)
                }
                QQBandType::Ks => {
                    let lp = (p - eps).max(0.0);
                    let up = (p + eps).min(1.0);
                    (
                        intercept + slope * dist.quantile(lp),
                        intercept + slope * dist.quantile(up),
                    )
                }
            };
            // A negative slope (or a degenerate one) swaps the bounds.
            let (l, u) = if l <= u { (l, u) } else { (u, l) };
            xs.push(q);
            fit.push(fitted);
            lo.push(l);
            hi.push(u);
        }
        let mut result = DataFrame::new();
        result.add_column("x".to_string(), floats(xs));
        result.add_column("y".to_string(), floats(fit));
        result.add_column("ymin".to_string(), floats(lo));
        result.add_column("ymax".to_string(), floats(hi));
        carry_groups(data, &mut result, n);
        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Y]
    }

    fn name(&self) -> &str {
        match self.band {
            QQBandType::Pointwise => "qq_band",
            QQBandType::Ks => "qq_band_ks",
        }
    }
}

/// StatQQ: sort sample, compute theoretical *standard-normal* quantiles.
/// Output: x (theoretical quantiles), y (sample sorted). Equivalent to
/// `StatQQDist::default()`.
pub struct StatQQ;

impl Stat for StatQQ {
    fn compute_group(&self, data: &DataFrame, scales: &ScaleSet) -> DataFrame {
        StatQQDist::default().compute_group(data, scales)
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Y]
    }

    fn name(&self) -> &str {
        "qq"
    }
}

/// StatQQLine: line through the sample/normal 1st and 3rd quartiles.
/// Equivalent to `StatQQLineDist::default()`.
pub struct StatQQLine;

impl Stat for StatQQLine {
    fn compute_group(&self, data: &DataFrame, scales: &ScaleSet) -> DataFrame {
        StatQQLineDist::default().compute_group(data, scales)
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Y]
    }

    fn name(&self) -> &str {
        "qq_line"
    }
}

impl std::fmt::Display for QQDistribution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(vals: &[f64]) -> DataFrame {
        let mut data = DataFrame::new();
        data.add_column("y".to_string(), floats(vals.iter().copied()));
        data
    }

    #[test]
    fn test_stat_qq() {
        let data = sample(&(0..100).map(|i| i as f64).collect::<Vec<_>>());
        let result = StatQQ.compute_group(&data, &ScaleSet::new());
        assert_eq!(result.nrows(), 100);
        let x = result.column("x").unwrap();
        let y = result.column("y").unwrap();
        for i in 1..y.len() {
            assert!(y[i].as_f64().unwrap() >= y[i - 1].as_f64().unwrap());
            assert!(x[i].as_f64().unwrap() >= x[i - 1].as_f64().unwrap());
        }
    }

    #[test]
    fn test_stat_qq_line() {
        let data = sample(&(0..100).map(|i| i as f64).collect::<Vec<_>>());
        let result = StatQQLine.compute_group(&data, &ScaleSet::new());
        assert_eq!(result.nrows(), 2);
    }

    #[test]
    fn quantiles_of_each_distribution() {
        assert!((QQDistribution::exponential().quantile(0.5) - 2f64.ln()).abs() < 1e-15);
        let hn = QQDistribution::half_normal();
        assert!((hn.quantile(0.5) - d::qnorm(0.75)).abs() < 1e-15);
        assert_eq!(hn.quantile(0.0), 0.0);
        assert!(QQDistribution::t(0.0).quantile(0.3).is_nan());
        let n = QQDistribution::Normal { mean: 2.0, sd: 3.0 };
        assert!((n.quantile(0.975) - (2.0 + 3.0 * 1.959_963_984_540_054)).abs() < 1e-12);
    }

    #[test]
    fn band_contains_line_and_ks_reaches_infinity() {
        let vals: Vec<f64> = (1..=30).map(|i| (i as f64 * 0.37).sin() * 2.0).collect();
        let data = sample(&vals);
        let pw = StatQQBand::default().compute_group(&data, &ScaleSet::new());
        let (lo, mid, hi) = (
            pw.column("ymin").unwrap(),
            pw.column("y").unwrap(),
            pw.column("ymax").unwrap(),
        );
        for i in 0..pw.nrows() {
            let (l, m, h) = (
                lo[i].as_f64().unwrap(),
                mid[i].as_f64().unwrap(),
                hi[i].as_f64().unwrap(),
            );
            assert!(l < m && m < h);
        }
        let ks = StatQQBand::default()
            .band(QQBandType::Ks)
            .compute_group(&data, &ScaleSet::new());
        let lo = ks.column("ymin").unwrap();
        assert_eq!(lo[0].as_f64(), Some(f64::NEG_INFINITY));
    }

    #[test]
    fn non_finite_and_tiny_samples() {
        let data = sample(&[f64::NAN, 1.0, f64::INFINITY]);
        assert_eq!(
            StatQQDist::default()
                .compute_group(&data, &ScaleSet::new())
                .nrows(),
            1
        );
        assert_eq!(
            StatQQBand::default()
                .compute_group(&data, &ScaleSet::new())
                .nrows(),
            0
        );
    }
}
