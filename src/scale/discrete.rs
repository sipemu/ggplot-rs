use crate::aes::Aesthetic;
use crate::data::Value;

use super::Scale;

/// Discrete scale: maps categorical values to evenly-spaced positions.
#[derive(Clone, Debug)]
pub struct ScaleDiscrete {
    aesthetic: Aesthetic,
    name: String,
    levels: Vec<String>,
}

impl ScaleDiscrete {
    pub fn new() -> Self {
        ScaleDiscrete {
            aesthetic: Aesthetic::X,
            name: String::new(),
            levels: Vec::new(),
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
        for v in values {
            let key = v.to_group_key();
            if !self.levels.contains(&key) {
                self.levels.push(key);
            }
        }
    }

    fn map(&self, value: &Value) -> f64 {
        let key = value.to_group_key();
        let n = self.levels.len();
        if n == 0 {
            return 0.5;
        }
        match self.levels.iter().position(|l| l == &key) {
            // Band-based positioning: each category centered in its band
            Some(idx) => (idx as f64 + 0.5) / n as f64,
            None => 0.5,
        }
    }

    fn breaks(&self) -> Vec<(f64, String)> {
        let n = self.levels.len();
        if n == 0 {
            return vec![];
        }
        self.levels
            .iter()
            .enumerate()
            .map(|(i, label)| {
                let pos = (i as f64 + 0.5) / n as f64;
                (pos, label.clone())
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
}
