use crate::aes::Aesthetic;
use crate::data::Value;

use super::util::format_number;
use super::Scale;

/// Continuous alpha scale — maps data values to an opacity range.
#[derive(Clone, Debug)]
pub struct ScaleAlphaContinuous {
    name: String,
    min: f64,
    max: f64,
    trained: bool,
    /// Output range: (min_alpha, max_alpha) in [0, 1].
    range: (f64, f64),
}

impl ScaleAlphaContinuous {
    pub fn new() -> Self {
        ScaleAlphaContinuous {
            name: String::new(),
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
            trained: false,
            range: (0.1, 1.0),
        }
    }

    pub fn with_range(mut self, min: f64, max: f64) -> Self {
        self.range = (min, max);
        self
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }
}

impl Default for ScaleAlphaContinuous {
    fn default() -> Self {
        Self::new()
    }
}

impl Scale for ScaleAlphaContinuous {
    fn aesthetic(&self) -> Aesthetic {
        Aesthetic::Alpha
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
        let range = self.max - self.min;
        if range.abs() < f64::EPSILON {
            0.5
        } else {
            ((f - self.min) / range).clamp(0.0, 1.0)
        }
    }

    fn breaks(&self) -> Vec<(f64, String)> {
        if !self.trained || self.min > self.max {
            return vec![];
        }
        let range = self.max - self.min;
        if range.abs() < f64::EPSILON {
            return vec![(0.5, format_number(self.min))];
        }
        let n_breaks = 4;
        let step = super::util::nice_step(range / n_breaks as f64);
        let start = (self.min / step).ceil() * step;
        let mut breaks = Vec::new();
        let mut v = start;
        while v <= self.max + step * 0.001 {
            let pos = self.map(&Value::Float(v));
            breaks.push((pos, format_number(v)));
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

    fn map_to_alpha(&self, value: &Value) -> Option<f64> {
        let t = self.map(value);
        let (lo, hi) = self.range;
        Some(lo + t * (hi - lo))
    }

    fn clone_box(&self) -> Box<dyn Scale> {
        Box::new(self.clone())
    }

    fn reset_training(&mut self) {
        self.min = f64::INFINITY;
        self.max = f64::NEG_INFINITY;
        self.trained = false;
    }
}
