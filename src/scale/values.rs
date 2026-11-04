//! Manual and identity scales for the numeric aesthetics (#408):
//! ggplot2's `scale_size_manual()` / `scale_alpha_manual()` /
//! `scale_linewidth_manual()` ([`ScaleValueManual`]) and
//! `scale_size_identity()` / `scale_alpha_identity()` /
//! `scale_linewidth_identity()` / `scale_shape_identity()`
//! ([`ScaleValueIdentity`]).

use crate::aes::Aesthetic;
use crate::data::Value;
use crate::render::backend::PointShape;
use indexmap::IndexSet;

use super::Scale;

/// A discrete scale mapping named levels to user-chosen numbers — point
/// sizes, alpha values or line widths, by `aesthetic` (`Size`, `Alpha` or
/// `Linewidth`). Levels the data has beyond `values` cycle through them,
/// as [`ScaleShapeManual`](super::shape_manual::ScaleShapeManual) does.
#[derive(Clone, Debug)]
pub struct ScaleValueManual {
    aesthetic: Aesthetic,
    name: String,
    configured: Vec<String>,
    levels: IndexSet<String>,
    values: Vec<f64>,
}

impl ScaleValueManual {
    pub fn new(aesthetic: Aesthetic, values: Vec<(&str, f64)>) -> Self {
        let configured: Vec<String> = values.iter().map(|(k, _)| k.to_string()).collect();
        ScaleValueManual {
            aesthetic,
            name: String::new(),
            levels: configured.iter().cloned().collect(),
            configured,
            values: values.iter().map(|(_, v)| *v).collect(),
        }
    }

    fn value_for(&self, value: &Value) -> Option<f64> {
        if self.values.is_empty() {
            return None;
        }
        let idx = self
            .levels
            .get_index_of(value.key_str().as_ref())
            .unwrap_or(0);
        Some(self.values[idx % self.values.len()])
    }
}

impl Scale for ScaleValueManual {
    fn aesthetic(&self) -> Aesthetic {
        self.aesthetic.clone()
    }

    fn train(&mut self, values: &[Value]) {
        for v in values {
            if v.is_na() {
                continue;
            }
            let key = v.key_str();
            if !self.levels.contains(key.as_ref()) {
                self.levels.insert(key.into_owned());
            }
        }
    }

    fn map(&self, value: &Value) -> f64 {
        self.levels
            .get_index_of(value.key_str().as_ref())
            .map(|i| i as f64)
            .unwrap_or(0.0)
    }

    fn breaks(&self) -> Vec<(f64, String)> {
        self.levels
            .iter()
            .enumerate()
            .map(|(i, l)| (i as f64, l.clone()))
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

    fn map_to_size(&self, value: &Value) -> Option<f64> {
        match self.aesthetic {
            Aesthetic::Size | Aesthetic::Linewidth => self.value_for(value),
            _ => None,
        }
    }

    fn map_to_alpha(&self, value: &Value) -> Option<f64> {
        match self.aesthetic {
            Aesthetic::Alpha => self.value_for(value).map(|a| a.clamp(0.0, 1.0)),
            _ => None,
        }
    }

    fn clone_box(&self) -> Box<dyn Scale> {
        Box::new(self.clone())
    }

    fn reset_training(&mut self) {
        self.levels = self.configured.iter().cloned().collect();
    }
}

/// An identity scale: the data already holds the size / alpha / line width
/// (numbers) or point shape (ggplot2 `pch` codes, or a shape name such as
/// `"triangle"`) to draw with, used as-is. No legend, as in ggplot2
/// (`guide = "none"`).
#[derive(Clone, Debug)]
pub struct ScaleValueIdentity {
    aesthetic: Aesthetic,
}

impl ScaleValueIdentity {
    /// `aesthetic` is `Size`, `Alpha`, `Linewidth` or `Shape`.
    pub fn new(aesthetic: Aesthetic) -> Self {
        ScaleValueIdentity { aesthetic }
    }
}

/// A ggplot2 `pch` code or shape name as the nearest [`PointShape`].
pub(crate) fn shape_from_value(value: &Value) -> Option<PointShape> {
    if let Some(name) = value.as_str() {
        return match name.trim().to_ascii_lowercase().as_str() {
            "circle" | "circle filled" | "circle open" | "circle small" => Some(PointShape::Circle),
            "triangle" | "triangle filled" | "triangle open" => Some(PointShape::Triangle),
            "square" | "square filled" | "square open" => Some(PointShape::Square),
            "diamond" | "diamond filled" | "diamond open" => Some(PointShape::Diamond),
            "cross" => Some(PointShape::Cross),
            "plus" => Some(PointShape::Plus),
            n => n.parse::<f64>().ok().and_then(shape_from_code),
        };
    }
    value.as_f64().and_then(shape_from_code)
}

fn shape_from_code(f: f64) -> Option<PointShape> {
    if !f.is_finite() || f.fract() != 0.0 {
        return None;
    }
    match f as i64 {
        1 | 10 | 13 | 16 | 19 | 20 | 21 => Some(PointShape::Circle),
        2 | 6 | 17 | 24 | 25 => Some(PointShape::Triangle),
        0 | 7 | 12 | 14 | 15 | 22 => Some(PointShape::Square),
        5 | 9 | 18 | 23 => Some(PointShape::Diamond),
        4 | 8 => Some(PointShape::Cross),
        3 => Some(PointShape::Plus),
        _ => None,
    }
}

impl Scale for ScaleValueIdentity {
    fn aesthetic(&self) -> Aesthetic {
        self.aesthetic.clone()
    }

