use crate::data::{DataFrame, Value};

use super::{Position, PositionParams};

/// Combined jitter and dodge position adjustment.
/// Useful for showing individual points within dodged groups (e.g., over boxplots).
pub struct PositionJitterDodge {
    pub jitter_width: f64,
    pub jitter_height: f64,
    pub dodge_width: f64,
    seed: u64,
}

impl PositionJitterDodge {
    pub fn new(jitter_width: f64, jitter_height: f64) -> Self {
        PositionJitterDodge {
            jitter_width,
            jitter_height,
            dodge_width: 0.9,
            seed: 42,
        }
    }

    pub fn with_dodge_width(mut self, width: f64) -> Self {
        self.dodge_width = width;
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    fn pseudo_random(seed: u64, i: usize) -> f64 {
        // Simple deterministic hash → uniform in [-0.5, 0.5]
        let h = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(i as u64)
            .wrapping_mul(1442695040888963407);
        let h = h ^ (h >> 33);
        let h = h.wrapping_mul(0xff51afd7ed558ccd);
        (h as f64 / u64::MAX as f64) - 0.5
    }
}

impl Default for PositionJitterDodge {
    fn default() -> Self {
        PositionJitterDodge::new(0.4, 0.0)
    }
}

impl Position for PositionJitterDodge {
    fn compute(&self, data: &mut DataFrame, _params: &PositionParams) {
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
            None => {
                // No groups — just jitter
                let mut new_x = x_col;
                for (i, v) in new_x.iter_mut().enumerate() {
                    if let Some(f) = v.as_f64() {
                        *v =
                            Value::Float(f + Self::pseudo_random(self.seed, i) * self.jitter_width);
                    }
                }
                if let Some(col) = data.column_mut("x") {
                    *col = new_x;
                }
                return;
            }
        };

        let mut unique_groups: Vec<String> = Vec::new();
        for g in &group_keys {
            if !unique_groups.contains(g) {
                unique_groups.push(g.clone());
            }
        }

        let n_groups = unique_groups.len() as f64;
        let group_width = if n_groups > 1.0 {
            self.dodge_width / n_groups
        } else {
            0.0
        };

        let mut new_x = x_col.clone();
        for (i, (x, group)) in x_col.iter().zip(group_keys.iter()).enumerate() {
            if let Some(x_val) = x.as_f64() {
                // Dodge offset
                let dodge_offset = if n_groups > 1.0 {
                    let group_idx = unique_groups.iter().position(|g| g == group).unwrap() as f64;
                    (group_idx - (n_groups - 1.0) / 2.0) * group_width
                } else {
                    0.0
                };

                // Jitter offset
                let jitter_x = Self::pseudo_random(self.seed, i) * self.jitter_width;

                new_x[i] = Value::Float(x_val + dodge_offset + jitter_x);
            }
        }

        if let Some(col) = data.column_mut("x") {
            *col = new_x;
        }

        // Y jitter
        if self.jitter_height.abs() > f64::EPSILON {
            if let Some(col) = data.column_mut("y") {
                for (i, v) in col.iter_mut().enumerate() {
                    if let Some(f) = v.as_f64() {
                        *v = Value::Float(
                            f + Self::pseudo_random(self.seed.wrapping_add(1), i)
                                * self.jitter_height,
                        );
                    }
                }
            }
        }
    }

    fn name(&self) -> &str {
        "jitterdodge"
    }
}
