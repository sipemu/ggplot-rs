use std::borrow::Cow;
use std::collections::HashMap;

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
        super::preserve_raw_y(data, &y_col);

        // ggplot2 stacks the first group at the TOP (so the stack order top-to-
        // bottom matches the legend), so accumulate downward from each x's total
        // rather than upward from 0.
        // Per-x accumulators keyed by the borrowed x key: O(1) per row instead
        // of a linear scan over the distinct x values.
        let mut totals: HashMap<Cow<'_, str>, f64> = HashMap::new();
        for (x, y) in x_col.iter().zip(y_col.iter()) {
            *totals.entry(x.key_str()).or_insert(0.0) += y.as_f64().unwrap_or(0.0);
        }

        let mut consumed: HashMap<Cow<'_, str>, f64> = HashMap::new();
        let mut new_y = Vec::with_capacity(y_col.len());
        let mut ymin_vals = Vec::with_capacity(y_col.len());

        for (x, y) in x_col.iter().zip(y_col.iter()) {
            let x_key = x.key_str();
            let y_val = y.as_f64().unwrap_or(0.0);
            let total = totals.get(&x_key).copied().unwrap_or(0.0);
            let run = consumed.get(&x_key).copied().unwrap_or(0.0);

            // This group occupies [total - run - y, total - run] (top-down).
            new_y.push(Value::Float(total - run));
            ymin_vals.push(Value::Float(total - run - y_val));

            *consumed.entry(x_key).or_insert(0.0) += y_val;
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
