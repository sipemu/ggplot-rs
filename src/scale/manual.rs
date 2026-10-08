use crate::aes::Aesthetic;
use crate::data::Value;
use indexmap::IndexSet;

use super::color::RGBAColor;
use super::Scale;

/// Manual color scale — maps named levels to user-specified colors.
#[derive(Clone, Debug)]
pub struct ScaleManual {
    aesthetic: Aesthetic,
    name: String,
    levels: IndexSet<String>,
    colors: Vec<RGBAColor>,
}

impl ScaleManual {
    pub fn new(aesthetic: Aesthetic, values: Vec<(&str, RGBAColor)>) -> Self {
        let levels: IndexSet<String> = values.iter().map(|(k, _)| k.to_string()).collect();
        let colors: Vec<RGBAColor> = values.iter().map(|(_, c)| *c).collect();
        ScaleManual {
            aesthetic,
            name: String::new(),
            levels,
            colors,
        }
    }
}

impl Scale for ScaleManual {
    fn aesthetic(&self) -> Aesthetic {
        self.aesthetic.clone()
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

    fn map_to_color(&self, value: &Value) -> Option<(u8, u8, u8)> {
        // No values supplied (`scale_color_manual(vec![])`): no mapping, so
        // the geom falls back to its default colour instead of panicking.
        if self.colors.is_empty() {
            return None;
        }
        let key = value.key_str();
        let idx = self.levels.get_index_of(key.as_ref()).unwrap_or(0);
        // Wrap around when there are more levels than colours.
        let c = self.colors[idx % self.colors.len()];
        Some((c.r, c.g, c.b))
    }

    fn clone_box(&self) -> Box<dyn Scale> {
        Box::new(self.clone())
    }

    fn reset_training(&mut self) {
        self.levels.clear();
    }
}
