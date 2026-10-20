use crate::aes::Aesthetic;
use crate::data::Value;

use super::Scale;

/// Continuous size scale — maps data values to a point radius range.
#[derive(Clone, Debug)]
pub struct ScaleSizeContinuous {
    name: String,
    min: f64,
    max: f64,
    trained: bool,
    /// Output range in pixels: (min_radius, max_radius).
    range: (f64, f64),
    /// Locale for the default break labels (`None` = default marks).
    locale: Option<crate::locale::Locale>,
    /// `scale_size_area(max_size)` (B3-6): private, set only via
    /// [`area`](Self::area). `None` (the default) keeps the linear radius
    /// mapping (`range`); `Some(max_size)` maps so *area* — not radius —
    /// is proportional to the value, with 0 mapping to 0.
    area_max: Option<f64>,
}

impl ScaleSizeContinuous {
    pub fn new() -> Self {
        ScaleSizeContinuous {
            name: String::new(),
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
            trained: false,
            range: (1.0, 6.0),
            locale: None,
            area_max: None,
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

    /// `scale_size_area(max_size)` (ggplot2): maps so that *area* — not
    /// radius — is proportional to `|value|`, with 0 mapping to 0:
    /// `size = max_size · √(|value| / max(|lo|, |hi|))`, where `lo`/`hi`
    /// are the trained domain. A non-finite or non-positive `max_size` is
    /// corrected to the ggplot2 default (6) by [`validate`](Self::validate)
    /// and by the mapping itself.
    pub fn area(mut self, max_size: f64) -> Self {
        self.area_max = Some(max_size);
        self
    }

    fn sanitized_area_max(max_size: f64) -> f64 {
        if max_size.is_finite() && max_size > 0.0 {
            max_size
        } else {
            6.0
        }
    }
}

impl Default for ScaleSizeContinuous {
    fn default() -> Self {
        Self::new()
    }
}

impl Scale for ScaleSizeContinuous {
    fn set_default_locale(&mut self, locale: &crate::locale::Locale) {
        if self.locale.is_none() {
            self.locale = Some(locale.clone());
        }
    }

    fn locale(&self) -> Option<&crate::locale::Locale> {
        self.locale.as_ref()
    }

    fn format_value(&self, value: &Value) -> Option<String> {
        let f = value.as_f64()?;
        if !f.is_finite() {
            return None;
        }
        Some(super::util::format_number_opt(f, self.locale.as_ref()))
    }

    fn aesthetic(&self) -> Aesthetic {
        Aesthetic::Size
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
        if super::util::is_degenerate_range(self.min, self.max) {
            return vec![(
                0.5,
                super::util::format_number_opt(self.min, self.locale.as_ref()),
            )];
        }
        let n_breaks = 4;
        let step = super::util::nice_step(range / n_breaks as f64);
        let start = (self.min / step).ceil() * step;
        let values = super::util::stepped_breaks(start, self.max, step);
        let labels = super::util::format_numbers_opt(&values, self.locale.as_ref());
        values
            .iter()
            .zip(labels)
            .map(|(&v, l)| (self.map(&Value::Float(v)), l))
            .collect()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    fn map_to_size(&self, value: &Value) -> Option<f64> {
        if let Some(max_size) = self.area_max {
            let v = value.as_f64()?;
            if !v.is_finite() {
                return None;
            }
            let max_size = Self::sanitized_area_max(max_size);
            let denom = self.min.abs().max(self.max.abs());
            return Some(if denom < f64::EPSILON {
                0.0
            } else {
                max_size * (v.abs() / denom).sqrt()
            });
        }
        let t = self.map(value);
        let (lo, hi) = self.range;
        Some(lo + t * (hi - lo))
    }

    fn validate(&self, warnings: &mut Vec<String>) {
        if let Some(max_size) = self.area_max {
            if !(max_size.is_finite() && max_size > 0.0) {
                warnings.push(format!(
                    "scale_size_area: max_size({max_size}) must be finite and positive; falling back to 6"
                ));
            }
        }
    }

    fn map_fraction_to_size(&self, fraction: f64) -> Option<f64> {
        let (lo, hi) = self.range;
        Some(lo + fraction.clamp(0.0, 1.0) * (hi - lo))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_value_formats_a_finite_number_and_rejects_non_finite() {
        let s = ScaleSizeContinuous::new();
        assert_eq!(
            s.format_value(&Value::Float(1234.0)),
            Some("1234".to_string())
        );
        assert_eq!(s.format_value(&Value::Float(f64::NAN)), None);
        assert_eq!(s.format_value(&Value::Na), None);
    }

    #[test]
    fn scale_size_area_matches_r_ggplot2() {
        let mut s = ScaleSizeContinuous::new().area(6.0);
        s.train(&[
            Value::Float(0.0),
            Value::Float(1.0),
            Value::Float(4.0),
            Value::Float(16.0),
        ]);
        let got: Vec<f64> = [0.0, 1.0, 4.0, 16.0]
            .iter()
            .map(|&v| s.map_to_size(&Value::Float(v)).unwrap())
            .collect();
        for (g, e) in got.iter().zip([0.0, 1.5, 3.0, 6.0]) {
            assert!((g - e).abs() < 1e-9, "got {got:?}");
        }
    }

    #[test]
    fn scale_size_area_rejects_na_and_non_finite() {
        let mut s = ScaleSizeContinuous::new().area(6.0);
        s.train(&[Value::Float(0.0), Value::Float(10.0)]);
        assert_eq!(s.map_to_size(&Value::Na), None);
        assert_eq!(s.map_to_size(&Value::Float(f64::NAN)), None);
    }

    #[test]
    fn scale_size_area_bad_max_warns_and_falls_back_to_six() {
        let mut s = ScaleSizeContinuous::new().area(-1.0);
        s.train(&[Value::Float(0.0), Value::Float(16.0)]);
        let mut warnings = Vec::new();
        s.validate(&mut warnings);
        assert!(warnings.iter().any(|w| w.contains("max_size")));
        // Falls back to the ggplot2 default (6) regardless of `validate`
        // being called — `map_to_size` guards it inline too.
        assert_eq!(
            s.map_to_size(&Value::Float(16.0)),
            Some(6.0),
            "falls back to max_size 6"
        );
    }
}
