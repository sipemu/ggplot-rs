use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Hexagonal binning using axial coordinates.
/// Output: x, y (hex centers), fill (count).
pub struct StatBinHex {
    pub bins_x: usize,
    pub bins_y: usize,
}

impl Default for StatBinHex {
    fn default() -> Self {
        StatBinHex {
            bins_x: 30,
            bins_y: 30,
        }
    }
}

impl Stat for StatBinHex {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let x_col = match data.column("x") {
            Some(c) => c,
            None => return DataFrame::new(),
        };
        let y_col = match data.column("y") {
            Some(c) => c,
            None => return DataFrame::new(),
        };

        let xs: Vec<f64> = x_col.iter().filter_map(|v| v.as_f64()).collect();
        let ys: Vec<f64> = y_col.iter().filter_map(|v| v.as_f64()).collect();
        let n = xs.len().min(ys.len());
        if n == 0 {
            return DataFrame::new();
        }

        let x_min = xs.iter().cloned().fold(f64::INFINITY, f64::min);
        let x_max = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let y_min = ys.iter().cloned().fold(f64::INFINITY, f64::min);
        let y_max = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        let x_range = if (x_max - x_min).abs() < f64::EPSILON {
            1.0
        } else {
            x_max - x_min
        };
        let y_range = if (y_max - y_min).abs() < f64::EPSILON {
            1.0
        } else {
            y_max - y_min
        };

        // Hex size
        let hex_w = x_range / self.bins_x as f64;
        let hex_h = y_range / self.bins_y as f64;

        // Use HashMap with (col, row) keys for hex bins
        let mut counts: std::collections::HashMap<(i64, i64), usize> =
            std::collections::HashMap::new();

        for i in 0..n {
            // Convert to hex grid coordinates
            let col = ((xs[i] - x_min) / hex_w).floor() as i64;
            let row = ((ys[i] - y_min) / hex_h).floor() as i64;

            // For offset rows, shift x
            let adj_col = if row % 2 != 0 {
                ((xs[i] - x_min - hex_w * 0.5) / hex_w).floor() as i64
            } else {
                col
            };

            *counts.entry((adj_col, row)).or_insert(0) += 1;
        }

        let mut x_vals = Vec::new();
        let mut y_vals = Vec::new();
        let mut fill_vals = Vec::new();

        for (&(col, row), &count) in &counts {
            if count == 0 {
                continue;
            }
            // Hex center
            let cx =
                x_min + (col as f64 + 0.5) * hex_w + if row % 2 != 0 { hex_w * 0.5 } else { 0.0 };
            let cy = y_min + (row as f64 + 0.5) * hex_h;

            x_vals.push(Value::Float(cx));
            y_vals.push(Value::Float(cy));
            fill_vals.push(Value::Float(count as f64));
        }

        let mut result = DataFrame::new();
        result.add_column("x".to_string(), x_vals);
        result.add_column("y".to_string(), y_vals);
        result.add_column("fill".to_string(), fill_vals);

        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn name(&self) -> &str {
        "binhex"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binhex_basic() {
        let mut data = DataFrame::new();
        let x_vals: Vec<Value> = (0..100).map(|i| Value::Float(i as f64 / 10.0)).collect();
        let y_vals: Vec<Value> = (0..100).map(|i| Value::Float(i as f64 / 5.0)).collect();
        data.add_column("x".to_string(), x_vals);
        data.add_column("y".to_string(), y_vals);

        let stat = StatBinHex {
            bins_x: 5,
            bins_y: 5,
        };
        let scales = ScaleSet::new();
        let result = stat.compute_group(&data, &scales);

        assert!(result.nrows() > 0);
        assert!(result.column("x").is_some());
        assert!(result.column("y").is_some());
        assert!(result.column("fill").is_some());
    }
}
