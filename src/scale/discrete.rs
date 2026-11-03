use crate::aes::Aesthetic;
use crate::data::Value;
use indexmap::IndexSet;

use super::Scale;

/// Discrete scale: maps categorical values to evenly-spaced positions.
#[derive(Clone, Debug)]
pub struct ScaleDiscrete {
    aesthetic: Aesthetic,
    name: String,
    /// Insertion-ordered set: O(1) training and lookup, so many-category
    /// scales stay linear in the number of rows.
    levels: IndexSet<String>,
    custom_labels: Option<Vec<String>>,
    /// Pre-set level order/filter. When set, only these levels are shown (in this order).
    limits: Option<IndexSet<String>>,
    /// Extra room below the first and above the last level, in levels
    /// ([`ScaleDiscrete::with_expand`]).
    expand: (f64, f64),
}

impl ScaleDiscrete {
    pub fn new() -> Self {
        ScaleDiscrete {
            aesthetic: Aesthetic::X,
            name: String::new(),
            levels: IndexSet::new(),
            custom_labels: None,
            limits: None,
            expand: (0.0, 0.0),
        }
    }

    /// Extra room below the first level and above the last one, in levels
    /// (0 by default: each level sits in the middle of its `1/n` slot). A
    /// ridgeline's top ridge needs `scale - 0.5` above
    /// ([`GeomDensityRidges`](crate::geom::density_ridges::GeomDensityRidges)).
    pub fn with_expand(mut self, lower: f64, upper: f64) -> Self {
        let ok = |v: f64| if v.is_finite() { v.max(0.0) } else { 0.0 };
        self.expand = (ok(lower), ok(upper));
        self
    }

    /// The mapped position of the `i`-th of `n` levels.
    fn slot(&self, i: usize, n: usize) -> f64 {
        let (lo, hi) = self.expand;
        (i as f64 + 0.5 + lo) / (n as f64 + lo + hi)
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
    fn effective_levels(&self) -> &IndexSet<String> {
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
                // ±Inf (a rect extending to the panel edge) is not a level.
                if matches!(v, Value::Float(f) if !f.is_finite()) {
                    continue;
                }
                let key = v.key_str();
                if !self.levels.contains(key.as_ref()) {
                    self.levels.insert(key.into_owned());
                }
            }
        }
    }

    fn map(&self, value: &Value) -> f64 {
        match value {
            Value::Float(f) if *f == f64::INFINITY => return 1.0,
            Value::Float(f) if *f == f64::NEG_INFINITY => return 0.0,
            _ => {}
        }
        let key = value.key_str();
        let effective = self.effective_levels();
        let n = effective.len();
        if n == 0 {
            return 0.5;
        }
        match effective.get_index_of(key.as_ref()) {
            Some(idx) => self.slot(idx, n),
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
                let pos = self.slot(i, n);
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
