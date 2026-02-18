use crate::aes::Aesthetic;
use crate::data::Value;

use super::Scale;

/// Greyscale discrete color scale.
/// Maps discrete levels to evenly-spaced grey values between `start` and `end`.
/// Default range is 0.2 (dark grey) to 0.8 (light grey), where 0.0 = black and 1.0 = white.
pub struct ScaleColorGrey {
    aesthetic: Aesthetic,
    name: String,
    start: f64,
    end: f64,
    levels: Vec<String>,
}

impl ScaleColorGrey {
    pub fn new(aesthetic: Aesthetic) -> Self {
        ScaleColorGrey {
            aesthetic,
            name: String::new(),
            start: 0.2,
            end: 0.8,
            levels: Vec::new(),
        }
    }

    /// Set the grey range. `start` and `end` are values in [0, 1]
    /// where 0.0 = black and 1.0 = white.
    pub fn with_range(mut self, start: f64, end: f64) -> Self {
        self.start = start;
        self.end = end;
        self
    }

    fn grey_for_index(&self, idx: usize) -> u8 {
        let n = self.levels.len().max(1);
        let t = if n == 1 {
            (self.start + self.end) / 2.0
        } else {
            self.start + (self.end - self.start) * (idx as f64 / (n - 1) as f64)
        };
        (t.clamp(0.0, 1.0) * 255.0) as u8
    }
}

impl Scale for ScaleColorGrey {
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
        let g = self.grey_for_index(idx);
        Some((g, g, g))
    }
}
