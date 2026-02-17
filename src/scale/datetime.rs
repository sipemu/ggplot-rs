use crate::aes::Aesthetic;
use crate::data::Value;

use super::Scale;

/// Date/time scale — maps epoch seconds to [0, 1] and formats axis labels as dates.
#[derive(Clone, Debug)]
pub struct ScaleDateTime {
    aesthetic: Aesthetic,
    name: String,
    min: f64,
    max: f64,
    trained: bool,
    expand: (f64, f64),
}

impl ScaleDateTime {
    pub fn new() -> Self {
        ScaleDateTime {
            aesthetic: Aesthetic::X,
            name: String::new(),
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
            trained: false,
            expand: (0.05, 0.0),
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

    fn expanded_range(&self) -> (f64, f64) {
        let range = self.max - self.min;
        let mult = self.expand.0;
        let add = self.expand.1;
        (self.min - range * mult - add, self.max + range * mult + add)
    }

    /// Choose a "nice" step size in seconds for date/time breaks.
    fn nice_datetime_step(range_secs: f64) -> f64 {
        const MINUTE: f64 = 60.0;
        const HOUR: f64 = 3600.0;
        const DAY: f64 = 86400.0;
        const WEEK: f64 = 7.0 * DAY;
        const MONTH: f64 = 30.0 * DAY;
        const YEAR: f64 = 365.25 * DAY;

        let candidates = [
            1.0,
            5.0,
            10.0,
            30.0,
            MINUTE,
            5.0 * MINUTE,
            10.0 * MINUTE,
            30.0 * MINUTE,
            HOUR,
            3.0 * HOUR,
            6.0 * HOUR,
            12.0 * HOUR,
            DAY,
            2.0 * DAY,
            WEEK,
            2.0 * WEEK,
            MONTH,
            3.0 * MONTH,
            6.0 * MONTH,
            YEAR,
            2.0 * YEAR,
            5.0 * YEAR,
            10.0 * YEAR,
            20.0 * YEAR,
            50.0 * YEAR,
            100.0 * YEAR,
        ];

        let target = range_secs / 5.0;
        for &c in &candidates {
            if c >= target {
                return c;
            }
        }
        // For very large ranges, use multiples of 100 years
        let n = (target / (100.0 * YEAR)).ceil();
        n * 100.0 * YEAR
    }

    /// Format a timestamp (epoch seconds) as a human-readable label,
    /// adapting precision to the break step size.
    fn format_datetime(secs: f64, _step: f64) -> String {
        let epoch_secs = secs as i64;
        crate::data::format_epoch_secs(epoch_secs)
    }
}

impl Default for ScaleDateTime {
    fn default() -> Self {
        Self::new()
    }
}

impl Scale for ScaleDateTime {
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
        let (emin, emax) = self.expanded_range();
        let range = emax - emin;
        if range.abs() < f64::EPSILON {
            0.5
        } else {
            (f - emin) / range
        }
    }

    fn breaks(&self) -> Vec<(f64, String)> {
        if !self.trained || self.min > self.max {
            return vec![];
        }

        let range = self.max - self.min;
        if range.abs() < f64::EPSILON {
            let label = Self::format_datetime(self.min, 1.0);
            return vec![(0.5, label)];
        }

        let (emin, emax) = self.expanded_range();
        let step = Self::nice_datetime_step(range);

        let start = (emin / step).ceil() * step;
        let mut breaks = Vec::new();
        let mut v = start;
        while v <= emax + step * 0.001 {
            let pos = self.map(&Value::Float(v));
            let label = Self::format_datetime(v, step);
            breaks.push((pos, label));
            v += step;
        }
        breaks
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }
}
