use crate::aes::Aesthetic;
use crate::data::Value;

use super::color::RGBAColor;
use super::format::LabelFormatter;
use super::util::nice_step;
use super::Scale;

/// N-stop continuous color gradient scale.
/// Interpolates linearly between user-defined color stops.
#[derive(Clone)]
pub struct ScaleColorGradientN {
    aesthetic: Aesthetic,
    name: String,
    /// Color stops as (position_0_to_1, color) pairs, sorted by position.
    stops: Vec<(f64, RGBAColor)>,
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
    /// Formatter for [`format_value`](super::Scale::format_value) (#164).
    /// Falls back to the plain locale-formatted number when unset.
    value_formatter: Option<LabelFormatter>,
}

// Manual `Debug`: `value_formatter` (an `Arc<dyn Fn>`) isn't `Debug`.
impl std::fmt::Debug for ScaleColorGradientN {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScaleColorGradientN")
            .field("aesthetic", &self.aesthetic)
            .field("name", &self.name)
            .field("stops", &self.stops)
            .field("min", &self.min)
            .field("max", &self.max)
            .field("locale", &self.locale)
            .field("limits", &self.limits)
            .field("oob", &self.oob)
            .field("na_value", &self.na_value)
            .field("value_formatter", &self.value_formatter.is_some())
            .finish()
    }
}

impl ScaleColorGradientN {
    /// Create a new N-stop gradient for the given aesthetic.
    /// Stops are `(position, color)` where position is in [0, 1].
    pub fn new(aesthetic: Aesthetic, stops: Vec<(f64, RGBAColor)>) -> Self {
        let mut stops = stops;
        stops.sort_by(|a, b| a.0.total_cmp(&b.0));
        ScaleColorGradientN {
            aesthetic,
            name: String::new(),
            stops,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
            locale: None,
            limits: None,
            oob: super::Oob::Censor,
            na_value: super::DEFAULT_NA_VALUE,
            value_formatter: None,
        }
    }

    /// Set a formatter for [`format_value`](super::Scale::format_value) —
    /// a per-mark `data-fvalue`/tooltip-template value for this fill/colour
    /// scale (#164). Falls back to the plain locale-formatted number when
    /// unset (byte-identical default output).
    pub fn with_value_formatter<F>(mut self, f: F) -> Self
    where
        F: Fn(f64) -> String + Send + Sync + 'static,
    {
        self.value_formatter = Some(std::sync::Arc::new(f));
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

    /// Create a continuous viridis palette.
    pub fn viridis(aesthetic: Aesthetic) -> Self {
        Self::new(aesthetic, viridis_stops())
    }

    /// Create a continuous magma palette.
    pub fn magma(aesthetic: Aesthetic) -> Self {
        Self::new(aesthetic, magma_stops())
    }

    /// Create a continuous plasma palette.
    pub fn plasma(aesthetic: Aesthetic) -> Self {
        Self::new(aesthetic, plasma_stops())
    }

    /// Create a continuous inferno palette.
    pub fn inferno(aesthetic: Aesthetic) -> Self {
        Self::new(aesthetic, inferno_stops())
    }

    /// Interpolate the color at a normalized position t in [0, 1].
    fn color_at(&self, t: f64) -> RGBAColor {
        let t = t.clamp(0.0, 1.0);
        if self.stops.is_empty() {
            return RGBAColor::new(127, 127, 127);
        }
        if self.stops.len() == 1 {
            return self.stops[0].1;
        }
        // Find the two surrounding stops
        if t <= self.stops[0].0 {
            return self.stops[0].1;
        }
        if t >= self.stops[self.stops.len() - 1].0 {
            return self.stops[self.stops.len() - 1].1;
        }
        for i in 0..self.stops.len() - 1 {
            let (p0, c0) = &self.stops[i];
            let (p1, c1) = &self.stops[i + 1];
            if t >= *p0 && t <= *p1 {
                let range = p1 - p0;
                let local_t = if range.abs() < f64::EPSILON {
                    0.0
                } else {
                    (t - p0) / range
                };
                return c0.lerp(c1, local_t);
            }
        }
        self.stops[self.stops.len() - 1].1
    }
}

impl Scale for ScaleColorGradientN {
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
        Some(match &self.value_formatter {
            Some(fmt) => fmt(f),
            None => super::util::format_number_opt(f, self.locale.as_ref()),
        })
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
        // A genuinely missing or non-finite value, and a value censored
        // by `with_limits`, render as `na_value` (ggplot2's own
        // behaviour) instead of a plausible-looking colour (review PR#141
        // finding 3). A non-numeric-but-present value (e.g. the wrong
        // column type) keeps the pre-existing contract of falling through
        // to `map`'s own 0.0 fallback instead — this type's `map_to_color`
        // always returned `Some` for it before `with_limits`/`Oob` existed.
        if value.is_na() {
            return Some(self.na_value);
        }
        if let Some(f) = value.as_f64() {
            if !f.is_finite() {
                return Some(self.na_value);
            }
            if super::resolve_oob(f, self.limits, self.oob).is_none() {
                return Some(self.na_value);
            }
        }
        let t = self.map(value);
        let c = self.color_at(t);
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

