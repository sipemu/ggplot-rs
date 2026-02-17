use crate::data::{DataFrame, Value};

use super::{Position, PositionParams};

/// Stack bars/areas on top of each other.
pub struct PositionStack;

impl Position for PositionStack {
    fn compute(&self, data: &mut DataFrame, _params: &PositionParams) {
        // Group by x, accumulate y values
        let x_col = match data.column("x") {
            Some(c) => c.to_vec(),
            None => return,
        };
        let y_col = match data.column("y") {
            Some(c) => c.to_vec(),
            None => return,
        };

        // Build cumulative sums per x group
        let mut x_cumsum: Vec<(String, f64)> = Vec::new();
        let mut new_y = Vec::with_capacity(y_col.len());
        let mut ymin_vals = Vec::with_capacity(y_col.len());

        for (x, y) in x_col.iter().zip(y_col.iter()) {
            let x_key = x.to_group_key();
            let y_val = y.as_f64().unwrap_or(0.0);

            let base = x_cumsum
                .iter()
                .find(|(k, _)| k == &x_key)
                .map(|(_, v)| *v)
                .unwrap_or(0.0);

            ymin_vals.push(Value::Float(base));
            new_y.push(Value::Float(base + y_val));

            if let Some(entry) = x_cumsum.iter_mut().find(|(k, _)| k == &x_key) {
                entry.1 += y_val;
            } else {
                x_cumsum.push((x_key, y_val));
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
        "stack"
    }
}
