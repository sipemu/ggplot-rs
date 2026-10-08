use crate::aes::Aesthetic;
use crate::data::Value;
use crate::render::backend::Linetype;
use indexmap::IndexSet;

use super::Scale;

/// Discrete linetype scale — maps categories to line dash patterns.
#[derive(Clone, Debug)]
pub struct ScaleLinetypeDiscrete {
    name: String,
    levels: IndexSet<String>,
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
            levels: IndexSet::new(),
        }
    }
}

impl Scale for ScaleLinetypeDiscrete {
    fn aesthetic(&self) -> Aesthetic {
        Aesthetic::Linetype
    }

    fn train(&mut self, values: &[Value]) {
        for v in values {
            let key = v.key_str();
            if !self.levels.contains(key.as_ref()) {
                self.levels.insert(key.into_owned());
            }
        }
    }

    fn map(&self, value: &Value) -> f64 {
        let key = value.key_str();
        self.levels
            .get_index_of(key.as_ref())
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
        let key = value.key_str();
        let idx = self.levels.get_index_of(key.as_ref()).unwrap_or(0);
        Some(Linetype::ALL[idx % Linetype::ALL.len()])
    }

    fn clone_box(&self) -> Box<dyn Scale> {
        Box::new(self.clone())
    }

    fn reset_training(&mut self) {
        self.levels.clear();
    }
}