    fn validate(&self, warnings: &mut Vec<String>) {
        if let Some((lo, hi)) = self.limits {
            if lo > hi {
                warnings.push(format!(
                    "scale_color/fill_gradientn: with_limits({lo}, {hi}) has lo > hi; every value maps to na_value"
                ));
            }
        }
    }
}

// ─── Continuous palette color stops ──────────────────────────────

fn c(r: u8, g: u8, b: u8) -> RGBAColor {
    RGBAColor::new(r, g, b)
}

fn viridis_stops() -> Vec<(f64, RGBAColor)> {
    let colors = [
        c(68, 1, 84),
        c(72, 26, 108),
        c(71, 47, 126),
        c(65, 68, 135),
        c(57, 86, 140),
        c(47, 104, 142),
        c(38, 121, 142),
        c(31, 138, 141),
        c(30, 155, 138),
        c(42, 172, 130),
        c(70, 188, 115),
        c(109, 202, 93),
        c(155, 213, 67),
        c(200, 222, 39),
        c(240, 229, 30),
        c(253, 231, 37),
    ];
    evenly_spaced_stops(&colors)
}

fn magma_stops() -> Vec<(f64, RGBAColor)> {
    let colors = [
        c(0, 0, 4),
        c(16, 12, 50),
        c(41, 17, 90),
        c(72, 12, 110),
        c(101, 19, 110),
        c(131, 29, 103),
        c(160, 42, 93),
        c(187, 55, 84),
        c(213, 72, 72),
        c(232, 99, 62),
        c(247, 131, 57),
        c(254, 167, 69),
        c(254, 203, 99),
        c(252, 235, 141),
        c(252, 254, 188),
        c(252, 253, 191),
    ];
    evenly_spaced_stops(&colors)
}

fn plasma_stops() -> Vec<(f64, RGBAColor)> {
    let colors = [
        c(13, 8, 135),
        c(53, 5, 157),
        c(82, 1, 163),
        c(109, 1, 159),
        c(133, 7, 147),
        c(156, 23, 127),
        c(175, 42, 106),
        c(192, 61, 85),
        c(206, 82, 66),
        c(218, 105, 46),
        c(228, 130, 24),
        c(236, 157, 6),
        c(240, 185, 11),
        c(239, 213, 38),
        c(232, 240, 73),
        c(240, 249, 33),
    ];
    evenly_spaced_stops(&colors)
}

fn inferno_stops() -> Vec<(f64, RGBAColor)> {
    let colors = [
        c(0, 0, 4),
        c(14, 11, 49),
        c(39, 15, 90),
        c(67, 10, 107),
        c(95, 13, 106),
        c(122, 21, 97),
        c(149, 33, 81),
        c(174, 49, 60),
        c(196, 69, 38),
        c(215, 95, 15),
        c(231, 124, 3),
        c(243, 155, 7),
        c(250, 189, 28),
        c(252, 222, 67),
        c(247, 252, 118),
        c(252, 255, 164),
    ];
    evenly_spaced_stops(&colors)
}

fn evenly_spaced_stops(colors: &[RGBAColor]) -> Vec<(f64, RGBAColor)> {
    let n = colors.len();
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        return vec![(0.0, colors[0])];
    }
    colors
        .iter()
        .enumerate()
        .map(|(i, c)| (i as f64 / (n - 1) as f64, *c))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_value_formats_a_finite_number_and_rejects_non_finite() {
        let g = ScaleColorGradientN::new(Aesthetic::Color, vec![(0.0, RGBAColor::new(0, 0, 0))]);
        assert_eq!(g.format_value(&Value::Float(42.0)), Some("42".to_string()));
        assert_eq!(g.format_value(&Value::Float(f64::INFINITY)), None);
    }

