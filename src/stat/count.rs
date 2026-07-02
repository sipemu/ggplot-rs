use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Counts occurrences of each unique x value.
pub struct StatCount;

impl Stat for StatCount {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let x_col = match data.column("x") {
            Some(c) => c,
            None => return DataFrame::new(),
        };

        // Count unique x values
        let mut counts: Vec<(String, usize)> = Vec::new();
        for v in x_col {
            let key = v.to_group_key();
            if let Some(entry) = counts.iter_mut().find(|(k, _)| k == &key) {
                entry.1 += 1;
            } else {
                counts.push((key, 1));
            }
        }

        let mut result = DataFrame::new();
        let x_values: Vec<Value> = counts.iter().map(|(k, _)| Value::Str(k.clone())).collect();

        // Try to preserve original value types
        let first_x = x_col.first();
        let x_values: Vec<Value> = if matches!(first_x, Some(Value::Float(_) | Value::Integer(_))) {
            counts
                .iter()
                .map(|(k, _)| {
                    k.parse::<f64>()
                        .map(Value::Float)
                        .unwrap_or_else(|_| Value::Str(k.clone()))
                })
                .collect()
        } else {
            x_values
        };

        let y_values: Vec<Value> = counts
            .iter()
            .map(|(_, c)| Value::Float(*c as f64))
            .collect();

        result.add_column("x".to_string(), x_values);
        result.add_column("y".to_string(), y_values.clone());
        // Expose the count under its ggplot stat name for after_stat expressions.
        result.add_column("count".to_string(), y_values);

        // Carry over group columns
        if data.has_column("fill") {
            if let Some(fill_col) = data.column("fill") {
                if let Some(first) = fill_col.first() {
                    result.add_column("fill".to_string(), vec![first.clone(); counts.len()]);
                }
            }
        }
        if data.has_column("color") {
            if let Some(color_col) = data.column("color") {
                if let Some(first) = color_col.first() {
                    result.add_column("color".to_string(), vec![first.clone(); counts.len()]);
                }
            }
        }

        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X]
    }

    fn name(&self) -> &str {
        "count"
    }
}
