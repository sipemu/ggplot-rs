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

        // ggplot2 stacks the first group at the TOP (so the stack order top-to-
        // bottom matches the legend), so accumulate downward from each x's total
        // rather than upward from 0.
        let mut totals: Vec<(String, f64)> = Vec::new();
        for (x, y) in x_col.iter().zip(y_col.iter()) {
            let x_key = x.to_group_key();
            let y_val = y.as_f64().unwrap_or(0.0);
            if let Some(entry) = totals.iter_mut().find(|(k, _)| k == &x_key) {
                entry.1 += y_val;
            } else {
                totals.push((x_key, y_val));
            }
        }

        let mut consumed: Vec<(String, f64)> = Vec::new();
        let mut new_y = Vec::with_capacity(y_col.len());
        let mut ymin_vals = Vec::with_capacity(y_col.len());

        for (x, y) in x_col.iter().zip(y_col.iter()) {
            let x_key = x.to_group_key();
            let y_val = y.as_f64().unwrap_or(0.0);
            let total = totals
                .iter()
                .find(|(k, _)| k == &x_key)
                .map(|(_, v)| *v)
                .unwrap_or(0.0);
            let run = consumed
                .iter()
                .find(|(k, _)| k == &x_key)
                .map(|(_, v)| *v)
                .unwrap_or(0.0);

            // This group occupies [total - run - y, total - run] (top-down).
            new_y.push(Value::Float(total - run));
            ymin_vals.push(Value::Float(total - run - y_val));

            if let Some(entry) = consumed.iter_mut().find(|(k, _)| k == &x_key) {
                entry.1 += y_val;
            } else {
                consumed.push((x_key, y_val));
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
