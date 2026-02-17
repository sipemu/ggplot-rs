use crate::aes::Aesthetic;
use crate::data::Value;

use super::sec_axis::SecAxis;
use super::transform::ScaleTransform;
use super::util::{format_number, nice_step};
use super::Scale;

/// Continuous linear scale.
#[derive(Clone, Debug)]
pub struct ScaleContinuous {
    aesthetic: Aesthetic,
    name: String,
    min: f64,
    max: f64,
    trained: bool,
    expand: (f64, f64), // multiplicative and additive expansion
    pub(crate) scale_transform: ScaleTransform,
    custom_breaks: Option<Vec<f64>>,
    custom_labels: Option<Vec<String>>,
    pub(crate) sec_axis: Option<SecAxis>,
}

impl ScaleContinuous {
    pub fn new() -> Self {
        ScaleContinuous {
            aesthetic: Aesthetic::X,
            name: String::new(),
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
            trained: false,
            expand: (0.05, 0.0),
            scale_transform: ScaleTransform::Identity,
            custom_breaks: None,
            custom_labels: None,
            sec_axis: None,
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

    pub fn with_limits(mut self, min: f64, max: f64) -> Self {
        self.min = min;
        self.max = max;
        self.trained = true;
        self
    }

    pub fn with_transform(mut self, transform: ScaleTransform) -> Self {
        self.scale_transform = transform;
        self
    }

    /// Set custom break positions (data values where ticks appear).
    pub fn with_breaks(mut self, breaks: Vec<f64>) -> Self {
        self.custom_breaks = Some(breaks);
        self
    }

    /// Set custom labels for breaks. Must match the number of breaks.
    pub fn with_labels(mut self, labels: Vec<String>) -> Self {
        self.custom_labels = Some(labels);
        self
    }

    /// Add a secondary axis with a transformation function.
    pub fn with_sec_axis(mut self, sec: SecAxis) -> Self {
        self.sec_axis = Some(sec);
        self
    }

    /// Get the secondary axis, if any.
    pub fn sec_axis(&self) -> Option<&SecAxis> {
        self.sec_axis.as_ref()
    }

    fn expanded_range(&self) -> (f64, f64) {
        let range = self.max - self.min;
        let mult = self.expand.0;
        let add = self.expand.1;
        (self.min - range * mult - add, self.max + range * mult + add)
    }
}

impl Default for ScaleContinuous {
    fn default() -> Self {
        Self::new()
    }
}

impl Scale for ScaleContinuous {
    fn aesthetic(&self) -> Aesthetic {
        self.aesthetic.clone()
    }

    fn train(&mut self, values: &[Value]) {
        for v in values {
            if let Some(f) = v.as_f64() {
                if f.is_finite() {
                    if f < self.min {
                        self.min = f;
                    }
                    if f > self.max {
                        self.max = f;
                    }
                }
            }
        }
        self.trained = true;
    }

    fn map(&self, value: &Value) -> f64 {
        let f = match value.as_f64() {
            Some(f) => f,
            None => return 0.0,
        };
        let (emin, emax) = self.expanded_range();
        let range = emax - emin;
        if range.abs() < f64::EPSILON {
            0.5
        } else {
            (f - emin) / range
        }
    }

    fn breaks(&self) -> Vec<(f64, String)> {
        if !self.trained || self.min > self.max {
            return vec![];
        }

        // Use custom breaks if provided
        if let Some(ref custom) = self.custom_breaks {
            return custom
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    let pos = self.map(&Value::Float(v));
                    let label = if let Some(ref labels) = self.custom_labels {
                        labels.get(i).cloned().unwrap_or_else(|| format_number(v))
                    } else {
                        format_number(self.scale_transform.inverse(v))
                    };
                    (pos, label)
                })
                .collect();
        }

        let range = self.max - self.min;
        if range.abs() < f64::EPSILON {
            let label = format_number(self.scale_transform.inverse(self.min));
            return vec![(0.5, label)];
        }

        // Generate nice breaks across the expanded (visible) range
        let (emin, emax) = self.expanded_range();
        let n_breaks = 5;
        let raw_step = range / n_breaks as f64;
        let step = nice_step(raw_step);

        let start = (emin / step).ceil() * step;
        let mut breaks = Vec::new();
        let mut v = start;
        while v <= emax + step * 0.001 {
            let pos = self.map(&Value::Float(v));
            // Labels show the original (inverse-transformed) value
            let label = format_number(self.scale_transform.inverse(v));
            breaks.push((pos, label));
            v += step;
        }
        breaks
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    fn transform(&self, value: &Value) -> Value {
        self.scale_transform.transform_value(value)
    }

    fn sec_axis(&self) -> Option<&SecAxis> {
        self.sec_axis.as_ref()
    }

    fn set_limits(&mut self, min: f64, max: f64) {
        self.min = min;
        self.max = max;
        self.trained = true;
    }
}
