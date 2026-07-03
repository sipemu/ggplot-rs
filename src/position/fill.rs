use crate::data::{DataFrame, Value};

use super::{Position, PositionParams};

/// Normalized stacking to 100% — like PositionStack but scales to [0, 1].
pub struct PositionFill;

impl Position for PositionFill {
    fn compute(&self, data: &mut DataFrame, _params: &PositionParams) {
        let x_col = match data.column("x") {
            Some(c) => c.to_vec(),
            None => return,
        };
        let y_col = match data.column("y") {
            Some(c) => c.to_vec(),
            None => return,
        };

        // First compute totals per x group
        let mut x_totals: Vec<(String, f64)> = Vec::new();
        for (x, y) in x_col.iter().zip(y_col.iter()) {
            let x_key = x.to_group_key();
            let y_val = y.as_f64().unwrap_or(0.0);

            if let Some(entry) = x_totals.iter_mut().find(|(k, _)| k == &x_key) {
                entry.1 += y_val;
            } else {
                x_totals.push((x_key, y_val));
            }
        }

        // Then compute normalized stacked positions
        let mut x_cumsum: Vec<(String, f64)> = Vec::new();
        let mut new_y = Vec::with_capacity(y_col.len());
        let mut ymin_vals = Vec::with_capacity(y_col.len());

        for (x, y) in x_col.iter().zip(y_col.iter()) {
            let x_key = x.to_group_key();
            let y_val = y.as_f64().unwrap_or(0.0);

            let total = x_totals
                .iter()
                .find(|(k, _)| k == &x_key)
                .map(|(_, v)| *v)
                .unwrap_or(1.0);
            let total = if total.abs() < f64::EPSILON {
                1.0
            } else {
                total
            };

            // ggplot2 puts the first group at the top, so fill downward from 1.
            let consumed = x_cumsum
                .iter()
                .find(|(k, _)| k == &x_key)
                .map(|(_, v)| *v)
                .unwrap_or(0.0);

            let norm_y = y_val / total;
            new_y.push(Value::Float(1.0 - consumed));
            ymin_vals.push(Value::Float(1.0 - consumed - norm_y));

            if let Some(entry) = x_cumsum.iter_mut().find(|(k, _)| k == &x_key) {
                entry.1 += norm_y;
            } else {
                x_cumsum.push((x_key, norm_y));
            }
        }

        if let Some(col) = data.column_mut("y") {
            *col = new_y;
        }
        if !data.has_column("ymin") {
            data.add_column("ymin".to_string(), ymin_vals);
        } else if let Some(col) = data.column_mut("ymin") {
            *col = ymin_vals;
        }
    }

    fn name(&self) -> &str {
        "fill"
    }
}
