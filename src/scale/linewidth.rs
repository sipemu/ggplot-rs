use std::sync::Arc;

use crate::aes::Aesthetic;
use crate::data::Value;

use super::format::LabelFormatter;
use super::transform::ScaleTransform;
use super::Scale;

/// Continuous linewidth scale — maps data values to a stroke-width range
/// (R's `scale_linewidth`). Unlike the position (x/y) scales, `with_limits`
/// squishes out-of-range values to the nearest limit rather than filtering
/// them, and `with_transform` reshapes the normalized `[0, 1]` fraction
/// inside [`map`](Scale::map) / [`map_to_size`](Scale::map_to_size) — it does
/// *not* override [`Scale::transform`] (that's for pre-stat domain
/// transforms like `scale_x_log10`, which linewidth has no use for).
#[derive(Clone)]
pub struct ScaleLinewidthContinuous {
    name: String,
    min: f64,
    max: f64,
    trained: bool,
    /// Set by [`with_limits`](Self::with_limits): the domain is fixed and no
    /// longer widened by [`train`](Scale::train) (ggplot2's non-position
    /// scales squish to a fixed domain, unlike x/y's OOB filtering).
    limits_fixed: bool,
    /// Output range in pixels: (min_width, max_width).
    range: (f64, f64),
    scale_transform: ScaleTransform,
    custom_breaks: Option<Vec<f64>>,
    custom_labels: Option<Vec<String>>,
    label_formatter: Option<LabelFormatter>,
    /// Locale for the default break labels (`None` = default marks).
    locale: Option<crate::locale::Locale>,
}

impl ScaleLinewidthContinuous {
    pub fn new() -> Self {
        ScaleLinewidthContinuous {
            name: String::new(),
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
            trained: false,
            limits_fixed: false,
            range: (1.0, 6.0),
            scale_transform: ScaleTransform::Identity,
            custom_breaks: None,
            custom_labels: None,
            label_formatter: None,
            locale: None,
        }
    }

    /// Output range in pixels. Default `(1.0, 6.0)`.
    pub fn with_range(mut self, min: f64, max: f64) -> Self {
        self.range = (min, max);
        self
    }

    /// Fix the input domain; values outside `[min, max]` are squished to the
    /// nearest bound rather than filtered.
    pub fn with_limits(mut self, min: f64, max: f64) -> Self {
        self.min = min;
        self.max = max;
        self.trained = true;
        self.limits_fixed = true;
        self
    }

    /// Reshape the normalized `[0, 1]` fraction before it is scaled into
    /// `range` — e.g. `Sqrt` so stroke width scales with the square root of
    /// the data value (closer to equal visual "ink" per unit, like
    /// ggplot2's default `scale_size`/`scale_linewidth` area transform).
    pub fn with_transform(mut self, transform: ScaleTransform) -> Self {
        self.scale_transform = transform;
        self
    }

    /// Set custom break positions (data values where legend keys appear).
    pub fn with_breaks(mut self, breaks: Vec<f64>) -> Self {
        self.custom_breaks = Some(breaks);
        self
    }

    /// Set custom labels for breaks. Must match the number of breaks.
    pub fn with_labels(mut self, labels: Vec<String>) -> Self {
        self.custom_labels = Some(labels);
        self
    }

    /// Set a label formatter. Accepts a plain `fn` (e.g. `label_comma`) or a
    /// configurable formatter such as `label_si()` / `label_number(...)`.
    pub fn with_label_formatter<F>(mut self, f: F) -> Self
    where
        F: Fn(f64) -> String + Send + Sync + 'static,
    {
        self.label_formatter = Some(Arc::new(f));
        self
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// Render the default break labels with `locale`'s decimal and
    /// thousands marks. A custom [`with_label_formatter`](Self::with_label_formatter)
    /// still wins.
    pub fn with_locale(mut self, locale: crate::locale::Locale) -> Self {
        self.locale = Some(locale);
        self
    }

    fn format_label(&self, v: f64) -> String {
        if let Some(f) = &self.label_formatter {
            f(v)
        } else {
            super::util::format_number_opt(v, self.locale.as_ref())
        }
    }

    /// Normalized fraction in `[0, 1]`: squished by the (possibly fixed)
    /// domain, then reshaped by `scale_transform`.
    fn fraction(&self, value: &Value) -> f64 {
        let f = match value.as_f64() {
            Some(f) => f,
            None => return 0.0,
        };
        if !self.trained || !self.min.is_finite() || !self.max.is_finite() {
            return 0.5;
        }
        let range = self.max - self.min;
        let t = if range.abs() < f64::EPSILON {
            0.5
        } else {
            ((f - self.min) / range).clamp(0.0, 1.0)
        };
        self.scale_transform.apply(t).clamp(0.0, 1.0)
    }
}

impl Default for ScaleLinewidthContinuous {
    fn default() -> Self {
        Self::new()
    }
}

impl Scale for ScaleLinewidthContinuous {
    fn set_default_locale(&mut self, locale: &crate::locale::Locale) {
        if self.locale.is_none() {
            self.locale = Some(locale.clone());
        }
    }

