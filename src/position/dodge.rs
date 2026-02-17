use crate::data::{DataFrame, Value};

use super::{Position, PositionParams};

/// Place groups side-by-side.
pub struct PositionDodge;

impl Position for PositionDodge {
    fn compute(&self, data: &mut DataFrame, params: &PositionParams) {
        let x_col = match data.column("x") {
            Some(c) => c.to_vec(),
            None => return,
        };

        // Determine groups from fill or color aesthetic
        let group_col = data
            .column("fill")
            .or_else(|| data.column("color"))
            .or_else(|| data.column("group"));

        let group_keys: Vec<String> = match group_col {
            Some(col) => col.iter().map(|v| v.to_group_key()).collect(),
            None => return, // No grouping, nothing to dodge
        };

        // Find unique groups
        let mut unique_groups: Vec<String> = Vec::new();
        for g in &group_keys {
            if !unique_groups.contains(g) {
                unique_groups.push(g.clone());
            }
        }

        let n_groups = unique_groups.len() as f64;
        if n_groups <= 1.0 {
            return;
        }

        let width = params.width;
        let group_width = width / n_groups;

        let mut new_x = x_col.clone();
        for (i, (x, group)) in x_col.iter().zip(group_keys.iter()).enumerate() {
            let group_idx = unique_groups.iter().position(|g| g == group).unwrap() as f64;
            let offset = (group_idx - (n_groups - 1.0) / 2.0) * group_width;

            if let Some(x_val) = x.as_f64() {
                new_x[i] = Value::Float(x_val + offset);
            }
        }

        if let Some(col) = data.column_mut("x") {
            *col = new_x;
        }
    }

    fn name(&self) -> &str {
        "dodge"
    }
}
