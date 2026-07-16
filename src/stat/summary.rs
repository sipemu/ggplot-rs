use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Aggregation function type for StatSummary (a single scalar per group).
#[derive(Clone)]
pub enum SummaryFun {
    Mean,
    Median,
    Min,
    Max,
    Sum,
}

impl SummaryFun {
    pub fn apply(&self, values: &[f64]) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        match self {
            SummaryFun::Mean => mean(values),
            SummaryFun::Median => {
                let mut sorted = values.to_vec();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                quantile_type7(&sorted, 0.5)
            }
            SummaryFun::Min => values.iter().cloned().fold(f64::INFINITY, f64::min),
            SummaryFun::Max => values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            SummaryFun::Sum => values.iter().sum(),
        }
    }
}

/// A `fun.data`-style summary: from a group's values it returns `(y, ymin, ymax)`
/// together — needed for measures where the interval depends on the centre (mean
/// ± CI), which the scalar [`SummaryFun`]s cannot express. These mirror R's
/// Hmisc helpers used by `ggplot2::stat_summary`.
#[derive(Clone)]
pub enum SummaryData {
    /// Mean ± standard error of the mean (`mean_se`).
    MeanSe,
    /// Mean with a normal-theory *t* confidence interval (`mean_cl_normal`).
    MeanClNormal { level: f64 },
    /// Mean with a bootstrap percentile CI (`mean_cl_boot`), `b` resamples.
    MeanClBoot { level: f64, b: usize },
    /// Mean ± `mult` × sd (`mean_sdl`; ggplot2 default `mult = 2`).
    MeanSdl { mult: f64 },
    /// Median with outer sample quantiles (`median_hilow`; `level = 0.95` →
    /// median with the 2.5% / 97.5% quantiles).
    MedianHilow { level: f64 },
}

impl SummaryData {
    /// `(y, ymin, ymax)` for a group's `values`.
    pub fn apply3(&self, values: &[f64]) -> (f64, f64, f64) {
        let n = values.len();
        if n == 0 {
            return (0.0, 0.0, 0.0);
        }
        let m = mean(values);
        match self {
            SummaryData::MeanSe => {
                let se = sd(values) / (n as f64).sqrt();
                (m, m - se, m + se)
            }
            SummaryData::MeanClNormal { level } => {
                if n < 2 {
                    return (m, m, m);
                }
                let se = sd(values) / (n as f64).sqrt();
                let t = crate::stat::dist::qt(0.5 + level / 2.0, n as f64 - 1.0);
                (m, m - t * se, m + t * se)
            }
            SummaryData::MeanSdl { mult } => {
                let s = sd(values);
                (m, m - mult * s, m + mult * s)
            }
            SummaryData::MedianHilow { level } => {
                let mut s = values.to_vec();
                s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                (
                    quantile_type7(&s, 0.5),
                    quantile_type7(&s, (1.0 - level) / 2.0),
                    quantile_type7(&s, (1.0 + level) / 2.0),
                )
            }
            SummaryData::MeanClBoot { level, b } => {
                if n < 2 {
                    return (m, m, m);
                }
                // Percentile bootstrap of the mean, seeded for reproducibility;
                // centre stays the observed mean (as in Hmisc::smean.cl.boot).
                use rand::{Rng, SeedableRng};
                let mut rng = rand::rngs::StdRng::seed_from_u64(0x5EED_B007);
                let mut means = Vec::with_capacity(*b);
                for _ in 0..*b {
                    let mut acc = 0.0;
                    for _ in 0..n {
                        acc += values[rng.gen_range(0..n)];
                    }
                    means.push(acc / n as f64);
                }
                means.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                (
                    m,
                    quantile_type7(&means, (1.0 - level) / 2.0),
                    quantile_type7(&means, (1.0 + level) / 2.0),
                )
            }
        }
    }
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

/// Sample standard deviation (denominator n − 1), matching R's `sd()`.
fn sd(v: &[f64]) -> f64 {
    let n = v.len();
    if n < 2 {
        return 0.0;
    }
    let m = mean(v);
    (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n as f64 - 1.0)).sqrt()
}

/// Type-7 (R default) quantile of an ascending-sorted slice.
fn quantile_type7(sorted: &[f64], p: f64) -> f64 {
    let n = sorted.len();
    if n == 0 {
        return f64::NAN;
    }
    if n == 1 {
        return sorted[0];
    }
    let h = (n as f64 - 1.0) * p;
    let lo = h.floor() as usize;
    let hi = (lo + 1).min(n - 1);
    sorted[lo] + (h - lo as f64) * (sorted[hi] - sorted[lo])
}

/// Summarize y values for each unique x. Either a `fun.data` measure (mean ± CI,
/// median hilow, …) or three independent scalar [`SummaryFun`]s (y / ymin / ymax).
pub struct StatSummary {
    pub fun_y: SummaryFun,
    pub fun_ymin: SummaryFun,
    pub fun_ymax: SummaryFun,
    /// When set, overrides the three scalar functions and computes y/ymin/ymax
    /// together (needed for centre-dependent intervals like mean ± CI).
    pub fun_data: Option<SummaryData>,
}

