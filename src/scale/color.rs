use crate::aes::Aesthetic;
use crate::data::Value;
use std::collections::HashMap;

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
    /// level → index in `levels`, so training/lookup are O(1) (many-category
    /// fills would otherwise be quadratic). Kept in sync with `levels`.
    level_index: HashMap<String, usize>,
    palette: Vec<RGBAColor>,
    /// Keep levels in sorted (lexicographic) order instead of first-seen
    /// order — see [`ScaleColorDiscrete::sorted`].
    sorted: bool,
}

impl ScaleColorDiscrete {
    pub fn new(aesthetic: Aesthetic) -> Self {
        ScaleColorDiscrete {
            aesthetic,
            name: String::new(),
            levels: Vec::new(),
            level_index: HashMap::new(),
            palette: DEFAULT_PALETTE.to_vec(),
            sorted: false,
        }
    }

    /// Order levels lexicographically (instead of first appearance) so a given
    /// level always gets the same palette colour and legend position across
    /// charts, whatever order the data arrives in — no need to pre-sort levels
    /// and repeat a manual palette per chart.
    pub fn sorted(mut self) -> Self {
        self.sorted = true;
        self.resort();
        self
    }

    fn resort(&mut self) {
        if !self.sorted {
            return;
        }
        self.levels.sort();
        self.level_index = self
            .levels
            .iter()
            .enumerate()
            .map(|(i, l)| (l.clone(), i))
            .collect();
    }

    pub fn with_palette(mut self, colors: Vec<RGBAColor>) -> Self {
        self.palette = colors;
        self
    }

    pub fn with_named_palette(mut self, name: &super::palettes::PaletteName) -> Self {
        self.palette = super::palettes::palette(name).to_vec();
        self
    }

    /// Pre-seed the factor levels (order), so each level keeps a fixed palette
    /// color regardless of which levels are present in the data — e.g. for a
    /// legend whose series can be toggled without the colors reshuffling.
    pub fn with_levels(mut self, levels: Vec<String>) -> Self {
        self.levels.clear();
        self.level_index.clear();
        for l in levels {
            self.push_level(l);
        }
        self.resort();
        self
    }

    fn push_level(&mut self, key: String) {
        if !self.level_index.contains_key(&key) {
            self.level_index.insert(key.clone(), self.levels.len());
            self.levels.push(key);
        }
    }

    fn level_position(&self, key: &str) -> Option<usize> {
        self.level_index.get(key).copied()
    }

    /// Get color for a given level index.
    pub fn color_for_index(&self, idx: usize) -> RGBAColor {
        if self.palette.is_empty() {
            return DEFAULT_PALETTE[idx % DEFAULT_PALETTE.len()];
        }
        self.palette[idx % self.palette.len()]
    }

    /// Get color for a value.
    pub fn color_for_value(&self, value: &Value) -> RGBAColor {
        let key = value.to_group_key();
        let idx = self.level_position(&key).unwrap_or(0);
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
        let before = self.levels.len();
        for v in values {
            let key = v.to_group_key();
            self.push_level(key);
        }
        if self.levels.len() != before {
            self.resort();
        }
    }

    fn map(&self, value: &Value) -> f64 {
        let key = value.to_group_key();
        self.level_position(&key).map(|i| i as f64).unwrap_or(0.0)
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

    fn clone_box(&self) -> Box<dyn Scale> {
        Box::new(self.clone())
    }

    fn reset_training(&mut self) {
        self.levels.clear();
        self.level_index.clear();
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
        if super::util::is_degenerate_range(self.min, self.max) {
            return vec![(0.5, format_number(self.min))];
        }

        let n_breaks = 5;
        let raw_step = range / n_breaks as f64;
        let step = nice_step(raw_step);

        let start = (self.min / step).ceil() * step;
        super::util::stepped_breaks(start, self.max, step)
            .into_iter()
            .map(|v| (self.map(&Value::Float(v)), format_number(v)))
            .collect()
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
