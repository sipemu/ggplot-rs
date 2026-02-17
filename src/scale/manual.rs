use crate::aes::Aesthetic;
use crate::data::Value;

use super::color::RGBAColor;
use super::Scale;

/// Manual color scale — maps named levels to user-specified colors.
#[derive(Clone, Debug)]
pub struct ScaleManual {
    aesthetic: Aesthetic,
    name: String,
    levels: Vec<String>,
    colors: Vec<RGBAColor>,
}

impl ScaleManual {
    pub fn new(aesthetic: Aesthetic, values: Vec<(&str, RGBAColor)>) -> Self {
        let levels: Vec<String> = values.iter().map(|(k, _)| k.to_string()).collect();
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

    fn map_to_color(&self, value: &Value) -> Option<(u8, u8, u8)> {
        let key = value.to_group_key();
        let idx = self.levels.iter().position(|l| l == &key).unwrap_or(0);
        if idx < self.colors.len() {
            let c = self.colors[idx];
            Some((c.r, c.g, c.b))
        } else {
            // Wrap around
            let c = self.colors[idx % self.colors.len()];
            Some((c.r, c.g, c.b))
        }
    }
}
