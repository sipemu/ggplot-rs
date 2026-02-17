use crate::aes::Aesthetic;
use crate::data::Value;

use super::util::{format_number, nice_step};
use super::Scale;

/// RGBA color representation.
#[derive(Clone, Debug, Copy)]
pub struct RGBAColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f64,
}

impl RGBAColor {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        RGBAColor { r, g, b, a: 1.0 }
    }

    pub fn with_alpha(mut self, a: f64) -> Self {
        self.a = a;
        self
    }

    /// Interpolate between two colors.
    pub fn lerp(&self, other: &RGBAColor, t: f64) -> RGBAColor {
        let t = t.clamp(0.0, 1.0);
        RGBAColor {
            r: (self.r as f64 * (1.0 - t) + other.r as f64 * t) as u8,
            g: (self.g as f64 * (1.0 - t) + other.g as f64 * t) as u8,
            b: (self.b as f64 * (1.0 - t) + other.b as f64 * t) as u8,
            a: self.a * (1.0 - t) + other.a * t,
        }
    }
}

/// Default discrete color palette (8 colors, similar to ggplot2 default).
pub const DEFAULT_PALETTE: &[RGBAColor] = &[
    RGBAColor {
        r: 248,
        g: 118,
        b: 109,
        a: 1.0,
    }, // red
    RGBAColor {
        r: 0,
        g: 186,
        b: 56,
        a: 1.0,
    }, // green
    RGBAColor {
        r: 97,
        g: 156,
        b: 255,
        a: 1.0,
    }, // blue
    RGBAColor {
        r: 163,
        g: 103,
        b: 203,
        a: 1.0,
    }, // purple
    RGBAColor {
        r: 231,
        g: 138,
        b: 0,
        a: 1.0,
    }, // orange
    RGBAColor {
        r: 0,
        g: 191,
        b: 196,
        a: 1.0,
    }, // cyan
    RGBAColor {
        r: 199,
        g: 124,
        b: 255,
        a: 1.0,
    }, // violet
    RGBAColor {
        r: 127,
        g: 127,
        b: 127,
        a: 1.0,
    }, // gray
];

/// Discrete color scale — maps categories to distinct colors.
#[derive(Clone, Debug)]
pub struct ScaleColorDiscrete {
    aesthetic: Aesthetic,
    name: String,
    levels: Vec<String>,
    palette: Vec<RGBAColor>,
}

impl ScaleColorDiscrete {
    pub fn new(aesthetic: Aesthetic) -> Self {
        ScaleColorDiscrete {
            aesthetic,
            name: String::new(),
            levels: Vec::new(),
            palette: DEFAULT_PALETTE.to_vec(),
        }
    }

    pub fn with_palette(mut self, colors: Vec<RGBAColor>) -> Self {
        self.palette = colors;
        self
    }

    pub fn with_named_palette(mut self, name: &super::palettes::PaletteName) -> Self {
        self.palette = super::palettes::palette(name).to_vec();
        self
    }

    /// Get color for a given level index.
    pub fn color_for_index(&self, idx: usize) -> RGBAColor {
        self.palette[idx % self.palette.len()]
    }

    /// Get color for a value.
    pub fn color_for_value(&self, value: &Value) -> RGBAColor {
        let key = value.to_group_key();
        let idx = self.levels.iter().position(|l| l == &key).unwrap_or(0);
        self.color_for_index(idx)
    }

    pub fn levels(&self) -> &[String] {
        &self.levels
    }
}

impl Scale for ScaleColorDiscrete {
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
        let c = self.color_for_value(value);
        Some((c.r, c.g, c.b))
    }
}

/// Continuous gradient color scale.
#[derive(Clone, Debug)]
pub struct ScaleColorContinuous {
    aesthetic: Aesthetic,
    name: String,
    low: RGBAColor,
    high: RGBAColor,
    min: f64,
    max: f64,
}

impl ScaleColorContinuous {
    pub fn new(aesthetic: Aesthetic) -> Self {
        ScaleColorContinuous {
            aesthetic,
            name: String::new(),
            low: RGBAColor::new(0, 0, 255),
            high: RGBAColor::new(255, 0, 0),
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        }
    }

    pub fn with_colors(mut self, low: RGBAColor, high: RGBAColor) -> Self {
        self.low = low;
        self.high = high;
        self
    }

    pub fn color_at(&self, t: f64) -> RGBAColor {
        self.low.lerp(&self.high, t)
    }
}

impl Scale for ScaleColorContinuous {
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
        let t = self.map(value);
        let c = self.color_at(t);
        Some((c.r, c.g, c.b))
    }
}
