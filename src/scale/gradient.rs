use crate::aes::Aesthetic;
use crate::data::Value;

use super::color::RGBAColor;
use super::util::nice_step;
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
    /// Locale for the default break labels (`None` = default marks).
    locale: Option<crate::locale::Locale>,
    /// Fixed `(lo, hi)` domain ([`with_limits`](Self::with_limits)):
    /// `train` no longer widens it, and `map_to_color` maps against it
    /// instead of the trained min/max.
    limits: Option<(f64, f64)>,
    /// Out-of-bounds policy for `limits` ([`with_oob`](Self::with_oob)).
    /// Default [`Oob::Censor`](super::Oob::Censor).
    oob: super::Oob,
    /// Colour for a genuinely missing or out-of-bounds-and-censored value
    /// ([`with_na_value`](Self::with_na_value)). Default ggplot2's
    /// `"grey50"`, `(127, 127, 127)`.
    na_value: (u8, u8, u8),
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
            locale: None,
            limits: None,
            oob: super::Oob::Censor,
            na_value: super::DEFAULT_NA_VALUE,
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

    /// Fix the colour domain to `[lo, hi]` instead of whatever the data
    /// happens to span. Out-of-range values follow
    /// [`with_oob`](Self::with_oob) (default: `na_value`).
    pub fn with_limits(mut self, lo: f64, hi: f64) -> Self {
        self.limits = Some((lo, hi));
        self
    }

    /// Out-of-bounds policy for [`with_limits`](Self::with_limits):
    /// [`Oob::Censor`](super::Oob::Censor) (default) or
    /// [`Oob::Squish`](super::Oob::Squish).
    pub fn with_oob(mut self, oob: super::Oob) -> Self {
        self.oob = oob;
        self
    }

    /// Colour for a genuinely missing (`NA`) value, or one censored by
    /// `with_limits`'s default [`Oob::Censor`]. Default ggplot2's
    /// `"grey50"`, `(127, 127, 127)`.
    pub fn with_na_value(mut self, color: (u8, u8, u8)) -> Self {
        self.na_value = color;
        self
    }
}

impl Scale for ScaleColorGradient2 {
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
        self.aesthetic.clone()
    }

    fn train(&mut self, values: &[Value]) {
        if self.limits.is_some() {
            return;
        }
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
        let (lo, hi) = self.limits.unwrap_or((self.min, self.max));
        let range = hi - lo;
        if range.abs() < f64::EPSILON {
            0.5
        } else {
            (f - lo) / range
        }
    }

    fn breaks(&self) -> Vec<(f64, String)> {
        let (lo, hi) = self.limits.unwrap_or((self.min, self.max));
        if lo > hi || !lo.is_finite() || !hi.is_finite() {
            return vec![];
        }
        let range = hi - lo;
        if super::util::is_degenerate_range(lo, hi) {
            return vec![(
                0.5,
                super::util::format_number_opt(lo, self.locale.as_ref()),
            )];
        }
        let n_breaks = 5;
        let raw_step = range / n_breaks as f64;
        let step = nice_step(raw_step);
        let start = (lo / step).ceil() * step;
        let values = super::util::stepped_breaks(start, hi, step);
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

    fn map_to_color(&self, value: &Value) -> Option<(u8, u8, u8)> {
        // A genuinely missing, non-numeric or non-finite value, and a
        // value censored by `with_limits`, all render as `na_value`
        // (ggplot2's own behaviour) instead of a plausible-looking colour
        // or a bare `None` the geom silently papers over with its own
        // unrelated default (review PR#141 finding 3).
        if value.is_na() {
            return Some(self.na_value);
        }
        let Some(f) = value.as_f64() else {
            return Some(self.na_value);
        };
        if !f.is_finite() {
            return Some(self.na_value);
        }
        let (lo, hi) = self.limits.unwrap_or((self.min, self.max));
        let Some(f) = super::resolve_oob(f, self.limits, self.oob) else {
            return Some(self.na_value);
        };

        // Map relative to midpoint
        let c = if f <= self.midpoint {
            let range = self.midpoint - lo;
            let t = if range.abs() < f64::EPSILON {
                0.0
            } else {
                (f - lo) / range
            };
            self.low.lerp(&self.mid, t.clamp(0.0, 1.0))
        } else {
            let range = hi - self.midpoint;
            let t = if range.abs() < f64::EPSILON {
                1.0
            } else {
                (f - self.midpoint) / range
            };
            self.mid.lerp(&self.high, t.clamp(0.0, 1.0))
        };

        Some((c.r, c.g, c.b))
    }

    fn domain(&self) -> Option<(f64, f64)> {
        if let Some((lo, hi)) = self.limits {
            return Some((lo, hi));
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_value_formats_a_finite_number_and_rejects_non_finite() {
        let s = ScaleColorGradient2::new(Aesthetic::Color);
        assert_eq!(s.format_value(&Value::Float(7.0)), Some("7".to_string()));
        assert_eq!(s.format_value(&Value::Float(f64::NEG_INFINITY)), None);
    }

    #[test]
    fn with_limits_fixes_the_domain_and_training_does_not_widen_it() {
        let mut s = ScaleColorGradient2::new(Aesthetic::Fill).with_limits(-2.0, 2.0);
        s.train(&[Value::Float(-50.0), Value::Float(50.0)]);
        assert_eq!(s.domain(), Some((-2.0, 2.0)));
    }
}