    fn locale(&self) -> Option<&crate::locale::Locale> {
        self.locale.as_ref()
    }

    fn aesthetic(&self) -> Aesthetic {
        Aesthetic::Linewidth
    }

    fn train(&mut self, values: &[Value]) {
        if self.limits_fixed {
            self.trained = true;
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
        self.trained = true;
    }

    fn map(&self, value: &Value) -> f64 {
        self.fraction(value)
    }

    fn breaks(&self) -> Vec<(f64, String)> {
        if !self.trained || self.min > self.max {
            return vec![];
        }
        if let Some(ref custom) = self.custom_breaks {
            return custom
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    let pos = self.map(&Value::Float(v));
                    let label = self
                        .custom_labels
                        .as_ref()
                        .and_then(|labels| labels.get(i).cloned())
                        .unwrap_or_else(|| self.format_label(v));
                    (pos, label)
                })
                .collect();
        }
        if super::util::is_degenerate_range(self.min, self.max) {
            return vec![(0.5, self.format_label(self.min))];
        }
        let n_breaks = 4;
        let range = self.max - self.min;
        let step = super::util::nice_step(range / n_breaks as f64);
        let start = (self.min / step).ceil() * step;
        let values = super::util::stepped_breaks(start, self.max, step);
        values
            .iter()
            .map(|&v| (self.map(&Value::Float(v)), self.format_label(v)))
            .collect()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    fn map_to_size(&self, value: &Value) -> Option<f64> {
        let t = self.fraction(value);
        let (lo, hi) = self.range;
        Some(lo + t * (hi - lo))
    }

    fn map_fraction_to_size(&self, fraction: f64) -> Option<f64> {
        let (lo, hi) = self.range;
        Some(lo + fraction.clamp(0.0, 1.0) * (hi - lo))
    }

    fn domain(&self) -> Option<(f64, f64)> {
        if self.trained {
            Some((self.min, self.max))
        } else {
            None
        }
    }

    fn clone_box(&self) -> Box<dyn Scale> {
        Box::new(self.clone())
    }

    fn reset_training(&mut self) {
        if !self.limits_fixed {
            self.min = f64::INFINITY;
            self.max = f64::NEG_INFINITY;
            self.trained = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_transform_and_limits_match_the_documented_formula() {
        let vmax = 10.0;
        let scale = ScaleLinewidthContinuous::new()
            .with_range(0.75, 8.0)
            .with_transform(ScaleTransform::Sqrt)
            .with_limits(0.0, vmax);
        for v in [0.0, 2.5, 5.0, 7.5, 10.0] {
            let width = scale.map_to_size(&Value::Float(v)).unwrap();
            let expected = 0.75 + 7.25 * (v / vmax).sqrt();
            assert!(
                (width - expected).abs() < 1e-9,
                "v={v}: width={width}, expected={expected}"
            );
        }
    }

    #[test]
    fn limits_squish_out_of_range_values() {
        let scale = ScaleLinewidthContinuous::new()
            .with_range(1.0, 6.0)
            .with_limits(0.0, 10.0);
        assert_eq!(scale.map_to_size(&Value::Float(-5.0)), Some(1.0));
        assert_eq!(scale.map_to_size(&Value::Float(50.0)), Some(6.0));
    }

    #[test]
    fn limits_fixed_domain_is_not_widened_by_training() {
        let mut scale = ScaleLinewidthContinuous::new().with_limits(0.0, 10.0);
        scale.train(&[Value::Float(-100.0), Value::Float(100.0)]);
        assert_eq!(scale.domain(), Some((0.0, 10.0)));
    }

    #[test]
    fn default_range_is_1_to_6() {
        let mut scale = ScaleLinewidthContinuous::new();
        scale.train(&[Value::Float(0.0), Value::Float(10.0)]);
        assert_eq!(scale.map_to_size(&Value::Float(0.0)), Some(1.0));
        assert_eq!(scale.map_to_size(&Value::Float(10.0)), Some(6.0));
    }
}
