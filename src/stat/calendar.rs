//! Calendar heatmap layout (`stat_calendar` / `GGPlot::geom_calendar`): a date
//! column becomes week-column × weekday-row tiles.

use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// Longest span laid out, in years. Older dates are clipped (with a build
/// warning from `geom_calendar`) so a stray 1900-01-01 cannot blow up the
/// layout or allocate thousands of month boundaries.
pub const MAX_CALENDAR_YEARS: i64 = 50;
const MAX_SPAN_DAYS: i64 = MAX_CALENDAR_YEARS * 366;

/// Column holding each cell's ISO date (`YYYY-MM-DD`); `geom_tile` uses it as
/// the hover key (`data-x` and tooltip) instead of the week/weekday position.
pub const DATE_KEY_COL: &str = ".key";

/// Days since 1970-01-01 for a date value: `DateTime` (epoch seconds), a
/// number (epoch seconds, as `DateTime` coerces), or an ISO `YYYY-MM-DD…`
/// string. `None` for anything else / out of range.
pub fn day_number(v: &Value) -> Option<i64> {
    const LIMIT: f64 = 1.0e14; // ~3 million years of seconds: plenty, no overflow
    match v {
        Value::DateTime(s) => Some(s.div_euclid(86_400)),
        Value::Integer(i) => Some(i.div_euclid(86_400)),
        Value::Float(f) if f.is_finite() && f.abs() < LIMIT => Some((f / 86_400.0).floor() as i64),
        Value::Str(s) => parse_iso_date(s),
        _ => None,
    }
}

fn parse_iso_date(s: &str) -> Option<i64> {
    let s = s.trim();
    let date = s.get(..10)?;
    let mut it = date.split('-');
    let y: i64 = it.next()?.parse().ok()?;
    let m: u32 = it.next()?.parse().ok()?;
    let d: u32 = it.next()?.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || !(-9999..=9999).contains(&y) {
        return None;
    }
    Some(days_from_civil(y, m, d))
}

/// Days since 1970-01-01 of a civil date (Hinnant).
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = m as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Civil `(year, month, day)` of a day number (Hinnant).
pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Weekday of a day number, 0-based from the week start (Sunday or Monday).
pub fn weekday(day: i64, monday_first: bool) -> i64 {
    // 1970-01-01 was a Thursday (Sunday-based 4, Monday-based 3).
    (day + if monday_first { 3 } else { 4 }).rem_euclid(7)
}

/// The calendar grid shared by [`StatCalendar`] and `geom_calendar`: which
/// day is column 0 / row 0, and the (clipped) first/last day.
#[derive(Clone, Copy, Debug)]
pub struct CalendarGrid {
    /// First day of week 0 (a Sunday, or a Monday when `monday_first`).
    pub origin: i64,
    pub first_day: i64,
    pub last_day: i64,
    pub monday_first: bool,
}

impl CalendarGrid {
    /// Grid over the given day numbers, clipped to the last
    /// [`MAX_CALENDAR_YEARS`] years. Returns the grid and whether it clipped.
    pub fn from_days(days: impl Iterator<Item = i64>, monday_first: bool) -> Option<(Self, bool)> {
        let (mut lo, mut hi) = (i64::MAX, i64::MIN);
        for d in days {
            lo = lo.min(d);
            hi = hi.max(d);
        }
        if lo > hi {
            return None;
        }
        let clipped = hi - lo > MAX_SPAN_DAYS;
        let lo = lo.max(hi - MAX_SPAN_DAYS);
        Some((
            CalendarGrid {
                origin: lo - weekday(lo, monday_first),
                first_day: lo,
                last_day: hi,
                monday_first,
            },
            clipped,
        ))
    }

    /// Week column of a day.
    pub fn column(&self, day: i64) -> i64 {
        (day - self.origin).div_euclid(7)
    }

    /// Row position of a day: the first weekday is the top row (y = 6).
    pub fn row(&self, day: i64) -> i64 {
        6 - weekday(day, self.monday_first)
    }

    pub fn n_weeks(&self) -> i64 {
        self.column(self.last_day) + 1
    }

    pub fn contains(&self, day: i64) -> bool {
        (self.first_day..=self.last_day).contains(&day)
    }
}

/// Lays a date `x` out as calendar cells: `x` = week column, `y` = weekday
/// row (first weekday on top, y = 6 … 0), unit `xmin`/`xmax`/`ymin`/`ymax`
/// extents, and the ISO date in [`DATE_KEY_COL`]. Other columns (`fill`,
/// `label`, …) pass through. Dates older than [`MAX_CALENDAR_YEARS`] before
/// the newest are dropped.
#[derive(Clone, Debug, Default)]
pub struct StatCalendar {
    /// Fix the grid (so separately computed groups/guides share it); `None`
    /// derives it from the group's own dates.
    pub grid: Option<CalendarGrid>,
    /// Weeks start on Monday instead of Sunday.
    pub monday_first: bool,
}

