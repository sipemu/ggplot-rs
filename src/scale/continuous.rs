use std::sync::Arc;

use crate::aes::Aesthetic;
use crate::data::Value;

use super::format::LabelFormatter;
use super::sec_axis::SecAxis;
use super::transform::ScaleTransform;
use super::util::extended_breaks;
use super::Scale;

/// Locale-dependent break labels ([`ScaleContinuous::with_localized_labels`]).
pub type LocalizedLabels = Arc<dyn Fn(Option<&crate::locale::Locale>) -> Vec<String> + Send + Sync>;

/// Continuous linear scale.
#[derive(Clone)]
pub struct ScaleContinuous {
    aesthetic: Aesthetic,
    name: String,
    min: f64,
    max: f64,
    trained: bool,
    filter_oob: bool,
    // Per-side expansion: (mult_lower, add_lower, mult_upper, add_upper).
    expand: (f64, f64, f64, f64),
    // Axis position: false = default (bottom for x, left for y), true = opposite
    // side (top for x, right for y).
    position_opposite: bool,
    pub(crate) scale_transform: ScaleTransform,
    custom_breaks: Option<Vec<f64>>,
    custom_labels: Option<Vec<String>>,
    /// Labels for `custom_breaks` resolved against the scale's locale.
    localized_labels: Option<LocalizedLabels>,
    pub(crate) sec_axis: Option<SecAxis>,
    label_formatter: Option<LabelFormatter>,
    /// Locale for the default labels (`None` = default marks).
    locale: Option<crate::locale::Locale>,
    /// Formatter for `format_value` (B2-4: `data-fvalue`/tooltip templates),
    /// distinct from `label_formatter` — e.g. a full currency string where
    /// the axis itself shows an abbreviated break.
    value_formatter: Option<LabelFormatter>,
    /// Target number of breaks (ggplot2's `n.breaks`, #408; default 5).
    n_breaks: usize,
    /// A time-of-day axis (`scale_x_time`, #408): values are seconds, breaks
    /// fall on round time steps.
    time: bool,
    /// `scale_x_binned(n.breaks)` (#408): bins between about this many breaks.
    binned: Option<usize>,
    /// The bin edges, fixed by the first layer binned (later layers share
    /// them); also the axis breaks.
    bin_edges: Vec<f64>,
    /// Draw ggplot2's `guide_axis_logticks()` ticks (#409).
    log_ticks: bool,
}

impl ScaleContinuous {
    pub fn new() -> Self {
        ScaleContinuous {
            aesthetic: Aesthetic::X,
            name: String::new(),
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
            trained: false,
            filter_oob: false,
            n_breaks: 5,
            time: false,
            binned: None,
            bin_edges: Vec::new(),
            log_ticks: false,
            expand: (0.05, 0.0, 0.05, 0.0),
            position_opposite: false,
            scale_transform: ScaleTransform::Identity,
            custom_breaks: None,
            custom_labels: None,
            localized_labels: None,
            sec_axis: None,
            label_formatter: None,
            locale: None,
            value_formatter: None,
        }
    }

