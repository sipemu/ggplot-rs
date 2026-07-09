use crate::aes::Aesthetic;
use crate::data::Value;

use super::Scale;

/// A break interval for a date/time axis.
#[derive(Clone, Copy, Debug)]
enum DateBreak {
    /// A fixed number of seconds (seconds/minutes/hours/days/weeks).
    Secs(f64),
    /// A number of whole calendar months (years = 12 × n).
    Months(u32),
}

/// Date/time scale — maps epoch seconds to [0, 1] and formats axis labels as dates.
#[derive(Clone, Debug)]
pub struct ScaleDateTime {
    aesthetic: Aesthetic,
    name: String,
    min: f64,
    max: f64,
    trained: bool,
    expand: (f64, f64),
    date_breaks: Option<DateBreak>,
    date_labels: Option<String>,
}

/// Decomposed UTC date/time.
struct DateParts {
    year: i64,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
}

/// Days since 1970-01-01 for a civil (Y, M, D) date — Howard Hinnant's algorithm.
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn secs_from_civil(y: i64, m: u32, d: u32) -> i64 {
    days_from_civil(y, m, d) * 86_400
}

/// Inverse of `days_from_civil`, plus the intra-day time.
fn civil_from_secs(secs: i64) -> DateParts {
    let (mut days, rem) = if secs >= 0 {
        (secs / 86_400, secs % 86_400)
    } else {
        let d = (secs - 86_400 + 1) / 86_400;
        (d, secs - d * 86_400)
    };
    let hour = (rem / 3600) as u32;
    let minute = ((rem % 3600) / 60) as u32;
    let second = (rem % 60) as u32;

    days += 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let doe = (days - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    DateParts {
        year,
        month,
        day,
        hour,
        minute,
        second,
    }
}

const MONTHS_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const MONTHS_LONG: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Format a timestamp with a strftime-style subset:
/// `%Y %y %m %b %B %d %e %H %M %S %%`.
fn strftime(secs: f64, fmt: &str) -> String {
    let p = civil_from_secs(secs as i64);
    let mi = (p.month.clamp(1, 12) - 1) as usize;
    let mut out = String::new();
    let mut chars = fmt.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('Y') => out.push_str(&format!("{:04}", p.year)),
            Some('y') => out.push_str(&format!("{:02}", p.year.rem_euclid(100))),
            Some('m') => out.push_str(&format!("{:02}", p.month)),
            Some('b') => out.push_str(MONTHS_SHORT[mi]),
            Some('B') => out.push_str(MONTHS_LONG[mi]),
            Some('d') => out.push_str(&format!("{:02}", p.day)),
            Some('e') => out.push_str(&format!("{:2}", p.day)),
            Some('H') => out.push_str(&format!("{:02}", p.hour)),
            Some('M') => out.push_str(&format!("{:02}", p.minute)),
            Some('S') => out.push_str(&format!("{:02}", p.second)),
            Some('%') => out.push('%'),
            Some(other) => {
                out.push('%');
                out.push(other);
            }
            None => out.push('%'),
        }
    }
    out
}

