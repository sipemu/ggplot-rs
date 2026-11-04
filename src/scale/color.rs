use crate::aes::Aesthetic;
use crate::data::Value;
use std::collections::HashMap;

use super::util::nice_step;
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
    /// A palette computed for the number of levels (#408: ggplot2's
    /// `hue_pal()` / `viridis_pal()` spread their colours over `n`);
    /// overrides `palette` when set.
    generated: Option<GeneratedPalette>,
}

/// ggplot2's `scale_colour_hue()` parameters (`scales::hue_pal`): `n` hues
/// evenly spaced over `h` (degrees), at chroma `c` and luminance `l`.
#[derive(Clone, Debug, PartialEq)]
pub struct HuePalette {
    pub h: (f64, f64),
    pub c: f64,
    pub l: f64,
    pub h_start: f64,
    /// `1`, or `-1` to reverse.
    pub direction: i32,
}

impl Default for HuePalette {
    /// ggplot2's default discrete palette.
    fn default() -> Self {
        HuePalette {
            h: (15.0, 375.0),
            c: 100.0,
            l: 65.0,
            h_start: 0.0,
            direction: 1,
        }
    }
}

/// A discrete palette generated for the number of levels.
#[derive(Clone, Debug, PartialEq)]
pub enum GeneratedPalette {
    Hue(HuePalette),
    /// `scale_colour_viridis_d()`: `n` colours sampled from a viridis map
    /// (`option` 'A' magma, 'B' inferno, 'C' plasma, 'D' viridis, 'E'
    /// cividis, 'F' rocket, 'G' mako, 'H' turbo) between
    /// `begin` and `end`, reversed with `direction = -1`.
    Viridis {
        option: char,
        begin: f64,
        end: f64,
        direction: i32,
    },
}

impl GeneratedPalette {
    /// The palette for `n` levels.
    pub fn colors(&self, n: usize) -> Vec<RGBAColor> {
        let n = n.max(1);
        match self {
            GeneratedPalette::Hue(p) => hue_palette(n, p),
            GeneratedPalette::Viridis {
                option,
                begin,
                end,
                direction,
            } => {
                let opt = option.to_ascii_uppercase();
                let lut = super::viridis_lut::VIRIDIS_LUT
                    .iter()
                    .find(|(o, _)| *o == opt)
                    .map_or(&super::viridis_lut::VIRIDIS_LUT[3].1, |(_, l)| l);
                // Linear interpolation on the 256-entry map (viridisLite
                // splines in Lab; the difference is ≤ 1 per channel).
                let at = |t: f64| {
                    let f = t.clamp(0.0, 1.0) * 255.0;
                    let (i, j) = (f.floor() as usize, (f.ceil() as usize).min(255));
                    let w = f - i as f64;
                    let mix = |k: usize| {
                        (lut[i][k] as f64 * (1.0 - w) + lut[j][k] as f64 * w).round() as u8
                    };
                    RGBAColor::new(mix(0), mix(1), mix(2))
                };
                let mut out: Vec<RGBAColor> = (0..n)
                    .map(|i| {
                        let t = if n == 1 {
                            *begin
                        } else {
                            begin + (end - begin) * i as f64 / (n - 1) as f64
                        };
                        at(t)
                    })
                    .collect();
                if *direction < 0 {
                    out.reverse();
                }
                out
            }
        }
    }
}