    pub fn for_aesthetic(mut self, aes: Aesthetic) -> Self {
        self.aesthetic = aes;
        self
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn with_limits(mut self, min: f64, max: f64) -> Self {
        self.min = min;
        self.max = max;
        self.trained = true;
        self.filter_oob = true;
        self
    }

    pub fn with_transform(mut self, transform: ScaleTransform) -> Self {
        self.scale_transform = transform;
        self
    }

    /// Set custom break positions (data values where ticks appear).
    /// Aim for about `n` breaks (ggplot2's `n.breaks`, #408) instead of 5;
    /// the extended-Wilkinson search still picks round values.
    pub fn with_n_breaks(mut self, n: usize) -> Self {
        self.n_breaks = n.max(2);
        self
    }

    /// ggplot2's `scale_x_time()` / `scale_y_time()` (#408): the values are
    /// seconds (e.g. since midnight, `hms`), the breaks fall on round time
    /// steps (1 s … 1 h … 1 day) and the labels read `HH:MM:SS`.
    pub fn time() -> Self {
        let mut s = ScaleContinuous::new().with_label_formatter(super::util::format_hms);
        s.time = true;
        s
    }

    /// ggplot2's `scale_x_binned(n.breaks = n)` (#408): positions are cut
    /// into round bins between about `n` breaks (right-closed, like ggplot2) and every value
    /// sits at its bin's midpoint before the stat runs — so `geom_bar()`
    /// draws a histogram. The axis breaks are the bin edges. Explicit
    /// `with_limits` set the binned range.
    pub fn binned(n: usize) -> Self {
        let mut s = ScaleContinuous::new();
        s.binned = Some(n.max(1));
        s
    }

    /// ggplot2's `guide = guide_axis_logticks()` (#409): on a log10 axis,
    /// extra ticks at every 1–9 × 10ᵏ — long at the decades, mid at the 5s,
    /// short otherwise. No effect on other transforms.
    pub fn with_log_ticks(mut self) -> Self {
        self.log_ticks = true;
        self
    }

    pub fn with_breaks(mut self, breaks: Vec<f64>) -> Self {
        self.custom_breaks = Some(breaks);
        self
    }

    /// Set custom labels for breaks. Must match the number of breaks.
    pub fn with_labels(mut self, labels: Vec<String>) -> Self {
        self.custom_labels = Some(labels);
        self
    }

    /// Labels for the custom breaks that depend on the locale (month or
    /// weekday names): `f` gets the scale's locale — the plot's
    /// [`GGPlot::locale`](crate::plot::GGPlot::locale) unless set on the
    /// scale — or `None`. Takes precedence over [`with_labels`](Self::with_labels).
    pub fn with_localized_labels<F>(mut self, f: F) -> Self
    where
        F: Fn(Option<&crate::locale::Locale>) -> Vec<String> + Send + Sync + 'static,
    {
        self.localized_labels = Some(Arc::new(f));
        self
    }

    /// Set the expansion multiplier and additive constant.
    /// Like R's `expand = c(mult, add)`, applied symmetrically. Default `(0.05, 0.0)`.
    pub fn with_expand(mut self, mult: f64, add: f64) -> Self {
        self.expand = (mult, add, mult, add);
        self
    }

    /// Per-side expansion (R's `expansion(mult = c(l, u), add = c(l, u))`):
    /// separate multiplicative/additive expansion for the lower and upper ends.
    pub fn with_expand_sides(
        mut self,
        mult_lower: f64,
        add_lower: f64,
        mult_upper: f64,
        add_upper: f64,
    ) -> Self {
        self.expand = (mult_lower, add_lower, mult_upper, add_upper);
        self
    }

    /// Place this axis on the opposite side (x → top, y → right).
    pub fn with_position_opposite(mut self) -> Self {
        self.position_opposite = true;
        self
    }

    /// Set a label formatter. Accepts a plain `fn` (e.g. `label_comma`) or a
    /// configurable formatter such as `label_si()` / `label_number(...)`.
    pub fn with_label_formatter<F>(mut self, f: F) -> Self
    where
        F: Fn(f64) -> String + Send + Sync + 'static,
    {
        self.label_formatter = Some(std::sync::Arc::new(f));
        self
    }

    /// Set a formatter for [`format_value`](Scale::format_value) — a
    /// per-mark `data-fvalue` or tooltip-template value — distinct from
    /// [`with_label_formatter`](Self::with_label_formatter), which only
    /// affects axis break labels. Falls back to the label formatter, then
    /// the locale default, when unset.
    pub fn with_value_formatter<F>(mut self, f: F) -> Self
    where
        F: Fn(f64) -> String + Send + Sync + 'static,
    {
        self.value_formatter = Some(std::sync::Arc::new(f));
        self
    }

    /// Render the default break labels with `locale`'s decimal and thousands
    /// marks (`1.000`, `2,5` in de-DE). Overrides the plot locale
    /// ([`GGPlot::locale`](crate::GGPlot::locale)); a custom
    /// [`with_label_formatter`](Self::with_label_formatter) still wins.
    pub fn with_locale(mut self, locale: crate::locale::Locale) -> Self {
        self.locale = Some(locale);
        self
    }

    /// Add a secondary axis with a transformation function.
    pub fn with_sec_axis(mut self, sec: SecAxis) -> Self {
        self.sec_axis = Some(sec);
        self
    }

    /// Get the secondary axis, if any.
    pub fn sec_axis(&self) -> Option<&SecAxis> {
        self.sec_axis.as_ref()
    }

    fn format_label(&self, v: f64) -> String {
        if let Some(f) = &self.label_formatter {
            f(v)
        } else {
            super::util::format_number_opt(v, self.locale.as_ref())
        }
    }

    fn expanded_range(&self) -> (f64, f64) {
        let range = self.max - self.min;
        let (ml, al, mu, au) = self.expand;
        (self.min - range * ml - al, self.max + range * mu + au)
    }

    /// Whether this axis should be drawn on the opposite side (x → top, y → right).
    pub fn is_opposite(&self) -> bool {
        self.position_opposite
    }
}

impl Default for ScaleContinuous {
    fn default() -> Self {
        Self::new()
    }
}

impl Scale for ScaleContinuous {
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
        // The built data holds the value in the scale's transformed space
        // (e.g. `scale_y_log10` stores `log10(y)`), but `data-fvalue`/a
        // tooltip template is meant to be the human-readable number —
        // `y = 100` under log10, not `2` — the same inverse the break
        // labels already apply (`breaks()` below calls `format_label`
        // with `scale_transform.inverse(v)`, never raw `v`).
        let f = self.scale_transform.inverse(f);
        if !f.is_finite() {
            return None;
        }
        Some(match &self.value_formatter {
            Some(fmt) => fmt(f),
            None => self.format_label(f),
        })
    }

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
        self.trained = true;
    }

    fn map(&self, value: &Value) -> f64 {
        let f = match value.as_f64() {
            Some(f) => f,
            None => return 0.0,
        };
        // ±Inf means "panel edge" (ggplot2): -Inf → lower edge, +Inf → upper.
        if f == f64::INFINITY {
            return 1.0;
        }
        if f == f64::NEG_INFINITY {
            return 0.0;
        }
        // An untrained scale (empty data) has no domain — centre everything
        // rather than producing NaN.
        if !self.trained || !self.min.is_finite() || !self.max.is_finite() {
            return 0.5;
        }
        let (emin, emax) = self.expanded_range();
        let range = emax - emin;
        if range.abs() < f64::EPSILON {
            0.5
        } else {
            (f - emin) / range
        }
    }

    fn expanded_domain(&self) -> Option<(f64, f64)> {
        if !self.trained || !self.min.is_finite() || !self.max.is_finite() {
            return None;
        }
        let (a, b) = self.expanded_range();
        if !(a.is_finite() && b.is_finite()) {
            return None;
        }
        if (b - a).abs() < f64::EPSILON {
            // A degenerate domain maps everything to the panel centre.
            return Some((a - 0.5, b + 0.5));
        }
        Some((a, b))
    }

    fn breaks(&self) -> Vec<(f64, String)> {
        if !self.trained || self.min > self.max {
            return vec![];
        }

        // Use custom breaks if provided
        if let Some(ref custom) = self.custom_breaks {
            let localized = self
                .localized_labels
                .as_ref()
                .map(|f| f(self.locale.as_ref()));
            let custom_labels = localized.as_ref().or(self.custom_labels.as_ref());
            return custom
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    let pos = self.map(&Value::Float(v));
                    let label = if let Some(labels) = custom_labels {
                        labels
                            .get(i)
                            .cloned()
                            .unwrap_or_else(|| self.format_label(v))
                    } else {
                        self.format_label(self.scale_transform.inverse(v))
                    };
                    (pos, label)
                })
                .collect();
        }

        if super::util::is_degenerate_range(self.min, self.max) {
            let label = self.format_label(self.scale_transform.inverse(self.min));
            return vec![(0.5, label)];
        }

        // Extended-Wilkinson breaks over the data range (matching ggplot2's
        // scales::extended_breaks), keeping those within the expanded (visible)
        // range. Labels show the original (inverse-transformed) value.
        let (emin, emax) = self.expanded_range();
        if !self.bin_edges.is_empty() {
            let shown: Vec<f64> = self
                .bin_edges
                .iter()
                .map(|&v| self.scale_transform.inverse(v))
                .collect();
            let labels: Vec<String> = match &self.label_formatter {
                Some(f) => shown.iter().map(|&v| f(v)).collect(),
                None => super::util::format_numbers_opt(&shown, self.locale.as_ref()),
            };
            return self
                .bin_edges
                .iter()
                .zip(labels)
                .map(|(&v, l)| (self.map(&Value::Float(v)), l))
                .collect();
        }
        let tol = (emax - emin).abs() * 1e-9;
        let breaks: Vec<f64> = if self.scale_transform.uses_data_space_breaks() {
            // Log/sqrt axes: pick round ticks in *data* space, then forward-
            // transform them back onto the scale — not `extended_breaks`
            // directly on the transformed range, which gives "nice" numbers
            // in log/sqrt space that read as noise once inverse-transformed
            // (half-decades like 316.23 under log10, squares like 625 under
            // sqrt).
            let data_min = self.scale_transform.inverse(self.min);
            let data_max = self.scale_transform.inverse(self.max);
            if data_min.is_finite() && data_max.is_finite() && data_max > data_min {
                self.scale_transform
                    .breaks_in_data_space(data_min, data_max, self.n_breaks)
                    .into_iter()
                    .map(|d| self.scale_transform.apply(d))
                    .filter(|&v| v.is_finite() && v >= emin - tol && v <= emax + tol)
                    .collect()
            } else {
                vec![]
            }
        } else {
            if self.time {
                super::util::time_breaks(self.min, self.max, self.n_breaks)
            } else {
                extended_breaks(self.min, self.max, self.n_breaks)
            }
            .into_iter()
            .filter(|&v| v >= emin - tol && v <= emax + tol)
            .collect()
        };
        let shown: Vec<f64> = breaks
            .iter()
            .map(|&v| self.scale_transform.inverse(v))
            .collect();
        // Without a custom formatter, label the breaks jointly so neighbours
        // stay distinguishable on narrow ranges.
        let labels: Vec<String> = match &self.label_formatter {
            Some(f) => shown.iter().map(|&v| f(v)).collect(),
            None => super::util::format_numbers_opt(&shown, self.locale.as_ref()),
        };
        breaks
            .iter()
            .zip(labels)
            .map(|(&v, label)| (self.map(&Value::Float(v)), label))
            .collect()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    fn transform(&self, value: &Value) -> Value {
        self.scale_transform.transform_value(value)
    }

    fn set_log_ticks(&mut self, on: bool) {
        self.log_ticks = on;
    }

    fn log_ticks(&self) -> Vec<(f64, u8)> {
        if !self.log_ticks
            || !self.trained
            || !matches!(self.scale_transform, ScaleTransform::Log10)
        {
            return Vec::new();
        }
        let (lo, hi) = self.expanded_range();
        if !(lo.is_finite() && hi.is_finite()) || hi <= lo || hi - lo > 30.0 {
            return Vec::new();
        }
        let mut out = Vec::new();
        for k in (lo.floor() as i64)..=(hi.ceil() as i64) {
            for m in 1..=9u32 {
                let t = k as f64 + (m as f64).log10();
                if t >= lo - 1e-9 && t <= hi + 1e-9 {
                    let kind = match m {
                        1 => 0,
                        5 => 1,
                        _ => 2,
                    };
                    out.push((self.map(&Value::Float(t)), kind));
                }
            }
        }
        out
    }

    fn bin_positions(&mut self, values: &[Value]) -> Option<Vec<Value>> {
        let n = self.binned?;
        if self.bin_edges.is_empty() {
            let (lo, hi) = match self.filter_limits() {
                Some(l) => l,
                None => {
                    let fin = values
                        .iter()
                        .filter_map(Value::as_f64)
                        .filter(|f| f.is_finite());
                    let (lo, hi) = fin.fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), f| {
                        (a.min(f), b.max(f))
                    });
                    if lo > hi {
                        return None;
                    }
                    (lo, hi)
                }
            };
            let (lo, hi) = if hi > lo {
                (lo, hi)
            } else {
                (lo - 0.5, hi + 0.5)
            };
            let mut e = super::util::extended_breaks(lo, hi, n);
            if e.len() < 2 {
                e = vec![lo, hi];
            }
            let step = e[1] - e[0];
            while e[0] > lo + step * 1e-9 {
                e.insert(0, e[0] - step);
            }
            while *e.last().unwrap() < hi - step * 1e-9 {
                e.push(e.last().unwrap() + step);
            }
            // The axis spans whole bins.
            self.min = self.min.min(e[0]);
            self.max = self.max.max(*e.last().unwrap());
            self.trained = true;
            self.bin_edges = e;
        }
        let e = &self.bin_edges;
        Some(
            values
                .iter()
                .map(|v| match v.as_f64().filter(|f| f.is_finite()) {
                    Some(f) => {
                        // Right-closed bins (lo, hi]; the first includes lo.
                        let k = e.windows(2).position(|w| f <= w[1]).unwrap_or(e.len() - 2);
                        Value::Float((e[k] + e[k + 1]) / 2.0)
                    }
                    None => v.clone(),
                })
                .collect(),
        )
    }

    fn sec_axis(&self) -> Option<&SecAxis> {
        self.sec_axis.as_ref()
    }

    fn set_limits(&mut self, min: f64, max: f64) {
        self.min = min;
        self.max = max;
        self.trained = true;
    }

    fn set_expand(&mut self, mult: f64, add: f64) {
        self.expand = (mult, add, mult, add);
    }

    fn clear_expand(&mut self, lower: bool, upper: bool) {
        if lower {
            (self.expand.0, self.expand.1) = (0.0, 0.0);
        }
        if upper {
            (self.expand.2, self.expand.3) = (0.0, 0.0);
        }
    }

    fn filter_limits(&self) -> Option<(f64, f64)> {
        if self.filter_oob && self.trained {
            Some((self.min, self.max))
        } else {
            None
        }
    }

    fn domain(&self) -> Option<(f64, f64)> {
        if self.trained {
            Some((self.min, self.max))
        } else {
            None
        }
    }

    fn axis_position_opposite(&self) -> bool {
        self.position_opposite
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