/// Parse an R-style break spec like "1 month", "3 months", "2 weeks", "1 year".
fn parse_date_break(spec: &str) -> Option<DateBreak> {
    let spec = spec.trim().to_lowercase();
    let mut parts = spec.split_whitespace();
    let first = parts.next()?;
    let (n, unit) = match first.parse::<f64>() {
        Ok(n) => (n, parts.next()?.to_string()),
        Err(_) => (1.0, first.to_string()),
    };
    let unit = unit.trim_end_matches('s');
    let secs = |s: f64| Some(DateBreak::Secs(n * s));
    match unit {
        "sec" | "second" => secs(1.0),
        "min" | "minute" => secs(60.0),
        "hour" => secs(3600.0),
        "day" => secs(86_400.0),
        "week" => secs(604_800.0),
        "month" => Some(DateBreak::Months(n.max(1.0) as u32)),
        "year" => Some(DateBreak::Months((n.max(1.0) as u32) * 12)),
        _ => None,
    }
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
            date_breaks: None,
            date_labels: None,
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

    /// Calendar-aware break interval, R-style: `"1 month"`, `"3 months"`,
    /// `"2 weeks"`, `"1 year"`, `"6 hours"`, … Unrecognised specs are ignored.
    pub fn with_date_breaks(mut self, spec: &str) -> Self {
        self.date_breaks = parse_date_break(spec);
        self
    }

    /// strftime-style label format, e.g. `"%b %Y"` or `"%Y-%m-%d"`.
    /// Supported: `%Y %y %m %b %B %d %e %H %M %S %%`.
    pub fn with_date_labels(mut self, fmt: &str) -> Self {
        self.date_labels = Some(fmt.to_string());
        self
    }

    fn label(&self, secs: f64, step: f64) -> String {
        match &self.date_labels {
            Some(fmt) => strftime(secs, fmt),
            None => Self::format_datetime(secs, step),
        }
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
            return vec![(0.5, self.label(self.min, 1.0))];
        }

        let (emin, emax) = self.expanded_range();

        // Calendar-month breaks snap to the first of the month.
        if let Some(DateBreak::Months(n)) = self.date_breaks {
            let n = n.max(1);
            let start = civil_from_secs(emin.ceil() as i64);
            let (mut y, mut m) = (start.year, start.month);
            // First-of-month boundary at or after emin.
            if (secs_from_civil(y, m, 1) as f64) < emin {
                m += 1;
                if m > 12 {
                    m = 1;
                    y += 1;
                }
            }
            let mut breaks = Vec::new();
            let mut guard = 0;
            loop {
                let secs = secs_from_civil(y, m, 1) as f64;
                if secs > emax + 1.0 || guard > 10_000 {
                    break;
                }
                breaks.push((self.map(&Value::Float(secs)), self.label(secs, 0.0)));
                m += n;
                while m > 12 {
                    m -= 12;
                    y += 1;
                }
                guard += 1;
            }
            return breaks;
        }

        let explicit_secs = matches!(self.date_breaks, Some(DateBreak::Secs(s)) if s > 0.0);
        let step = match self.date_breaks {
            Some(DateBreak::Secs(s)) if s > 0.0 => s,
            _ => Self::nice_datetime_step(range),
        };

        // For month-or-larger auto steps, snap breaks to calendar boundaries with
        // clean year / year-month labels (ggplot2-style) instead of fixed-second
        // timestamps that drift off midnight (e.g. "2012-01-01 12:00:00").
        const DAY: f64 = 86_400.0;
        const MONTH: f64 = 30.0 * DAY;
        const YEAR: f64 = 365.25 * DAY;
        if !explicit_secs && step >= 0.9 * YEAR {
            let n = ((step / YEAR).round() as i64).max(1);
            let mut y = civil_from_secs(emin.ceil() as i64).year.div_euclid(n) * n;
            while (secs_from_civil(y, 1, 1) as f64) < emin {
                y += n;
            }
            let mut breaks = Vec::new();
            while (secs_from_civil(y, 1, 1) as f64) <= emax + 1.0 {
                let secs = secs_from_civil(y, 1, 1) as f64;
                breaks.push((self.map(&Value::Float(secs)), format!("{y}")));
                y += n;
            }
            return breaks;
        }
        if !explicit_secs && step >= 0.9 * MONTH {
            let n = ((step / MONTH).round() as i64).max(1);
            let start = civil_from_secs(emin.ceil() as i64);
            let mut tm = start.year * 12 + (start.month as i64 - 1); // months since year 0
            while (secs_from_civil(tm.div_euclid(12), tm.rem_euclid(12) as u32 + 1, 1) as f64)
                < emin
            {
                tm += 1;
            }
            let mut breaks = Vec::new();
            let mut guard = 0;
            loop {
                let (y, m) = (tm.div_euclid(12), tm.rem_euclid(12) as u32 + 1);
                let secs = secs_from_civil(y, m, 1) as f64;
                if secs > emax + 1.0 || guard > 10_000 {
                    break;
                }
                breaks.push((self.map(&Value::Float(secs)), format!("{y:04}-{m:02}")));
                tm += n;
                guard += 1;
            }
            return breaks;
        }

        let start = (emin / step).ceil() * step;
        let mut breaks = Vec::new();
        let mut v = start;
        while v <= emax + step * 0.001 {
            breaks.push((self.map(&Value::Float(v)), self.label(v, step)));
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

    fn set_limits(&mut self, min: f64, max: f64) {
        self.min = min;
        self.max = max;
        self.trained = true;
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
    fn civil_roundtrip() {
        // 2021-03-15 12:30:45 UTC = 1615811445
        let p = civil_from_secs(1_615_811_445);
        assert_eq!((p.year, p.month, p.day), (2021, 3, 15));
        assert_eq!((p.hour, p.minute, p.second), (12, 30, 45));
        assert_eq!(secs_from_civil(2021, 3, 15), 1_615_766_400); // midnight
    }

    #[test]
    fn multiyear_default_breaks_are_clean_years() {
        // A multi-year range with no explicit date_breaks should snap to calendar
        // year boundaries and label with plain years (not drifting timestamps).
        let mut s = ScaleDateTime::new();
        s.train(&[
            Value::DateTime(secs_from_civil(2011, 2, 1)),
            Value::DateTime(secs_from_civil(2016, 4, 1)),
        ]);
        let labels: Vec<String> = s.breaks().into_iter().map(|(_, l)| l).collect();
        assert!(!labels.is_empty());
        for l in &labels {
            assert!(
                l.len() == 4 && l.chars().all(|c| c.is_ascii_digit()),
                "expected a bare year label, got {l:?}"
            );
        }
        assert!(labels.contains(&"2014".to_string()));
    }

    #[test]
    fn strftime_subset() {
        let s = 1_615_766_400.0; // 2021-03-15 00:00
        assert_eq!(strftime(s, "%Y-%m-%d"), "2021-03-15");
        assert_eq!(strftime(s, "%b %Y"), "Mar 2021");
        assert_eq!(strftime(s, "%B"), "March");
        assert_eq!(strftime(s, "100%%"), "100%");
    }

    #[test]
    fn parse_specs() {
        assert!(matches!(
            parse_date_break("1 month"),
            Some(DateBreak::Months(1))
        ));
        assert!(matches!(
            parse_date_break("3 months"),
            Some(DateBreak::Months(3))
        ));
        assert!(matches!(
            parse_date_break("1 year"),
            Some(DateBreak::Months(12))
        ));
        assert!(
            matches!(parse_date_break("2 weeks"), Some(DateBreak::Secs(s)) if s == 1_209_600.0)
        );
        assert!(matches!(
            parse_date_break("month"),
            Some(DateBreak::Months(1))
        ));
        assert!(parse_date_break("fortnight").is_none());
    }

    #[test]
    fn monthly_breaks_land_on_first_of_month() {
        let mut s = ScaleDateTime::new()
            .with_date_breaks("1 month")
            .with_date_labels("%Y-%m-%d");
        // Jan 10 2021 .. Apr 20 2021
        s.set_limits(
            secs_from_civil(2021, 1, 10) as f64,
            secs_from_civil(2021, 4, 20) as f64,
        );
        let labels: Vec<String> = s.breaks().into_iter().map(|(_, l)| l).collect();
        assert!(labels.iter().all(|l| l.ends_with("-01")), "{labels:?}");
        assert!(labels.contains(&"2021-02-01".to_string()));
    }
}