/// `grDevices::hcl(h, c, l)` (CIE-LUV polar → sRGB, D65), R's implementation.
pub fn hcl_color(h: f64, c: f64, l: f64) -> RGBAColor {
    const WHITE_Y: f64 = 100.0;
    const WHITE_U: f64 = 0.1978398;
    const WHITE_V: f64 = 0.4683363;
    let hr = h.to_radians();
    let (u_, v_) = (c * hr.cos(), c * hr.sin());
    let (x, y, z) = if l <= 0.0 && u_ == 0.0 && v_ == 0.0 {
        (0.0, 0.0, 0.0)
    } else {
        let y = WHITE_Y
            * if l > 7.999592 {
                ((l + 16.0) / 116.0).powi(3)
            } else {
                l / 903.3
            };
        let u = u_ / (13.0 * l) + WHITE_U;
        let v = v_ / (13.0 * l) + WHITE_V;
        let x = 9.0 * y * u / (4.0 * v);
        let z = -x / 3.0 - 5.0 * y + 3.0 * y / v;
        (x, y, z)
    };
    let gamma = |u: f64| {
        let u = u / WHITE_Y;
        if u > 0.00304 {
            1.055 * u.powf(1.0 / 2.4) - 0.055
        } else {
            12.92 * u
        }
    };
    let byte = |v: f64| (255.0 * v.clamp(0.0, 1.0) + 0.5) as u8;
    RGBAColor::new(
        byte(gamma(3.240479 * x - 1.537150 * y - 0.498535 * z)),
        byte(gamma(-0.969256 * x + 1.875992 * y + 0.041556 * z)),
        byte(gamma(0.055648 * x - 0.204043 * y + 1.057311 * z)),
    )
}

