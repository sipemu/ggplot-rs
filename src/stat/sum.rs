use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Count overlapping (x, y) pairs. Used by geom_count.
/// Produces x, y, n (count) columns.
pub struct StatSum;

impl Default for StatSum {
    fn default() -> Self {
        StatSum
    }
}

impl Stat for StatSum {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let x_col = match data.column("x") {
            Some(c) => c,
            None => return DataFrame::new(),
        };
        let y_col = match data.column("y") {
            Some(c) => c,
            None => return DataFrame::new(),
        };

        // Group by (x, y) key pairs
        let mut groups: Vec<(String, Value, Value, usize)> = Vec::new();
        for (x, y) in x_col.iter().zip(y_col.iter()) {
            let key = format!("{}|{}", x.to_group_key(), y.to_group_key());
            if let Some(entry) = groups.iter_mut().find(|(k, _, _, _)| k == &key) {
                entry.3 += 1;
            } else {
                groups.push((key, x.clone(), y.clone(), 1));
            }
        }

        let n = groups.len();
        let mut x_vals = Vec::with_capacity(n);
        let mut y_vals = Vec::with_capacity(n);
        let mut n_vals = Vec::with_capacity(n);

        for (_, x_val, y_val, count) in groups {
            x_vals.push(x_val);
            y_vals.push(y_val);
            n_vals.push(Value::Float(count as f64));
        }

        let mut result = DataFrame::new();
        result.add_column("x".to_string(), x_vals);
        result.add_column("y".to_string(), y_vals);
        result.add_column("n".to_string(), n_vals);

        // Carry over grouping columns
        for col_name in &["color", "fill", "group"] {
            if let Some(col) = data.column(col_name) {
                if let Some(first) = col.first() {
                    result.add_column(col_name.to_string(), vec![first.clone(); n]);
                }
            }
        }

        result
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn name(&self) -> &str {
        "sum"
    }
}
