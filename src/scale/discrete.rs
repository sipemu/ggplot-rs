use crate::aes::Aesthetic;
use crate::data::Value;

use super::Scale;

/// Discrete scale: maps categorical values to evenly-spaced positions.
#[derive(Clone, Debug)]
pub struct ScaleDiscrete {
    aesthetic: Aesthetic,
    name: String,
    levels: Vec<String>,
    custom_labels: Option<Vec<String>>,
    /// Pre-set level order/filter. When set, only these levels are shown (in this order).
    limits: Option<Vec<String>>,
}

impl ScaleDiscrete {
    pub fn new() -> Self {
        ScaleDiscrete {
            aesthetic: Aesthetic::X,
            name: String::new(),
            levels: Vec::new(),
            custom_labels: None,
            limits: None,
        }
    }

    pub fn for_aesthetic(mut self, aes: Aesthetic) -> Self {
        self.aesthetic = aes;
        self
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// Set custom display labels for each level. Must match the number of levels.
    pub fn with_labels(mut self, labels: Vec<String>) -> Self {
        self.custom_labels = Some(labels);
        self
    }

    /// Set level order and filter. Only these levels are shown, in this order.
    /// Data values not in limits are mapped to the middle (0.5).
    pub fn with_limits(mut self, limits: Vec<&str>) -> Self {
        self.limits = Some(limits.into_iter().map(|s| s.to_string()).collect());
        self
    }
}

impl ScaleDiscrete {
    /// Get the effective levels (filtered by limits if set).
    fn effective_levels(&self) -> &[String] {
        if let Some(ref limits) = self.limits {
            limits
        } else {
            &self.levels
        }
    }
}

impl Default for ScaleDiscrete {
    fn default() -> Self {
        Self::new()
    }
}

impl Scale for ScaleDiscrete {
    fn aesthetic(&self) -> Aesthetic {
        self.aesthetic.clone()
    }

    fn train(&mut self, values: &[Value]) {
        if let Some(ref limits) = self.limits {
            // When limits are set, use them as the level order (ignore data order)
            self.levels = limits.clone();
        } else {
            for v in values {
                let key = v.to_group_key();
                if !self.levels.contains(&key) {
                    self.levels.push(key);
                }
            }
        }
    }

    fn map(&self, value: &Value) -> f64 {
        let key = value.to_group_key();
        let effective = self.effective_levels();
        let n = effective.len();
        if n == 0 {
            return 0.5;
        }
        match effective.iter().position(|l| l == &key) {
            Some(idx) => (idx as f64 + 0.5) / n as f64,
            None => 0.5, // Not in limits → maps to middle
        }
    }

    fn breaks(&self) -> Vec<(f64, String)> {
        let effective = self.effective_levels();
        let n = effective.len();
        if n == 0 {
            return vec![];
        }
        effective
            .iter()
            .enumerate()
            .map(|(i, level)| {
                let pos = (i as f64 + 0.5) / n as f64;
                let label = if let Some(ref labels) = self.custom_labels {
                    labels.get(i).cloned().unwrap_or_else(|| level.clone())
                } else {
                    level.clone()
                };
                (pos, label)
            })
            .collect()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    fn is_discrete(&self) -> bool {
        true
    }

    fn clone_box(&self) -> Box<dyn Scale> {
        Box::new(self.clone())
    }

    fn reset_training(&mut self) {
        if self.limits.is_none() {
            self.levels.clear();
        }
    }
}
