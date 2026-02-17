use crate::data::{DataFrame, Value};

use super::{Position, PositionParams};

/// Like position_dodge but preserves total width and adds padding between groups.
pub struct PositionDodge2 {
    pub padding: f64,
}

impl PositionDodge2 {
    pub fn new(padding: f64) -> Self {
        PositionDodge2 { padding }
    }
}

impl Default for PositionDodge2 {
    fn default() -> Self {
        PositionDodge2 { padding: 0.1 }
    }
}

impl Position for PositionDodge2 {
    fn compute(&self, data: &mut DataFrame, params: &PositionParams) {
        let x_col = match data.column("x") {
            Some(c) => c.to_vec(),
            None => return,
        };

        let group_col = data
            .column("fill")
            .or_else(|| data.column("color"))
            .or_else(|| data.column("group"));

        let group_keys: Vec<String> = match group_col {
            Some(col) => col.iter().map(|v| v.to_group_key()).collect(),
            None => return,
        };

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
        // Shrink each element to leave padding between them
        let group_width = width / n_groups;
        let element_width = group_width * (1.0 - self.padding);

        let mut new_x = x_col.clone();
        let has_xmin = data.has_column("xmin");

        let xmin_col = data.column("xmin").map(|c| c.to_vec());
        let xmax_col = data.column("xmax").map(|c| c.to_vec());

        let mut new_xmin = xmin_col.clone();
        let mut new_xmax = xmax_col.clone();

        for (i, (x, group)) in x_col.iter().zip(group_keys.iter()).enumerate() {
            let group_idx = unique_groups.iter().position(|g| g == group).unwrap() as f64;
            let offset = (group_idx - (n_groups - 1.0) / 2.0) * group_width;

            if let Some(x_val) = x.as_f64() {
                new_x[i] = Value::Float(x_val + offset);

                // Also adjust xmin/xmax if present (for bars)
                if has_xmin {
                    if let Some(ref mut xmin) = new_xmin {
                        let center = x_val + offset;
                        xmin[i] = Value::Float(center - element_width / 2.0);
                    }
                    if let Some(ref mut xmax) = new_xmax {
                        let center = x_val + offset;
                        xmax[i] = Value::Float(center + element_width / 2.0);
                    }
                }
            }
        }

        if let Some(col) = data.column_mut("x") {
            *col = new_x;
        }
        if let Some(xmin) = new_xmin {
            if let Some(col) = data.column_mut("xmin") {
                *col = xmin;
            }
        }
        if let Some(xmax) = new_xmax {
            if let Some(col) = data.column_mut("xmax") {
                *col = xmax;
            }
        }
    }

    fn name(&self) -> &str {
        "dodge2"
    }
}
