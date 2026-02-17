use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Bins continuous x values into histogram bins.
pub struct StatBin {
    pub bins: usize,
}

impl Default for StatBin {
    fn default() -> Self {
        StatBin { bins: 30 }
    }
}

impl Stat for StatBin {
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

        // Handle case where all values are the same
        let (min, max) = if (max - min).abs() < f64::EPSILON {
            (min - 0.5, max + 0.5)
        } else {
            (min, max)
        };

        let bin_width = (max - min) / self.bins as f64;
        let mut counts = vec![0usize; self.bins];

        for &v in &values {
            let bin = ((v - min) / bin_width).floor() as usize;
            let bin = bin.min(self.bins - 1); // Clamp last value
            counts[bin] += 1;
        }

        let total = values.len() as f64;
        let mut x_vals = Vec::with_capacity(self.bins);
        let mut y_vals = Vec::with_capacity(self.bins);
        let mut density_vals = Vec::with_capacity(self.bins);
        let mut xmin_vals = Vec::with_capacity(self.bins);
        let mut xmax_vals = Vec::with_capacity(self.bins);

        for (i, &count) in counts.iter().enumerate() {
            let bin_min = min + i as f64 * bin_width;
            let bin_max = bin_min + bin_width;
            let center = (bin_min + bin_max) / 2.0;

            x_vals.push(Value::Float(center));
            y_vals.push(Value::Float(count as f64));
            density_vals.push(Value::Float(count as f64 / (total * bin_width)));
            xmin_vals.push(Value::Float(bin_min));
            xmax_vals.push(Value::Float(bin_max));
        }

        let mut result = DataFrame::new();
        result.add_column("x".to_string(), x_vals);
        result.add_column("y".to_string(), y_vals);
        result.add_column("density".to_string(), density_vals);
        result.add_column("xmin".to_string(), xmin_vals);
        result.add_column("xmax".to_string(), xmax_vals);
        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X]
    }

    fn name(&self) -> &str {
        "bin"
    }
}
