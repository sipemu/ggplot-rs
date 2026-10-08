use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Empirical cumulative distribution function.
/// Sorts x values and assigns y = rank / n.
pub struct StatEcdf;

impl Default for StatEcdf {
    fn default() -> Self {
        StatEcdf
    }
}

impl Stat for StatEcdf {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let x_col = match data.column("x") {
            Some(c) => c,
            None => return DataFrame::new(),
        };

        let mut values: Vec<f64> = x_col
            .iter()
            .filter_map(|v| v.as_f64())
            .filter(|v| v.is_finite())
            .collect();
        if values.is_empty() {
            return DataFrame::new();
        }

        values.sort_by(|a, b| a.total_cmp(b));
        let n = values.len() as f64;

        let mut x_vals = Vec::with_capacity(values.len() + 2);
        let mut y_vals = Vec::with_capacity(values.len() + 2);

        // ggplot2 pads the step to ±Inf (y = 0 before the first point, y = 1
        // after the last) so it spans the panel. Scales ignore non-finite values
        // when training, and geom_step clamps the ±Inf segments to the panel edge.
        x_vals.push(Value::Float(f64::NEG_INFINITY));
        y_vals.push(Value::Float(0.0));
        for (i, &x) in values.iter().enumerate() {
            x_vals.push(Value::Float(x));
            y_vals.push(Value::Float((i + 1) as f64 / n));
        }
        x_vals.push(Value::Float(f64::INFINITY));
        y_vals.push(Value::Float(1.0));

        let mut result = DataFrame::new();
        result.add_column("x".to_string(), x_vals);
        result.add_column("y".to_string(), y_vals);

        // Carry over grouping columns
        let nrows = values.len() + 2;
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
        vec![Aesthetic::X]
    }

    fn name(&self) -> &str {
        "ecdf"
    }
}

/// ECDF with a simultaneous Dvoretzky–Kiefer–Wolfowitz confidence band:
/// `F̂(x) ± ε`, `ε = √(ln(2 / (1 − level)) / (2n))`, clamped to `[0, 1]`.
/// Output `x` (±Inf padded, as [`StatEcdf`]), `y`, `ymin`, `ymax` — draw it
/// with `geom_stepribbon` (see `GGPlot::stat_ecdf_band`).
#[derive(Clone, Debug)]
pub struct StatEcdfBand {
    /// Confidence level (default 0.95).
    pub level: f64,
}

impl Default for StatEcdfBand {
    fn default() -> Self {
        StatEcdfBand { level: 0.95 }
    }
}

impl StatEcdfBand {
    pub fn new(level: f64) -> Self {
        StatEcdfBand { level }
    }

    /// The DKW half-width `ε` for `n` observations at this level.
    pub fn epsilon(&self, n: usize) -> f64 {
        ((2.0 / (1.0 - self.level)).ln() / (2.0 * n as f64)).sqrt()
    }
}

impl Stat for StatEcdfBand {
    fn compute_group(&self, data: &DataFrame, scales: &ScaleSet) -> DataFrame {
        if !(self.level > 0.0 && self.level < 1.0) {
            return DataFrame::new();
        }
        // Only finite x count towards n (±Inf / NaN input rows are dropped).
        let finite: Vec<Value> = data
            .column("x")
            .map(|c| {
                c.iter()
                    .filter(|v| v.as_f64().is_some_and(f64::is_finite))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let n = finite.len();
        if n == 0 {
            return DataFrame::new();
        }
        let mut input = DataFrame::new();
        input.add_column("x".into(), finite);
        for col in ["color", "fill", "group"] {
            if let Some(first) = data.column(col).and_then(|c| c.first()) {
                input.add_column(col.into(), vec![first.clone(); n]);
            }
        }
        let mut out = StatEcdf.compute_group(&input, scales);
        let eps = self.epsilon(n);
        let y: Vec<f64> = out
            .column("y")
            .map(|c| c.iter().filter_map(|v| v.as_f64()).collect())
            .unwrap_or_default();
        out.add_column(
            "ymin".into(),
            y.iter()
                .map(|&v| Value::Float((v - eps).max(0.0)))
                .collect(),
        );
        out.add_column(
            "ymax".into(),
            y.iter()
                .map(|&v| Value::Float((v + eps).min(1.0)))
                .collect(),
        );
        out
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X]
    }

    fn name(&self) -> &str {
        "ecdf_band"
    }
}
