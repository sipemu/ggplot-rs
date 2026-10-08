use std::borrow::Cow;
use std::collections::HashMap;

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
        let mut x_totals: HashMap<Cow<'_, str>, f64> = HashMap::new();
        for (x, y) in x_col.iter().zip(y_col.iter()) {
            *x_totals.entry(x.key_str()).or_insert(0.0) += y.as_f64().unwrap_or(0.0);
        }

        // Then compute normalized stacked positions
        let mut x_cumsum: HashMap<Cow<'_, str>, f64> = HashMap::new();
        let mut new_y = Vec::with_capacity(y_col.len());
        let mut ymin_vals = Vec::with_capacity(y_col.len());

        for (x, y) in x_col.iter().zip(y_col.iter()) {
            let x_key = x.key_str();
            let y_val = y.as_f64().unwrap_or(0.0);

            let total = x_totals.get(&x_key).copied().unwrap_or(1.0);
            let total = if total.abs() < f64::EPSILON {
                1.0
            } else {
                total
            };

            // ggplot2 puts the first group at the top, so fill downward from 1.
            let consumed = x_cumsum.get(&x_key).copied().unwrap_or(0.0);

            let norm_y = y_val / total;
            new_y.push(Value::Float(1.0 - consumed));
            ymin_vals.push(Value::Float(1.0 - consumed - norm_y));

            *x_cumsum.entry(x_key).or_insert(0.0) += norm_y;
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
