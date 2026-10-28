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
}

impl SecAxis {
    /// Create a secondary axis with the given transformation function.
    pub fn new(transform: fn(f64) -> f64) -> Self {
        SecAxis {
            transform,
            name: String::new(),
            breaks: None,
            percent_of_max: false,
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
        (self.transform)(v)
    }

    /// The axis' own ticks as `(normalized position, label)` when it does not
    /// simply relabel the primary breaks (the [`percent_of_max`](Self::percent_of_max)
    /// mode); `None` otherwise.
    pub(crate) fn own_ticks(&self, primary: &dyn super::Scale) -> Option<Vec<(f64, String)>> {
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
            .finish()
    }
}
