use crate::aes::Aesthetic;
use crate::data::Value;
use crate::render::backend::PointShape;

use super::Scale;

/// Manual shape scale — maps named levels to user-specified point shapes.
#[derive(Clone, Debug)]
pub struct ScaleShapeManual {
    name: String,
    levels: Vec<String>,
    shapes: Vec<PointShape>,
}

impl ScaleShapeManual {
    pub fn new(values: Vec<(&str, PointShape)>) -> Self {
        let levels: Vec<String> = values.iter().map(|(k, _)| k.to_string()).collect();
        let shapes: Vec<PointShape> = values.iter().map(|(_, s)| *s).collect();
        ScaleShapeManual {
            name: String::new(),
            levels,
            shapes,
        }
    }
}

impl Scale for ScaleShapeManual {
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
        if idx < self.shapes.len() {
            Some(self.shapes[idx])
        } else {
            Some(self.shapes[idx % self.shapes.len()])
        }
    }

    fn clone_box(&self) -> Box<dyn Scale> {
        Box::new(self.clone())
    }

    fn reset_training(&mut self) {
        self.levels.clear();
    }
}
