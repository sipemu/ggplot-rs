use std::sync::Arc;

use crate::aes::Aesthetic;
use crate::data::Value;

use super::format::LabelFormatter;
use super::transform::ScaleTransform;
use super::Scale;

/// Continuous linewidth scale — maps data values to a stroke-width range
/// (R's `scale_linewidth`). Unlike the position (x/y) scales, `with_limits`
/// squishes out-of-range values to the nearest limit rather than filtering
/// them. `with_transform` is applied to the value and the limits *before*
/// rescaling into `range`, inside [`map`](Scale::map) /
/// [`map_to_size`](Scale::map_to_size) — it does *not* override
/// [`Scale::transform`] (that's for pre-stat domain transforms like
/// `scale_x_log10`, which linewidth has no use for).
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

    /// Transform the value and the (untransformed) `with_limits` domain
    /// before rescaling into `range` (ggplot2's order for a scale `trans`):
    /// `t = (T(v) - T(min)) / (T(max) - T(min))`. Default `Identity`, which
    /// matches ggplot2's own default for `scale_linewidth` — a *linear*
    /// `rescale_pal`, unlike `scale_size`'s default `area_pal` (sqrt-like,
    /// since visual area grows with the square of a linear dimension).
    /// Pass `Sqrt` to opt into that area-proportional look for linewidth
    /// too.
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

    /// Whether the configured transform is undefined somewhere in
    /// `[min, max]` itself — e.g. `Log10` with a limit `<= 0` (common: a
    /// flow-count column that legitimately contains 0, or an explicit
    /// `with_limits(0, vmax)`). When this is true, no value in the domain
    /// has a well-defined fraction under this transform; `validate()`
    /// reports it once per scale, and `map_to_size` returns `None` for
    /// every value (falling back to the geom's own width) rather than
    /// guessing.
    fn domain_transform_is_broken(&self) -> bool {
        if !self.trained || !self.min.is_finite() || !self.max.is_finite() {
            return false;
        }
        let t_min = self.scale_transform.apply(self.min);
        let t_max = self.scale_transform.apply(self.max);
        !t_min.is_finite() || !t_max.is_finite()
    }

    /// Normalized fraction in `[0, 1]`, ggplot2's order: squish the value to
    /// the domain, *then* transform both it and the (untransformed) limits,
    /// then rescale over the transformed range — not the other way around.
    /// Transforming an already-rescaled `[0, 1]` fraction (an earlier,
    /// incorrect implementation) collapses most transforms: e.g.
    /// `Log10`/`Reverse`/`Exp` of values near 0 and 1 all land near the same
    /// couple of outputs, regardless of the input. Squishing *before* the
    /// transform (rather than transforming the raw value) means an
    /// ordinary out-of-limits value — e.g. `v = 0` under `Log10` with
    /// `limits(1, 100)`, which is outside the domain but the domain itself
    /// is fine — still squishes to the nearest end (here, the minimum
    /// width) instead of landing on an undefined `Log10(0)`. The
    /// transformed limits are sorted ascending before rescaling, so a
    /// transform that reverses order (`Reverse`, or `Reciprocal` on
    /// positive limits) still produces the expected result: the domain
    /// minimum gets the *larger* output, since its transformed value is the
    /// larger of the two transformed limits.
    ///
    /// Returns `0.5` (not NaN) when [`domain_transform_is_broken`](Self::domain_transform_is_broken)
    /// — this keeps callers that need a finite `[0, 1]` position regardless
    /// (e.g. a legend key's x/y) from ever seeing NaN; `map_to_size` makes
    /// its own, separate check and returns `None` in that case instead of
    /// this placeholder.
    fn fraction(&self, value: &Value) -> f64 {
        let f = match value.as_f64() {
            Some(f) => f,
            None => return 0.0,
        };
        if !f.is_finite() {
            return 0.0;
        }
        if !self.trained || !self.min.is_finite() || !self.max.is_finite() {
            return 0.5;
        }
        if self.domain_transform_is_broken() {
            return 0.5;
        }
        let (dom_lo, dom_hi) = (self.min.min(self.max), self.min.max(self.max));
        let clamped = f.clamp(dom_lo, dom_hi);
        let t_min = self.scale_transform.apply(self.min);
        let t_max = self.scale_transform.apply(self.max);
        let t_v = self.scale_transform.apply(clamped);
        let (lo, hi) = (t_min.min(t_max), t_min.max(t_max));
        let range = hi - lo;
        if !t_v.is_finite() || range.abs() < f64::EPSILON {
            return 0.5;
        }
        ((t_v - lo) / range).clamp(0.0, 1.0)
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
        // NA / non-numeric / non-finite: no width to report, so the caller
        // (every geom calls this through `.unwrap_or(self.width)`) falls
        // back to the geom's own configured width instead of silently
        // getting the scale's minimum.
        let f = value.as_f64()?;
        if !f.is_finite() {
            return None;
        }
        // The transform is undefined somewhere in the domain itself (e.g.
        // Log10 with a limit <= 0): no well-defined width for *any* value,
        // so every row falls back to the geom's own width too, instead of
        // the previous behaviour of silently reporting the midpoint width.
        // `validate()` reports this once per scale, at build time.
        if self.domain_transform_is_broken() {
            return None;
        }
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

    fn validate(&self, warnings: &mut Vec<String>) {
        if self.domain_transform_is_broken() {
            warnings.push(format!(
                "scale_linewidth: {:?} is undefined at the domain limits ({}, {}); \
                 every row falls back to the geom's own linewidth instead of a mapped size \
                 (ggplot2's equivalent: NA plus a warning)",
                self.scale_transform, self.min, self.max
            ));
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

    // ─── Review finding #4: transform the value and the limits first, then
    // rescale (ggplot2's order) — not the previous, incorrect order that
    // transformed the already-rescaled [0,1] fraction, which collapsed
    // every transform except Sqrt-with-a-zero-lower-limit. ───

    #[test]
    fn log10_transform_with_a_nonzero_lower_limit() {
        let scale = ScaleLinewidthContinuous::new()
            .with_range(1.0, 6.0)
            .with_transform(ScaleTransform::Log10)
            .with_limits(1.0, 100.0);
        // log10(1)=0, log10(100)=2: v=10 (log10=1) is exactly the midpoint.
        let cases = [(1.0, 1.0), (10.0, 3.5), (100.0, 6.0)];
        for (v, expected) in cases {
            let width = scale.map_to_size(&Value::Float(v)).unwrap();
            assert!(
                (width - expected).abs() < 1e-9,
                "v={v}: width={width}, expected={expected}"
            );
        }
    }

    #[test]
    fn reverse_transform_flips_which_end_gets_the_larger_width() {
        let scale = ScaleLinewidthContinuous::new()
            .with_range(1.0, 6.0)
            .with_transform(ScaleTransform::Reverse)
            .with_limits(1.0, 100.0);
        // The domain minimum gets the *larger* output under Reverse.
        let min_width = scale.map_to_size(&Value::Float(1.0)).unwrap();
        let max_width = scale.map_to_size(&Value::Float(100.0)).unwrap();
        assert!((min_width - 6.0).abs() < 1e-9, "min_width={min_width}");
        assert!((max_width - 1.0).abs() < 1e-9, "max_width={max_width}");
    }

    #[test]
    fn sqrt_transform_with_a_nonzero_lower_limit() {
        let scale = ScaleLinewidthContinuous::new()
            .with_range(1.0, 6.0)
            .with_transform(ScaleTransform::Sqrt)
            .with_limits(4.0, 100.0);
        // sqrt(4)=2, sqrt(100)=10, sqrt(16)=4 -> fraction (4-2)/8 = 0.25.
        let cases = [(4.0, 1.0), (16.0, 2.25), (100.0, 6.0)];
        for (v, expected) in cases {
            let width = scale.map_to_size(&Value::Float(v)).unwrap();
            assert!(
                (width - expected).abs() < 1e-9,
                "v={v}: width={width}, expected={expected}"
            );
        }
    }

    /// Review finding #6: NA / non-numeric / non-finite input must not map
    /// to the minimum width — the scale has no width to report, so callers
    /// (`scales.map_linewidth(..).unwrap_or(self.width)`) fall back to the
    /// geom's own configured width instead.
    #[test]
    fn na_or_non_finite_input_returns_none_not_the_minimum() {
        let scale = ScaleLinewidthContinuous::new()
            .with_range(1.0, 6.0)
            .with_limits(0.0, 10.0);
        assert_eq!(scale.map_to_size(&Value::Na), None);
        assert_eq!(scale.map_to_size(&Value::Str("x".into())), None);
        assert_eq!(scale.map_to_size(&Value::Float(f64::NAN)), None);
        assert_eq!(scale.map_to_size(&Value::Float(f64::INFINITY)), None);
    }

    // ─── Review R-2: a transform undefined somewhere in the domain (e.g.
    // Log10 with a limit <= 0) must map every value to None (falls back to
    // the geom's own width) and warn once, instead of the previous
    // behaviour of silently reporting the midpoint width. An ordinary
    // out-of-limits value, when the domain itself is fine, still squishes
    // to the nearest end. ───

    #[test]
    fn log10_with_a_zero_lower_limit_maps_every_value_to_none_and_warns() {
        let scale = ScaleLinewidthContinuous::new()
            .with_range(1.0, 6.0)
            .with_transform(ScaleTransform::Log10)
            .with_limits(0.0, 100.0);
        for v in [0.0, 1.0, 50.0, 100.0] {
            assert_eq!(
                scale.map_to_size(&Value::Float(v)),
                None,
                "v={v} should map to None, not the midpoint width"
            );
        }
        let mut warnings = Vec::new();
        scale.validate(&mut warnings);
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("Log10"));
    }

    #[test]
    fn a_value_trained_to_zero_breaks_log10_the_same_way() {
        // The domain minimum comes from training on data that legitimately
        // contains 0 (e.g. a flow count), not an explicit `with_limits`.
        let mut scale = ScaleLinewidthContinuous::new()
            .with_range(1.0, 6.0)
            .with_transform(ScaleTransform::Log10);
        scale.train(&[Value::Float(0.0), Value::Float(100.0)]);
        assert_eq!(scale.map_to_size(&Value::Float(50.0)), None);
        let mut warnings = Vec::new();
        scale.validate(&mut warnings);
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn out_of_limits_value_squishes_to_the_nearest_end_even_under_log10() {
        // The domain (1, 100) is fine for Log10; v=0 is simply outside it,
        // like any other oob value — squish to the minimum, not None.
        let scale = ScaleLinewidthContinuous::new()
            .with_range(1.0, 6.0)
            .with_transform(ScaleTransform::Log10)
            .with_limits(1.0, 100.0);
        assert_eq!(scale.map_to_size(&Value::Float(0.0)), Some(1.0));
        assert_eq!(scale.map_to_size(&Value::Float(-50.0)), Some(1.0));
        assert_eq!(scale.map_to_size(&Value::Float(1000.0)), Some(6.0));

        let mut warnings = Vec::new();
        scale.validate(&mut warnings);
        assert!(
            warnings.is_empty(),
            "a fine domain shouldn't warn: {warnings:?}"
        );
    }

    #[test]
    fn fraction_stays_finite_even_when_the_domain_transform_is_broken() {
        // Legend/axis-style callers need a finite [0,1] position regardless.
        let scale = ScaleLinewidthContinuous::new()
            .with_transform(ScaleTransform::Log10)
            .with_limits(0.0, 100.0);
        let t = scale.map(&Value::Float(50.0));
        assert!(t.is_finite());
    }

    /// End-to-end: the warning reaches `GGPlot`'s own warnings channel, not
    /// just the scale's own `validate()`.
    #[test]
    fn broken_domain_transform_warning_reaches_the_plot_warnings() {
        use crate::prelude::*;

        let (_svg, warnings) = GGPlot::new(vec![
            ("x".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
            ("y".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
            ("xend".into(), vec![Value::Float(0.5), Value::Float(1.5)]),
            ("yend".into(), vec![Value::Float(0.5), Value::Float(1.5)]),
            ("size".into(), vec![Value::Float(0.0), Value::Float(100.0)]),
        ])
        .aes(
            Aes::new()
                .x("x")
                .y("y")
                .xend("xend")
                .yend("yend")
                .linewidth("size"),
        )
        .geom_segment()
        .scale_linewidth(ScaleLinewidthContinuous::new().with_transform(ScaleTransform::Log10))
        .render_svg_native_with_warnings(200, 200)
        .unwrap();

        assert!(warnings.iter().any(|w| w.contains("Log10")), "{warnings:?}");
    }

    // ─── Coverage: builders and Scale trait methods not exercised above ───

    #[test]
    fn with_labels_overrides_the_formatted_label_for_each_break() {
        let scale = ScaleLinewidthContinuous::new()
            .with_name("flow size")
            .with_breaks(vec![1.0, 2.0])
            .with_labels(vec!["small".into(), "big".into()])
            .with_limits(0.0, 10.0);
        assert_eq!(scale.name(), "flow size");
        let breaks = scale.breaks();
        let labels: Vec<&str> = breaks.iter().map(|(_, l)| l.as_str()).collect();
        assert_eq!(labels, vec!["small", "big"]);
    }

    #[test]
    fn with_labels_shorter_than_breaks_falls_back_to_the_formatted_number() {
        // Only the first break gets a custom label; the second falls back
        // to `format_label` (no formatter, no locale -> plain number).
        let scale = ScaleLinewidthContinuous::new()
            .with_breaks(vec![1.0, 2.5])
            .with_labels(vec!["small".into()])
            .with_limits(0.0, 10.0);
        let breaks = scale.breaks();
        assert_eq!(breaks[0].1, "small");
        assert_eq!(breaks[1].1, "2.5");
    }

    #[test]
    fn with_label_formatter_formats_every_break() {
        let scale = ScaleLinewidthContinuous::new()
            .with_breaks(vec![1.0, 2.0])
            .with_label_formatter(|v| format!("{v:.1}mm"))
            .with_limits(0.0, 10.0);
        let labels: Vec<String> = scale.breaks().into_iter().map(|(_, l)| l).collect();
        assert_eq!(labels, vec!["1.0mm", "2.0mm"]);
    }

    #[test]
    fn with_locale_formats_default_break_labels() {
        let mut scale = ScaleLinewidthContinuous::new().with_locale(crate::locale::Locale::de_de());
        scale.train(&[Value::Float(1000.0), Value::Float(2000.0)]);
        let breaks = scale.breaks();
        assert!(!breaks.is_empty());
        // de-DE uses '.' as the thousands mark — at least one label should
        // show it for a four-digit break value.
        assert!(breaks.iter().any(|(_, l)| l.contains('.')), "{breaks:?}");
    }

    #[test]
    fn set_default_locale_only_applies_when_no_locale_was_set() {
        let mut with_own =
            ScaleLinewidthContinuous::new().with_locale(crate::locale::Locale::de_de());
        with_own.set_default_locale(&crate::locale::Locale::en_us());
        assert_eq!(
            with_own.locale().unwrap().tag,
            crate::locale::Locale::de_de().tag
        );

        let mut without_own = ScaleLinewidthContinuous::new();
        assert!(without_own.locale().is_none());
        without_own.set_default_locale(&crate::locale::Locale::en_us());
        assert_eq!(
            without_own.locale().unwrap().tag,
            crate::locale::Locale::en_us().tag
        );
    }

    #[test]
    fn degenerate_range_gives_a_single_break_at_the_midpoint() {
        let mut scale = ScaleLinewidthContinuous::new();
        scale.train(&[Value::Float(5.0), Value::Float(5.0)]);
        let breaks = scale.breaks();
        assert_eq!(breaks.len(), 1);
        assert_eq!(breaks[0].0, 0.5);
        assert_eq!(breaks[0].1, "5");
    }

    #[test]
    fn untrained_scale_has_no_breaks() {
        let scale = ScaleLinewidthContinuous::new();
        assert!(scale.breaks().is_empty());
    }

    #[test]
    fn clone_box_preserves_configuration() {
        let scale = ScaleLinewidthContinuous::new()
            .with_range(2.0, 9.0)
            .with_name("w")
            .with_limits(0.0, 5.0);
        let cloned = scale.clone_box();
        assert_eq!(cloned.name(), "w");
        assert_eq!(
            cloned.map_to_size(&Value::Float(5.0)),
            scale.map_to_size(&Value::Float(5.0))
        );
    }

    #[test]
    fn reset_training_clears_an_auto_trained_domain_but_not_fixed_limits() {
        let mut auto = ScaleLinewidthContinuous::new();
        auto.train(&[Value::Float(1.0), Value::Float(9.0)]);
        assert_eq!(auto.domain(), Some((1.0, 9.0)));
        auto.reset_training();
        assert_eq!(auto.domain(), None);

        let mut fixed = ScaleLinewidthContinuous::new().with_limits(0.0, 10.0);
        fixed.reset_training();
        assert_eq!(fixed.domain(), Some((0.0, 10.0)));
    }

    #[test]
    fn default_trait_impl_matches_new() {
        let default_scale = ScaleLinewidthContinuous::default();
        let new_scale = ScaleLinewidthContinuous::new();
        assert_eq!(default_scale.name(), new_scale.name());
        assert_eq!(default_scale.breaks(), new_scale.breaks());
    }

    #[test]
    fn set_name_overrides_the_axis_legend_title() {
        let mut scale = ScaleLinewidthContinuous::new();
        scale.set_name("custom title");
        assert_eq!(scale.name(), "custom title");
    }

    // `map` (the `Scale` trait method) calls `fraction` directly, without
    // `map_to_size`'s own NA/non-finite/broken-domain short-circuits — so
    // it has to handle those cases itself too, and always returns a finite
    // `[0, 1]` number (never panics, never NaN) for any caller that needs
    // a plain position rather than a width.
    #[test]
    fn map_handles_na_non_finite_and_untrained_without_panicking() {
        let untrained = ScaleLinewidthContinuous::new();
        assert_eq!(untrained.map(&Value::Float(5.0)), 0.5);

        let trained = ScaleLinewidthContinuous::new().with_limits(0.0, 10.0);
        assert_eq!(trained.map(&Value::Na), 0.0);
        assert_eq!(trained.map(&Value::Str("x".into())), 0.0);
        assert_eq!(trained.map(&Value::Float(f64::NAN)), 0.0);
    }
}
