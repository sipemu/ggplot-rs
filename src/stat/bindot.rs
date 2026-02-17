use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Bin-packing stat for dot plots. Bins x values and assigns stacking y positions.
pub struct StatBindot {
    pub bins: usize,
}

impl Default for StatBindot {
    fn default() -> Self {
        StatBindot { bins: 30 }
    }
}

impl Stat for StatBindot {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let x_col = match data.column("x") {
            Some(c) => c,
            None => return DataFrame::new(),
        };

        let values: Vec<f64> = x_col.iter().filter_map(|v| v.as_f64()).collect();
        if values.is_empty() {
            return DataFrame::new();
        }

        let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        let (min, max) = if (max - min).abs() < f64::EPSILON {
            (min - 0.5, max + 0.5)
        } else {
            (min, max)
        };

        let bin_width = (max - min) / self.bins as f64;
        let mut bin_counts = vec![0usize; self.bins];

        // Assign each value to a bin and track its stack position
        let mut x_vals = Vec::with_capacity(values.len());
        let mut y_vals = Vec::with_capacity(values.len());

        for &v in &values {
            let bin = ((v - min) / bin_width).floor() as usize;
            let bin = bin.min(self.bins - 1);
            let center = min + (bin as f64 + 0.5) * bin_width;
            let stack_pos = bin_counts[bin];
            bin_counts[bin] += 1;

            x_vals.push(Value::Float(center));
            y_vals.push(Value::Float(stack_pos as f64 + 0.5)); // center of dot
        }

        let mut result = DataFrame::new();
        result.add_column("x".to_string(), x_vals);
        result.add_column("y".to_string(), y_vals);

        // Carry over grouping columns
        for col_name in &["color", "fill", "group"] {
            if let Some(col) = data.column(col_name) {
                if col.len() == values.len() {
                    result.add_column(col_name.to_string(), col.to_vec());
                }
            }
        }

        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X]
    }

    fn name(&self) -> &str {
        "bindot"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bindot_basic() {
        let mut data = DataFrame::new();
        let x_vals: Vec<Value> = vec![1.0, 1.1, 1.2, 2.0, 2.1, 3.0]
            .into_iter()
            .map(Value::Float)
            .collect();
        data.add_column("x".to_string(), x_vals);

        let stat = StatBindot { bins: 3 };
        let scales = ScaleSet::new();
        let result = stat.compute_group(&data, &scales);

        assert_eq!(result.nrows(), 6);
        assert!(result.column("x").is_some());
        assert!(result.column("y").is_some());
    }
}