impl Default for StatSummary {
    fn default() -> Self {
        StatSummary {
            fun_y: SummaryFun::Mean,
            fun_ymin: SummaryFun::Min,
            fun_ymax: SummaryFun::Max,
            fun_data: None,
        }
    }
}

impl StatSummary {
    fn with_data(d: SummaryData) -> Self {
        StatSummary {
            fun_data: Some(d),
            ..Default::default()
        }
    }
    /// Mean ± standard error.
    pub fn mean_se() -> Self {
        Self::with_data(SummaryData::MeanSe)
    }
    /// Mean with a 95% normal-theory (t) confidence interval.
    pub fn mean_cl_normal() -> Self {
        Self::with_data(SummaryData::MeanClNormal { level: 0.95 })
    }
    /// Mean with a 95% bootstrap-percentile confidence interval.
    pub fn mean_cl_boot() -> Self {
        Self::with_data(SummaryData::MeanClBoot {
            level: 0.95,
            b: 1000,
        })
    }
    /// Mean ± 2 sd.
    pub fn mean_sdl() -> Self {
        Self::with_data(SummaryData::MeanSdl { mult: 2.0 })
    }
    /// Median with the 2.5% / 97.5% sample quantiles.
    pub fn median_hilow() -> Self {
        Self::with_data(SummaryData::MedianHilow { level: 0.95 })
    }
}

impl Stat for StatSummary {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let x_col = match data.column("x") {
            Some(c) => c,
            None => return DataFrame::new(),
        };
        let y_col = match data.column("y") {
            Some(c) => c,
            None => return DataFrame::new(),
        };

        // Group y values by x
        let mut groups: Vec<(String, Value, Vec<f64>)> = Vec::new();
        for (x, y) in x_col.iter().zip(y_col.iter()) {
            let key = x.to_group_key();
            let y_val = y.as_f64().unwrap_or(0.0);
            if let Some(entry) = groups.iter_mut().find(|(k, _, _)| k == &key) {
                entry.2.push(y_val);
            } else {
                groups.push((key, x.clone(), vec![y_val]));
            }
        }

        let n = groups.len();
        let mut x_vals = Vec::with_capacity(n);
        let mut y_vals = Vec::with_capacity(n);
        let mut ymin_vals = Vec::with_capacity(n);
        let mut ymax_vals = Vec::with_capacity(n);

        for (_, x_val, ys) in &groups {
            x_vals.push(x_val.clone());
            let (y, ymin, ymax) = match &self.fun_data {
                Some(fd) => fd.apply3(ys),
                None => (
                    self.fun_y.apply(ys),
                    self.fun_ymin.apply(ys),
                    self.fun_ymax.apply(ys),
                ),
            };
            y_vals.push(Value::Float(y));
            ymin_vals.push(Value::Float(ymin));
            ymax_vals.push(Value::Float(ymax));
        }

        let mut result = DataFrame::new();
        result.add_column("x".to_string(), x_vals);
        result.add_column("y".to_string(), y_vals);
        result.add_column("ymin".to_string(), ymin_vals);
        result.add_column("ymax".to_string(), ymax_vals);

        // Carry over grouping columns
        for col_name in &["color", "fill", "group"] {
            if let Some(col) = data.column(col_name) {
                if let Some(first) = col.first() {
                    result.add_column(col_name.to_string(), vec![first.clone(); n]);
                }
            }
        }

        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn name(&self) -> &str {
        "summary"
    }
}

#[cfg(test)]
mod tests {
    use super::SummaryData;

    // Reference values from R (mean_se / mean_cl_normal / mean_sdl / median_hilow)
    // on v = c(2,4,4,4,5,5,7,9).
    #[test]
    fn summary_data_matches_r() {
        let v = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let close = |a: f64, b: f64| (a - b).abs() < 1e-5;

        let (y, lo, hi) = SummaryData::MeanSe.apply3(&v);
        assert!(close(y, 5.0) && close(lo, 4.244071) && close(hi, 5.755929));

        let (y, lo, hi) = SummaryData::MeanClNormal { level: 0.95 }.apply3(&v);
        assert!(close(y, 5.0) && close(lo, 3.212512) && close(hi, 6.787488));

        let (y, lo, hi) = SummaryData::MeanSdl { mult: 2.0 }.apply3(&v);
        assert!(close(y, 5.0) && close(lo, 0.723820) && close(hi, 9.276180));

        let (y, lo, hi) = SummaryData::MedianHilow { level: 0.95 }.apply3(&v);
        assert!(close(y, 4.5) && close(lo, 2.35) && close(hi, 8.65));

        // Bootstrap CI is seeded → deterministic; it should centre on the mean
        // and bracket it within a sane range.
        let (y, lo, hi) = SummaryData::MeanClBoot {
            level: 0.95,
            b: 1000,
        }
        .apply3(&v);
        assert!(close(y, 5.0) && lo < 5.0 && hi > 5.0 && lo > 3.0 && hi < 7.0);
    }
}
