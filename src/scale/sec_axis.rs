/// A label formatter for a secondary axis' tick values.
pub type SecLabeller = std::sync::Arc<dyn Fn(f64) -> String + Send + Sync>;

/// A secondary axis specification — defines a transformation from the primary axis domain.
///
/// # Example
/// ```
/// use ggplot_rs::prelude::*;
///
/// // Temperature in Celsius on primary, Fahrenheit on secondary
/// let sec = SecAxis::new(|c| c * 9.0 / 5.0 + 32.0).with_name("Fahrenheit");
/// ```
#[derive(Clone)]
pub struct SecAxis {
    /// Transformation from primary axis values to secondary axis values.
    pub transform: fn(f64) -> f64,
    /// Name / title for the secondary axis.
    pub name: String,
    /// Custom break positions in primary axis coordinates.
    pub breaks: Option<Vec<f64>>,
    /// Label the axis as a percentage of the primary scale's trained data
    /// maximum ([`SecAxis::percent_of_max`]) instead of through `transform`.
    pub percent_of_max: bool,
    /// `secondary = primary * scale + offset` ([`SecAxis::linear`]): the
    /// axis gets its own nice ticks in the secondary range.
    pub linear: Option<(f64, f64)>,
    /// Formats the secondary axis' tick values ([`SecAxis::with_labeller`]);
    /// `None` uses the plot's number format.
    pub labeller: Option<SecLabeller>,
}

impl SecAxis {
    /// Create a secondary axis with the given transformation function.
    pub fn new(transform: fn(f64) -> f64) -> Self {
        SecAxis {
            transform,
            name: String::new(),
            breaks: None,
            percent_of_max: false,
            linear: None,
            labeller: None,
        }
    }

    /// A 0–100 % axis relative to the primary scale's trained data maximum
    /// (the value drawn at the right axis' 100 % tick) — e.g. a Pareto
    /// chart's cumulative share, where the maximum is the grand total. The
    /// ticks sit at their own nice percentages (0, 25, …, 100 %) rather than
    /// at the primary breaks.
    pub fn percent_of_max() -> Self {
        SecAxis {
            percent_of_max: true,
            ..SecAxis::new(|v| v)
        }
    }

    /// A linear secondary axis: `secondary = primary * scale + offset`,
    /// e.g. a second measure drawn rescaled into the primary range (a
    /// combo chart's right axis). Its ticks sit at nice values of the
    /// secondary range, not at the primary breaks. A zero or non-finite
    /// `scale` falls back to the identity.
    pub fn linear(scale: f64, offset: f64) -> Self {
        let ok = scale.is_finite() && scale != 0.0 && offset.is_finite();
        SecAxis {
            linear: Some(if ok { (scale, offset) } else { (1.0, 0.0) }),
            ..SecAxis::new(|v| v)
        }
    }

    /// Format the secondary axis' tick labels with `f` (e.g. a currency or
    /// percent format of the second measure).
    pub fn with_labeller(mut self, f: impl Fn(f64) -> String + Send + Sync + 'static) -> Self {
        self.labeller = Some(std::sync::Arc::new(f));
        self
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn with_breaks(mut self, breaks: Vec<f64>) -> Self {
        self.breaks = Some(breaks);
        self
    }

    /// Transform a primary axis value to the secondary axis value.
    pub fn transform_value(&self, v: f64) -> f64 {
        match self.linear {
            Some((a, b)) => v * a + b,
            None => (self.transform)(v),
        }
    }

    /// A tick label for a secondary value: the labeller, else the number
    /// format.
    pub(crate) fn label(&self, v: f64, locale: Option<&crate::locale::Locale>) -> String {
        match &self.labeller {
            Some(f) => f(v),
            None => crate::scale::util::format_number_opt(v, locale),
        }
    }

    /// The axis' own ticks as `(normalized position, label)` when it does not
    /// simply relabel the primary breaks (the [`percent_of_max`](Self::percent_of_max)
    /// mode); `None` otherwise.
    pub(crate) fn own_ticks(&self, primary: &dyn super::Scale) -> Option<Vec<(f64, String)>> {
        if let Some((a, b)) = self.linear {
            let (lo, hi) = primary.expanded_domain()?;
            let (s1, s2) = (lo * a + b, hi * a + b);
            let (smin, smax) = (s1.min(s2), s1.max(s2));
            let eps = 1e-9 * (smax - smin).abs().max(1e-12);
            return Some(
                super::util::extended_breaks(smin, smax, 5)
                    .into_iter()
                    .filter(|t| *t >= smin - eps && *t <= smax + eps)
                    .map(|t| {
                        let pos = primary.map(&crate::data::Value::Float((t - b) / a));
                        (pos, self.label(t, primary.locale()))
                    })
                    .collect(),
            );
        }
        if !self.percent_of_max {
            return None;
        }
        let (_, max) = primary.domain()?;
        let (lo, hi) = primary.expanded_domain()?;
        if !(max > 0.0 && max.is_finite()) {
            return Some(vec![]);
        }
        let (lo_pct, hi_pct) = (lo / max * 100.0, hi / max * 100.0);
        let eps = 1e-9 * (hi_pct - lo_pct).abs().max(1.0);
        Some(
            super::util::extended_breaks(0.0, 100.0, 5)
                .into_iter()
                .filter(|b| *b >= lo_pct - eps && *b <= hi_pct + eps)
                .map(|b| {
                    let pos = primary.map(&crate::data::Value::Float(b / 100.0 * max));
                    let label = crate::scale::util::format_number_opt(b, primary.locale());
                    (pos, format!("{label}%"))
                })
                .collect(),
        )
    }
}

impl std::fmt::Debug for SecAxis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecAxis")
            .field("name", &self.name)
            .field("breaks", &self.breaks)
            .field("percent_of_max", &self.percent_of_max)
            .field("linear", &self.linear)
            .field("labeller", &self.labeller.is_some())
            .finish()
    }
}
