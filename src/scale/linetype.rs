use crate::aes::Aesthetic;
use crate::data::Value;
use crate::render::backend::Linetype;

use super::Scale;

/// Discrete linetype scale — maps categories to line dash patterns.
#[derive(Clone, Debug)]
pub struct ScaleLinetypeDiscrete {
    name: String,
    levels: Vec<String>,
}

impl Default for ScaleLinetypeDiscrete {
    fn default() -> Self {
        Self::new()
    }
}

impl ScaleLinetypeDiscrete {
    pub fn new() -> Self {
        ScaleLinetypeDiscrete {
            name: String::new(),
            levels: Vec::new(),
        }
    }
}

impl Scale for ScaleLinetypeDiscrete {
    fn aesthetic(&self) -> Aesthetic {
        Aesthetic::Linetype
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
        self.levels
            .iter()
            .position(|l| l == &key)
            .map(|i| i as f64)
            .unwrap_or(0.0)
    }

    fn breaks(&self) -> Vec<(f64, String)> {
        self.levels
            .iter()
            .enumerate()
            .map(|(i, label)| (i as f64, label.clone()))
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

    fn map_to_linetype(&self, value: &Value) -> Option<Linetype> {
        let key = value.to_group_key();
        let idx = self.levels.iter().position(|l| l == &key).unwrap_or(0);
        Some(Linetype::ALL[idx % Linetype::ALL.len()])
    }
}
