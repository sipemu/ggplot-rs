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

        let mut values: Vec<f64> = x_col.iter().filter_map(|v| v.as_f64()).collect();
        if values.is_empty() {
            return DataFrame::new();
        }

        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
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