/// `scales::hue_pal(h, c, l, h.start, direction)(n)`.
pub fn hue_palette(n: usize, p: &HuePalette) -> Vec<RGBAColor> {
    let n = n.max(1);
    let (h0, mut h1) = p.h;
    if (h1 - h0).rem_euclid(360.0) < 1.0 {
        h1 -= 360.0 / n as f64;
    }
    let mut out: Vec<RGBAColor> = (0..n)
        .map(|i| {
            let h = if n == 1 {
                h0
            } else {
                h0 + (h1 - h0) * i as f64 / (n - 1) as f64
            };
            hcl_color((h + p.h_start).rem_euclid(360.0), p.c, p.l)
        })
        .collect();
    if p.direction == -1 {
        out.reverse();
    }
    out
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
            generated: None,
        }
    }

    /// Colours computed for the number of levels (ggplot2's `hue_pal()` /
    /// `viridis_pal()`), e.g. `GeneratedPalette::Hue(HuePalette::default())`
    /// — exactly ggplot2's default colours for any number of groups.
    pub fn with_generated(mut self, p: GeneratedPalette) -> Self {
        self.generated = Some(p);
        self
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
        self.generated = None;
        self
    }

    pub fn with_named_palette(mut self, name: &super::palettes::PaletteName) -> Self {
        self.palette = super::palettes::palette(name).to_vec();
        self.generated = None;
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
        if let Some(g) = &self.generated {
            let pal = g.colors(self.levels.len().max(idx + 1));
            return pal[idx % pal.len()];
        }
        if self.palette.is_empty() {
            return DEFAULT_PALETTE[idx % DEFAULT_PALETTE.len()];
        }
        self.palette[idx % self.palette.len()]
    }

    /// Get color for a value.
    pub fn color_for_value(&self, value: &Value) -> RGBAColor {
        let key = value.key_str();
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
            let key = v.key_str();
            if !self.level_index.contains_key(key.as_ref()) {
                self.push_level(key.into_owned());
            }
        }
        if self.levels.len() != before {
            self.resort();
        }
    }

    fn map(&self, value: &Value) -> f64 {
        let key = value.key_str();
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
#[derive(Clone)]
pub struct ScaleColorContinuous {
    aesthetic: Aesthetic,
    name: String,
    low: RGBAColor,
    high: RGBAColor,
    min: f64,
    max: f64,
    /// Locale for the default break labels (`None` = default marks).
    locale: Option<crate::locale::Locale>,
    /// Fixed `(lo, hi)` domain ([`with_limits`](Self::with_limits)):
    /// `train` no longer widens it, and `map_to_color` maps against it
    /// instead of the trained min/max.
    limits: Option<(f64, f64)>,
    /// Out-of-bounds policy for `limits` ([`with_oob`](Self::with_oob)).
    /// Default [`Oob::Censor`].
    oob: super::Oob,
    /// Colour for a genuinely missing or out-of-bounds-and-censored value
    /// ([`with_na_value`](Self::with_na_value)). Default ggplot2's
    /// `"grey50"`, `(127, 127, 127)`.
    na_value: (u8, u8, u8),
    /// Formatter for [`format_value`](super::Scale::format_value) (#164,
    /// same shape as `ScaleContinuous::with_value_formatter`), distinct
    /// from the axis-break label formatting this scale has none of. Falls
    /// back to the plain locale-formatted number when unset. Wrapped in
    /// [`UnwindSafeFormatter`](super::format::UnwindSafeFormatter) so this
    /// field doesn't drop `UnwindSafe`/`RefUnwindSafe` from the struct.
    value_formatter: Option<super::format::UnwindSafeFormatter>,
}

// Manual `Debug`, since `value_formatter` (an `Arc<dyn Fn>`) isn't
// `Debug`; every other field is printed as the derive would.
impl std::fmt::Debug for ScaleColorContinuous {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScaleColorContinuous")
            .field("aesthetic", &self.aesthetic)
            .field("name", &self.name)
            .field("low", &self.low)
            .field("high", &self.high)
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

impl ScaleColorContinuous {
    pub fn new(aesthetic: Aesthetic) -> Self {
        ScaleColorContinuous {
            aesthetic,
            name: String::new(),
            low: RGBAColor::new(0, 0, 255),
            high: RGBAColor::new(255, 0, 0),
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
        self.value_formatter = Some(super::format::UnwindSafeFormatter(std::sync::Arc::new(f)));
        self
    }

    pub fn with_colors(mut self, low: RGBAColor, high: RGBAColor) -> Self {
        self.low = low;
        self.high = high;
        self
    }

    /// Fix the colour domain to `[lo, hi]` — e.g. `(-1.0, 1.0)` for a
    /// correlation matrix — instead of whatever the data happens to span.
    /// Out-of-range values follow [`with_oob`](Self::with_oob) (default:
    /// `na_value`, not the geom's own default colour).
    pub fn with_limits(mut self, lo: f64, hi: f64) -> Self {
        self.limits = Some((lo, hi));
        self
    }

    /// Out-of-bounds policy for [`with_limits`](Self::with_limits):
    /// [`Oob::Censor`] (default, ggplot2's own default) or
    /// [`Oob::Squish`].
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

    pub fn color_at(&self, t: f64) -> RGBAColor {
        self.low.lerp(&self.high, t)
    }
}

impl Scale for ScaleColorContinuous {
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
            Some(fmt) => fmt.call(f),
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
        // A genuinely missing value, or a non-finite one (NaN/+-Inf),
        // renders as `na_value` — ggplot2's own behaviour, and review
        // PR#141 finding 3. A non-numeric-but-present value (e.g. the
        // wrong column type) keeps the pre-existing contract of falling
        // through to `map`'s own 0.0 fallback instead (this type's
        // `map_to_color` always returned `Some` before `with_limits`/`Oob`
        // existed, and nothing here can tell "wrong type" apart from
        // "never had a numeric concept to begin with").
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
                    "scale_color/fill_gradient: with_limits({lo}, {hi}) has lo > hi; every value maps to na_value"
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aes::Aesthetic;

    #[test]
    fn continuous_format_value_rejects_non_finite() {
        let s = ScaleColorContinuous::new(Aesthetic::Fill);
        assert_eq!(s.format_value(&Value::Float(3.5)), Some("3.5".to_string()));
        assert_eq!(s.format_value(&Value::Float(f64::NAN)), None);
        assert_eq!(s.format_value(&Value::Na), None);
    }

    #[test]
    fn continuous_with_value_formatter_overrides_format_value() {
        let s =
            ScaleColorContinuous::new(Aesthetic::Fill).with_value_formatter(|v| format!("${v:.2}"));
        assert_eq!(
            s.format_value(&Value::Float(1234.5)),
            Some("$1234.50".to_string())
        );
        // Non-finite is still rejected before the formatter ever runs.
        assert_eq!(s.format_value(&Value::Float(f64::NAN)), None);
    }

    #[test]
    fn continuous_debug_reports_whether_a_formatter_is_set() {
        let without = format!("{:?}", ScaleColorContinuous::new(Aesthetic::Fill));
        assert!(without.contains("ScaleColorContinuous"));
        assert!(without.contains("value_formatter: false"));
        let with = format!(
            "{:?}",
            ScaleColorContinuous::new(Aesthetic::Fill).with_value_formatter(|v| v.to_string())
        );
        assert!(with.contains("value_formatter: true"));
    }

    /// #164 review: `value_formatter`'s `Arc<dyn Fn>` must not drop
    /// `UnwindSafe`/`RefUnwindSafe` (or `Send`/`Sync`/`Clone`) from
    /// `ScaleColorContinuous` — `cargo-semver-checks` flags losing an
    /// auto trait as a major-version break, which a patch must not cause.
    #[test]
    fn scale_color_continuous_keeps_every_auto_trait() {
        fn assert_traits<
            T: Send + Sync + std::panic::UnwindSafe + std::panic::RefUnwindSafe + Clone,
        >() {
        }
        assert_traits::<ScaleColorContinuous>();
    }
}

#[cfg(test)]
mod hue_tests {
    use super::*;

    fn hex(c: RGBAColor) -> String {
        format!("#{:02X}{:02X}{:02X}", c.r, c.g, c.b)
    }

    #[test]
    fn hue_pal_matches_scales() {
        // R: scales::hue_pal()(2), (5), and a custom one.
        let d = HuePalette::default();
        let got: Vec<String> = hue_palette(2, &d).into_iter().map(hex).collect();
        assert_eq!(got, ["#F8766D", "#00BFC4"]);
        let got: Vec<String> = hue_palette(5, &d).into_iter().map(hex).collect();
        assert_eq!(got, ["#F8766D", "#A3A500", "#00BF7D", "#00B0F6", "#E76BF3"]);
        let p = HuePalette {
            h: (0.0, 180.0),
            c: 50.0,
            l: 80.0,
            h_start: 30.0,
            direction: -1,
        };
        let got: Vec<String> = hue_palette(4, &p).into_iter().map(hex).collect();
        assert_eq!(got, ["#70D5E7", "#7BD9AD", "#C6CC82", "#F7B8A1"]);
    }

    #[test]
    fn viridis_d_is_close_to_scales() {
        // R: scales::viridis_pal(option = "D")(4); viridisLite splines in
        // Lab space, we interpolate its 256-entry map linearly.
        let want = [
            (0x44, 0x01, 0x54),
            (0x31, 0x68, 0x8E),
            (0x35, 0xB7, 0x79),
            (0xFD, 0xE7, 0x25),
        ];
        let got = GeneratedPalette::Viridis {
            option: 'D',
            begin: 0.0,
            end: 1.0,
            direction: 1,
        }
        .colors(4);
        for (g, w) in got.iter().zip(want) {
            for (a, b) in [(g.r, w.0), (g.g, w.1), (g.b, w.2)] {
                assert!((a as i32 - b).abs() <= 1, "{} vs {w:?}", hex(*g));
            }
        }
        // option A, begin .2, end .8, reversed: #FE9F6D #B63679 #3B0F70.
        let want = [(0xFE, 0x9F, 0x6D), (0xB6, 0x36, 0x79), (0x3B, 0x0F, 0x70)];
        let got = GeneratedPalette::Viridis {
            option: 'A',
            begin: 0.2,
            end: 0.8,
            direction: -1,
        }
        .colors(3);
        for (g, w) in got.iter().zip(want) {
            for (a, b) in [(g.r, w.0), (g.g, w.1), (g.b, w.2)] {
                assert!((a as i32 - b).abs() <= 1, "{} vs {w:?}", hex(*g));
            }
        }
    }
}
