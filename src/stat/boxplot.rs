use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Computes boxplot statistics: quartiles, whiskers, outliers.
pub struct StatBoxplot;

impl Stat for StatBoxplot {
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
        let q1 = percentile(&values, 25.0);
        let median = percentile(&values, 50.0);
        let q3 = percentile(&values, 75.0);
        let iqr = q3 - q1;

        let lower_fence = q1 - 1.5 * iqr;
        let upper_fence = q3 + 1.5 * iqr;

        // Whiskers extend to most extreme non-outlier
        let ymin = values
            .iter()
            .find(|&&v| v >= lower_fence)
            .copied()
            .unwrap_or(q1);
        let ymax = values
            .iter()
            .rev()
            .find(|&&v| v <= upper_fence)
            .copied()
            .unwrap_or(q3);

        // Outliers
        let outliers: Vec<f64> = values
            .iter()
            .filter(|&&v| v < lower_fence || v > upper_fence)
            .copied()
            .collect();

        // Get x value (group identifier)
        let x_val = data
            .column("x")
            .and_then(|c| c.first())
            .cloned()
            .unwrap_or(Value::Float(0.0));

        let mut result = DataFrame::new();
        result.add_column("x".to_string(), vec![x_val.clone()]);
        result.add_column("ymin".to_string(), vec![Value::Float(ymin)]);
        result.add_column("lower".to_string(), vec![Value::Float(q1)]);
        result.add_column("middle".to_string(), vec![Value::Float(median)]);
        result.add_column("upper".to_string(), vec![Value::Float(q3)]);
        result.add_column("ymax".to_string(), vec![Value::Float(ymax)]);
        result.add_column(
            "notchupper".to_string(),
            vec![Value::Float(median + 1.58 * iqr / (n as f64).sqrt())],
        );
        result.add_column(
            "notchlower".to_string(),
            vec![Value::Float(median - 1.58 * iqr / (n as f64).sqrt())],
        );

        // Store outliers as separate rows in a companion column
        if !outliers.is_empty() {
            // We'll encode outlier count; actual outlier drawing handled by geom
            let outlier_str = outliers
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",");
            result.add_column("outliers".to_string(), vec![Value::Str(outlier_str)]);
        }

        // Carry over fill/color
        for col_name in &["fill", "color"] {
            if let Some(col) = data.column(col_name) {
                if let Some(first) = col.first() {
                    result.add_column(col_name.to_string(), vec![first.clone()]);
                }
            }
        }

        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn name(&self) -> &str {
        "boxplot"
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }

    let k = (p / 100.0) * (sorted.len() - 1) as f64;
    let f = k.floor() as usize;
    let c = k.ceil() as usize;

    if f == c {
        sorted[f]
    } else {
        let d = k - f as f64;
        sorted[f] * (1.0 - d) + sorted[c] * d
    }
}