    #[test]
    fn with_value_formatter_overrides_format_value() {
        let g = ScaleColorGradientN::new(Aesthetic::Color, vec![(0.0, RGBAColor::new(0, 0, 0))])
            .with_value_formatter(|v| format!("#{v:.0}"));
        assert_eq!(g.format_value(&Value::Float(42.0)), Some("#42".to_string()));
        assert_eq!(g.format_value(&Value::Float(f64::INFINITY)), None);
    }

    #[test]
    fn debug_reports_whether_a_formatter_is_set() {
        let stops = vec![(0.0, RGBAColor::new(0, 0, 0))];
        let without = format!(
            "{:?}",
            ScaleColorGradientN::new(Aesthetic::Color, stops.clone())
        );
        assert!(without.contains("ScaleColorGradientN"));
        assert!(without.contains("value_formatter: false"));
        let with = format!(
            "{:?}",
            ScaleColorGradientN::new(Aesthetic::Color, stops)
                .with_value_formatter(|v| v.to_string())
        );
        assert!(with.contains("value_formatter: true"));
    }

    #[test]
    fn with_limits_fixes_the_domain_and_training_does_not_widen_it() {
        let mut g = ScaleColorGradientN::new(
            Aesthetic::Color,
            vec![
                (0.0, RGBAColor::new(0, 0, 0)),
                (1.0, RGBAColor::new(255, 255, 255)),
            ],
        )
        .with_limits(0.0, 10.0);
        g.train(&[Value::Float(-1000.0), Value::Float(1000.0)]);
        assert_eq!(g.domain(), Some((0.0, 10.0)));
    }

    #[test]
    fn test_gradient_n_interpolation() {
        let g = ScaleColorGradientN::new(
            Aesthetic::Color,
            vec![
                (0.0, RGBAColor::new(0, 0, 0)),
                (0.5, RGBAColor::new(255, 0, 0)),
                (1.0, RGBAColor::new(255, 255, 255)),
            ],
        );
        // At t=0 should be black
        let c0 = g.color_at(0.0);
        assert_eq!((c0.r, c0.g, c0.b), (0, 0, 0));
        // At t=0.5 should be red
        let c5 = g.color_at(0.5);
        assert_eq!((c5.r, c5.g, c5.b), (255, 0, 0));
        // At t=1.0 should be white
        let c1 = g.color_at(1.0);
        assert_eq!((c1.r, c1.g, c1.b), (255, 255, 255));
        // At t=0.25 should be ~midpoint between black and red
        let c25 = g.color_at(0.25);
        assert_eq!(c25.r, 127); // roughly half of 255
    }

    #[test]
    fn test_viridis_continuous_endpoints() {
        let g = ScaleColorGradientN::viridis(Aesthetic::Fill);
        let c0 = g.color_at(0.0);
        assert_eq!((c0.r, c0.g, c0.b), (68, 1, 84));
        let c1 = g.color_at(1.0);
        assert_eq!((c1.r, c1.g, c1.b), (253, 231, 37));
    }
}