impl Stat for StatCalendar {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let Some(x) = data.column("x") else {
            return DataFrame::new();
        };
        let days: Vec<Option<i64>> = x.iter().map(day_number).collect();
        let grid = match self.grid {
            Some(g) => g,
            None => {
                match CalendarGrid::from_days(days.iter().flatten().copied(), self.monday_first) {
                    Some((g, _)) => g,
                    None => return DataFrame::new(),
                }
            }
        };
        let rows: Vec<(usize, i64)> = days
            .iter()
            .enumerate()
            .filter_map(|(i, d)| d.filter(|d| grid.contains(*d)).map(|d| (i, d)))
            .collect();
        let mut out = DataFrame::new();
        let col = |off: f64, row: bool| -> Vec<Value> {
            rows.iter()
                .map(|&(_, d)| {
                    let base = if row { grid.row(d) } else { grid.column(d) };
                    Value::Float(base as f64 + off)
                })
                .collect()
        };
        out.add_column("x".to_string(), col(0.0, false));
        out.add_column("y".to_string(), col(0.0, true));
        out.add_column("xmin".to_string(), col(-0.5, false));
        out.add_column("xmax".to_string(), col(0.5, false));
        out.add_column("ymin".to_string(), col(-0.5, true));
        out.add_column("ymax".to_string(), col(0.5, true));
        out.add_column(
            DATE_KEY_COL.to_string(),
            rows.iter()
                .map(|&(_, d)| {
                    let (y, m, dd) = civil_from_days(d);
                    Value::Str(format!("{y:04}-{m:02}-{dd:02}"))
                })
                .collect(),
        );
        for name in data.column_names() {
            if out.has_column(name) {
                continue;
            }
            if let Some(src) = data.column(name) {
                out.add_column(
                    name.to_string(),
                    rows.iter().map(|&(i, _)| src[i].clone()).collect(),
                );
            }
        }
        out
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X]
    }

    fn name(&self) -> &str {
        "calendar"
    }
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Month-start x breaks/labels for a calendar grid: the week column of each
/// month's first day. Over two years, only Januaries are labelled (with the
/// year); a multi-year range adds the year to each January.
pub fn month_breaks(grid: &CalendarGrid) -> (Vec<f64>, Vec<String>) {
    let starts = month_starts(grid);
    let (y0, _, _) = civil_from_days(grid.first_day);
    let (y1, _, _) = civil_from_days(grid.last_day);
    let many = starts.len() > 24;
    let mut breaks = Vec::new();
    let mut labels = Vec::new();
    for (day, y, m) in starts {
        if many && m != 1 {
            continue;
        }
        breaks.push(grid.column(day) as f64);
        labels.push(if many {
            y.to_string()
        } else if y0 != y1 && m == 1 {
            format!("{} {y}", MONTHS[0])
        } else {
            MONTHS[(m - 1) as usize].to_string()
        });
    }
    (breaks, labels)
}

/// First days of each month intersecting the grid (the first visible day for
/// the first month), as `(day, year, month)`.
fn month_starts(grid: &CalendarGrid) -> Vec<(i64, i64, u32)> {
    let (mut y, mut m, _) = civil_from_days(grid.first_day);
    let mut out = vec![(grid.first_day, y, m)];
    loop {
        (y, m) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
        let d = days_from_civil(y, m, 1);
        if d > grid.last_day {
            break;
        }
        out.push((d, y, m));
    }
    out
}

/// Month boundary segments `(x, y, xend, yend)` separating consecutive
/// months (GitHub/ECharts style step outlines), in calendar coordinates.
pub fn month_boundaries(grid: &CalendarGrid) -> Vec<(f64, f64, f64, f64)> {
    let mut segs = Vec::new();
    for (day, _, _) in month_starts(grid).into_iter().skip(1) {
        let c = grid.column(day) as f64;
        let w = weekday(day, grid.monday_first) as f64;
        let row_top = 6.5 - w;
        if w > 0.0 {
            segs.push((c + 0.5, 6.5, c + 0.5, row_top));
            segs.push((c + 0.5, row_top, c - 0.5, row_top));
        }
        segs.push((c - 0.5, row_top, c - 0.5, -0.5));
    }
    segs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_roundtrip() {
        for d in [-1_000_000, -1, 0, 1, 19_000, 2_000_000] {
            let (y, m, dd) = civil_from_days(d);
            assert_eq!(days_from_civil(y, m, dd), d);
        }
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(weekday(0, false), 4); // Thursday
        assert_eq!(weekday(0, true), 3);
    }

    #[test]
    fn huge_span_is_clipped() {
        let (g, clipped) =
            CalendarGrid::from_days([-5_000_000, 20_000].into_iter(), false).unwrap();
        assert!(clipped);
        assert!(g.n_weeks() <= MAX_SPAN_DAYS / 7 + 2);
        assert!(month_starts(&g).len() <= (MAX_CALENDAR_YEARS as usize + 1) * 12 + 1);
    }
}