    fn train(&mut self, _values: &[Value]) {}

    fn map(&self, _value: &Value) -> f64 {
        0.0
    }

    fn breaks(&self) -> Vec<(f64, String)> {
        Vec::new()
    }

    fn name(&self) -> &str {
        ""
    }

    fn set_name(&mut self, _name: &str) {}

    fn map_to_size(&self, value: &Value) -> Option<f64> {
        match self.aesthetic {
            Aesthetic::Size | Aesthetic::Linewidth => {
                value.as_f64().filter(|f| f.is_finite() && *f >= 0.0)
            }
            _ => None,
        }
    }

    fn map_to_alpha(&self, value: &Value) -> Option<f64> {
        match self.aesthetic {
            Aesthetic::Alpha => value
                .as_f64()
                .filter(|f| f.is_finite())
                .map(|a| a.clamp(0.0, 1.0)),
            _ => None,
        }
    }

    fn map_to_shape(&self, value: &Value) -> Option<PointShape> {
        match self.aesthetic {
            Aesthetic::Shape => shape_from_value(value),
            _ => None,
        }
    }

    fn clone_box(&self) -> Box<dyn Scale> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_maps_levels_and_cycles() {
        let mut s = ScaleValueManual::new(Aesthetic::Size, vec![("a", 2.0), ("b", 6.0)]);
        s.train(&[Value::Str("c".into())]);
        assert_eq!(s.map_to_size(&Value::Str("b".into())), Some(6.0));
        assert_eq!(s.map_to_size(&Value::Str("c".into())), Some(2.0));
        assert_eq!(s.map_to_alpha(&Value::Str("a".into())), None);
        assert_eq!(s.breaks().len(), 3);
        s.reset_training();
        assert_eq!(s.breaks().len(), 2);
        let a = ScaleValueManual::new(Aesthetic::Alpha, vec![("a", 1.5)]);
        assert_eq!(a.map_to_alpha(&Value::Str("a".into())), Some(1.0));
    }

    #[test]
    fn identity_uses_the_values() {
        let s = ScaleValueIdentity::new(Aesthetic::Size);
        assert_eq!(s.map_to_size(&Value::Float(3.5)), Some(3.5));
        assert!(s.breaks().is_empty());
        let a = ScaleValueIdentity::new(Aesthetic::Alpha);
        assert_eq!(a.map_to_alpha(&Value::Float(0.25)), Some(0.25));
        let sh = ScaleValueIdentity::new(Aesthetic::Shape);
        assert_eq!(
            sh.map_to_shape(&Value::Integer(17)),
            Some(PointShape::Triangle)
        );
        assert_eq!(
            sh.map_to_shape(&Value::Str("diamond".into())),
            Some(PointShape::Diamond)
        );
        assert_eq!(
            sh.map_to_shape(&Value::Str("4".into())),
            Some(PointShape::Cross)
        );
        assert_eq!(sh.map_to_shape(&Value::Float(99.0)), None);
    }
}
