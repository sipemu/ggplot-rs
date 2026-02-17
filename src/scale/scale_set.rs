use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::render::backend::{Linetype, PointShape};
use crate::scale::alpha::ScaleAlphaContinuous;
use crate::scale::color::{ScaleColorContinuous, ScaleColorDiscrete};
use crate::scale::continuous::ScaleContinuous;
use crate::scale::datetime::ScaleDateTime;
use crate::scale::discrete::ScaleDiscrete;
use crate::scale::linetype::ScaleLinetypeDiscrete;
use crate::scale::shape::ScaleShapeDiscrete;
use crate::scale::size::ScaleSizeContinuous;

use super::Scale;

/// Registry of all scales for a plot. Handles auto-detection and training.
pub struct ScaleSet {
    scales: Vec<Box<dyn Scale>>,
}

impl ScaleSet {
    pub fn new() -> Self {
        ScaleSet { scales: Vec::new() }
    }

    /// Add a user-specified scale.
    pub fn add(&mut self, scale: Box<dyn Scale>) {
        // Replace existing scale for same aesthetic
        let aes = scale.aesthetic();
        self.scales.retain(|s| s.aesthetic() != aes);
        self.scales.push(scale);
    }

    /// Get a scale for a specific aesthetic.
    pub fn get(&self, aes: &Aesthetic) -> Option<&dyn Scale> {
        self.scales
            .iter()
            .find(|s| s.aesthetic() == *aes)
            .map(|s| s.as_ref())
    }

    /// Get mutable scale for a specific aesthetic.
    pub fn get_mut(&mut self, aes: &Aesthetic) -> Option<&mut Box<dyn Scale>> {
        self.scales.iter_mut().find(|s| s.aesthetic() == *aes)
    }

    /// Ensure a scale exists for a given aesthetic. Auto-detect type from data.
    pub fn ensure_scale(&mut self, aes: &Aesthetic, data: &DataFrame) {
        if self.get(aes).is_some() {
            return;
        }

        let col_name = aes.col_name();
        let values = data.column(col_name);

        let is_discrete = match values {
            Some(vals) => vals
                .iter()
                .any(|v| matches!(v, Value::Str(_) | Value::Bool(_))),
            None => false,
        };

        let is_datetime = match values {
            Some(vals) => vals.iter().any(|v| v.is_datetime()),
            None => false,
        };

        match aes {
            Aesthetic::Color | Aesthetic::Fill => {
                if is_discrete {
                    let scale = ScaleColorDiscrete::new(aes.clone());
                    self.scales.push(Box::new(scale));
                } else {
                    let scale = ScaleColorContinuous::new(aes.clone());
                    self.scales.push(Box::new(scale));
                }
            }
            Aesthetic::Shape => {
                let scale = ScaleShapeDiscrete::new();
                self.scales.push(Box::new(scale));
            }
            Aesthetic::Linetype => {
                let scale = ScaleLinetypeDiscrete::new();
                self.scales.push(Box::new(scale));
            }
            Aesthetic::Size => {
                let scale = ScaleSizeContinuous::new();
                self.scales.push(Box::new(scale));
            }
            Aesthetic::Alpha => {
                let scale = ScaleAlphaContinuous::new();
                self.scales.push(Box::new(scale));
            }
            _ => {
                if is_discrete {
                    let scale = ScaleDiscrete::new().for_aesthetic(aes.clone());
                    self.scales.push(Box::new(scale));
                } else if is_datetime {
                    let scale = ScaleDateTime::new().for_aesthetic(aes.clone());
                    self.scales.push(Box::new(scale));
                } else {
                    let scale = ScaleContinuous::new().for_aesthetic(aes.clone());
                    self.scales.push(Box::new(scale));
                }
            }
        }
    }

    /// Train all scales on data from one layer.
    pub fn train_layer(&mut self, data: &DataFrame) {
        for scale in &mut self.scales {
            let col_name = scale.aesthetic().col_name().to_string();
            if let Some(values) = data.column(&col_name) {
                scale.train(values);
            }
        }
    }

    /// Map a single value through the appropriate scale.
    pub fn map_value(&self, aes: &Aesthetic, value: &Value) -> f64 {
        self.get(aes).map(|s| s.map(value)).unwrap_or(0.0)
    }

    /// Map a value to an RGB color through the appropriate color/fill scale.
    pub fn map_color(&self, aes: &Aesthetic, value: &Value) -> Option<(u8, u8, u8)> {
        self.get(aes).and_then(|s| s.map_to_color(value))
    }

    /// Map a value to a point shape through the shape scale.
    pub fn map_shape(&self, value: &Value) -> Option<PointShape> {
        self.get(&Aesthetic::Shape)
            .and_then(|s| s.map_to_shape(value))
    }

    /// Map a value to a linetype through the linetype scale.
    pub fn map_linetype(&self, value: &Value) -> Option<Linetype> {
        self.get(&Aesthetic::Linetype)
            .and_then(|s| s.map_to_linetype(value))
    }

    /// Map a value to a point size through the size scale.
    pub fn map_size(&self, value: &Value) -> Option<f64> {
        self.get(&Aesthetic::Size)
            .and_then(|s| s.map_to_size(value))
    }

    /// Map a value to an alpha (opacity) through the alpha scale.
    pub fn map_alpha(&self, value: &Value) -> Option<f64> {
        self.get(&Aesthetic::Alpha)
            .and_then(|s| s.map_to_alpha(value))
    }

    /// Override the domain limits for a scale (used by coord_cartesian zoom).
    pub fn set_limits(&mut self, aes: &Aesthetic, min: f64, max: f64) {
        if let Some(scale) = self.get_mut(aes) {
            scale.set_limits(min, max);
        }
    }

    /// Get the secondary axis for an aesthetic, if one exists.
    pub fn sec_axis(&self, aes: &Aesthetic) -> Option<&crate::scale::sec_axis::SecAxis> {
        self.get(aes).and_then(|s| s.sec_axis())
    }

    /// Get all scales.
    pub fn iter(&self) -> impl Iterator<Item = &dyn Scale> {
        self.scales.iter().map(|s| s.as_ref())
    }
}

impl Default for ScaleSet {
    fn default() -> Self {
        Self::new()
    }
}
