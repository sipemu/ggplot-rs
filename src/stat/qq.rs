use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;
use crate::stat::dist::qnorm;

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

/// StatQQ: sort sample, compute theoretical normal quantiles.
/// Output: x (theoretical quantiles), y (sample sorted).
pub struct StatQQ;

impl Stat for StatQQ {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let y_col = match data.column("y") {
            Some(c) => c,
            None => return DataFrame::new(),
        };

        let mut values: Vec<f64> = y_col.iter().filter_map(|v| v.as_f64()).collect();
        if values.is_empty() {
            return DataFrame::new();
        }

        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = values.len();

        let mut x_vals = Vec::with_capacity(n);
        let mut y_vals = Vec::with_capacity(n);

        for (i, &val) in values.iter().enumerate() {
            // R's ppoints(): (i + 1 - a) / (n + 1 - 2*a) where a = 3/8 for n <= 10 (matches R's ppoints)
            let a = if n <= 10 { 3.0 / 8.0 } else { 0.5 };
            let p = (i as f64 + 1.0 - a) / (n as f64 + 1.0 - 2.0 * a);
            let theoretical = qnorm(p);
            x_vals.push(Value::Float(theoretical));
            y_vals.push(Value::Float(val));
        }

        let mut result = DataFrame::new();
        result.add_column("x".to_string(), x_vals);
        result.add_column("y".to_string(), y_vals);

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
        vec![Aesthetic::Y]
    }

    fn name(&self) -> &str {
        "qq"
    }
}

/// StatQQLine: fit line through Q1/Q3 of sample vs theoretical.
/// Output: x, y (two points defining the reference line).
pub struct StatQQLine;

impl Stat for StatQQLine {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let y_col = match data.column("y") {
            Some(c) => c,
            None => return DataFrame::new(),
        };

        let mut values: Vec<f64> = y_col.iter().filter_map(|v| v.as_f64()).collect();
        if values.len() < 4 {
            return DataFrame::new();
        }

        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = values.len();

        // Sample Q1 and Q3 using R-compatible type-7 quantile interpolation
        let sample_q1 = quantile_type7(&values, 0.25);
        let sample_q3 = quantile_type7(&values, 0.75);

        // Theoretical Q1 and Q3
        let theo_q1 = qnorm(0.25);
        let theo_q3 = qnorm(0.75);

        // Line through (theo_q1, sample_q1) and (theo_q3, sample_q3)
        let slope = (sample_q3 - sample_q1) / (theo_q3 - theo_q1);
        let intercept = sample_q1 - slope * theo_q1;

        // Extend line to cover full theoretical range using R's ppoints formula
        let a = if n <= 10 { 3.0 / 8.0 } else { 0.5 };
        let x_min = qnorm((1.0 - a) / (n as f64 + 1.0 - 2.0 * a));
        let x_max = qnorm((n as f64 - a) / (n as f64 + 1.0 - 2.0 * a));

        let mut result = DataFrame::new();
        result.add_column(
            "x".to_string(),
            vec![Value::Float(x_min), Value::Float(x_max)],
        );
        result.add_column(
            "y".to_string(),
            vec![
                Value::Float(intercept + slope * x_min),
                Value::Float(intercept + slope * x_max),
            ],
        );

        // Carry over grouping columns
        for col_name in &["color", "fill", "group"] {
            if let Some(col) = data.column(col_name) {
                if let Some(first) = col.first() {
                    result.add_column(col_name.to_string(), vec![first.clone(); 2]);
                }
            }
        }

        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Y]
    }

    fn name(&self) -> &str {
        "qq_line"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qnorm_symmetry() {
        // `qnorm` now lives in stat::dist (exact under `regression`, A&S
        // approximation otherwise); check it stays symmetric in either build.
        let q = qnorm(0.5);
        assert!((q).abs() < 0.01, "qnorm(0.5) should be ~0, got {q}");

        let q1 = qnorm(0.25);
        let q3 = qnorm(0.75);
        assert!((q1 + q3).abs() < 0.01, "qnorm should be symmetric");
        assert!(q1 < 0.0);
        assert!(q3 > 0.0);
    }

    #[test]
    fn test_stat_qq() {
        let mut data = DataFrame::new();
        let y_vals: Vec<Value> = (0..100).map(|i| Value::Float(i as f64)).collect();
        data.add_column("y".to_string(), y_vals);

        let stat = StatQQ;
        let scales = ScaleSet::new();
        let result = stat.compute_group(&data, &scales);

        assert_eq!(result.nrows(), 100);
        let x = result.column("x").unwrap();
        let y = result.column("y").unwrap();
        // y should be sorted
        for i in 1..y.len() {
            assert!(y[i].as_f64().unwrap() >= y[i - 1].as_f64().unwrap());
        }
        // x should be sorted (theoretical quantiles)
        for i in 1..x.len() {
            assert!(x[i].as_f64().unwrap() >= x[i - 1].as_f64().unwrap());
        }
    }

    #[test]
    fn test_stat_qq_line() {
        let mut data = DataFrame::new();
        let y_vals: Vec<Value> = (0..100).map(|i| Value::Float(i as f64)).collect();
        data.add_column("y".to_string(), y_vals);

        let stat = StatQQLine;
        let scales = ScaleSet::new();
        let result = stat.compute_group(&data, &scales);

        assert_eq!(result.nrows(), 2);
    }
}
