use crate::aes::Aesthetic;
use crate::data::Value;
use crate::render::backend::PointShape;

use super::Scale;

/// Discrete shape scale — maps categories to point shapes.
#[derive(Clone, Debug)]
pub struct ScaleShapeDiscrete {
    name: String,
    levels: Vec<String>,
}

impl Default for ScaleShapeDiscrete {
    fn default() -> Self {
        Self::new()
    }
}

impl ScaleShapeDiscrete {
    pub fn new() -> Self {
        ScaleShapeDiscrete {
            name: String::new(),
            levels: Vec::new(),
        }
    }
}

impl Scale for ScaleShapeDiscrete {
    fn aesthetic(&self) -> Aesthetic {
        Aesthetic::Shape
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

    fn map_to_shape(&self, value: &Value) -> Option<PointShape> {
        let key = value.to_group_key();
        let idx = self.levels.iter().position(|l| l == &key).unwrap_or(0);
        Some(PointShape::ALL[idx % PointShape::ALL.len()])
    }
}
