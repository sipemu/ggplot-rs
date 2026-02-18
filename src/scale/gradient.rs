use crate::aes::Aesthetic;
use crate::data::Value;

use super::color::RGBAColor;
use super::util::{format_number, nice_step};
use super::Scale;

/// Diverging gradient color scale (low → mid → high with midpoint).
#[derive(Clone, Debug)]
pub struct ScaleColorGradient2 {
    aesthetic: Aesthetic,
    name: String,
    low: RGBAColor,
    mid: RGBAColor,
    high: RGBAColor,
    midpoint: f64,
    min: f64,
    max: f64,
}

impl ScaleColorGradient2 {
    pub fn new(aesthetic: Aesthetic) -> Self {
        ScaleColorGradient2 {
            aesthetic,
            name: String::new(),
            low: RGBAColor::new(0, 0, 255),
            mid: RGBAColor::new(255, 255, 255),
            high: RGBAColor::new(255, 0, 0),
            midpoint: 0.0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        }
    }

    pub fn with_colors(mut self, low: RGBAColor, mid: RGBAColor, high: RGBAColor) -> Self {
        self.low = low;
        self.mid = mid;
        self.high = high;
        self
    }

    pub fn with_midpoint(mut self, midpoint: f64) -> Self {
        self.midpoint = midpoint;
        self
    }
}

impl Scale for ScaleColorGradient2 {
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
            (f - self.min) / range
        }
    }

    fn breaks(&self) -> Vec<(f64, String)> {
        if self.min > self.max || !self.min.is_finite() || !self.max.is_finite() {
            return vec![];
        }
        let range = self.max - self.min;
        if range.abs() < f64::EPSILON {
            return vec![(0.5, format_number(self.min))];
        }
        let n_breaks = 5;
        let raw_step = range / n_breaks as f64;
        let step = nice_step(raw_step);
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

    fn map_to_color(&self, value: &Value) -> Option<(u8, u8, u8)> {
        let f = value.as_f64()?;

        // Map relative to midpoint
        let c = if f <= self.midpoint {
            let range = self.midpoint - self.min;
            let t = if range.abs() < f64::EPSILON {
                0.0
            } else {
                (f - self.min) / range
            };
            self.low.lerp(&self.mid, t)
        } else {
            let range = self.max - self.midpoint;
            let t = if range.abs() < f64::EPSILON {
                1.0
            } else {
                (f - self.midpoint) / range
            };
            self.mid.lerp(&self.high, t)
        };

        Some((c.r, c.g, c.b))
    }

    fn domain(&self) -> Option<(f64, f64)> {
        if self.min.is_finite() && self.max.is_finite() && self.min <= self.max {
            Some((self.min, self.max))
        } else {
            None
        }
    }

    fn clone_box(&self) -> Box<dyn Scale> {
        Box::new(self.clone())
    }

    fn reset_training(&mut self) {
        self.min = f64::INFINITY;
        self.max = f64::NEG_INFINITY;
    }
}
