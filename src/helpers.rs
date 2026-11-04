//! ggplot2's helper functions (#414): discretising a continuous variable
//! (`cut_interval`, `cut_number`, `cut_width`), `resolution`, and the
//! `stat_summary` summary functions (`mean_se`, `mean_cl_normal`,
//! `mean_cl_boot`, `mean_sdl`, `median_hilow`) as plain functions.

use crate::data::Value;
use crate::stat::summary::SummaryData;

/// A discretised variable: one interval label per input value (`Value::Na`
/// for missing / non-finite input) and the interval labels in order — use
/// `values` as a `group` / `fill` column and `levels` for the scale order.
#[derive(Clone, Debug, PartialEq)]
pub struct Cut {
    pub values: Vec<Value>,
    pub levels: Vec<String>,
}

impl From<Cut> for Vec<Value> {
    fn from(c: Cut) -> Self {
        c.values
    }
}

/// C's `%.{digits}g` (what R's `formatC(x, digits)` prints).
fn fmt_g(v: f64, digits: usize) -> String {
    if v == 0.0 {
        return "0".into();
    }
    let digits = digits.max(1);
    let sci = format!("{:.*e}", digits - 1, v);
    let (mant, exp) = sci.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    let trim = |s: &str| -> String {
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s.to_string()
        }
    };
    if exp < -4 || exp >= digits as i32 {
        let sign = if exp < 0 { '-' } else { '+' };
        format!("{}e{sign}{:02}", trim(mant), exp.abs())
    } else {
        let decimals = (digits as i32 - 1 - exp).max(0) as usize;
        trim(&format!("{v:.decimals$}"))
    }
}

/// R's `cut(x, breaks, include.lowest = TRUE, right)` with its default
/// `dig.lab = 3` labels.
fn cut(x: &[f64], breaks: &[f64], right: bool) -> Cut {
    let nb = breaks.len();
    if nb < 2 {
        return Cut {
            values: vec![Value::Na; x.len()],
            levels: Vec::new(),
        };
    }
    // R: widen dig.lab until neighbouring break labels differ.
    let mut labels: Vec<String> = Vec::new();
    for dig in 3..=12 {
        labels = breaks.iter().map(|b| fmt_g(*b, dig)).collect();
        if labels.windows(2).all(|w| w[0] != w[1]) {
            break;
        }
    }
    let levels: Vec<String> = (0..nb - 1)
        .map(|i| {
            let (lo, hi) = (&labels[i], &labels[i + 1]);
            match (right, i == 0, i == nb - 2) {
                (true, true, _) => format!("[{lo},{hi}]"),
                (true, false, _) => format!("({lo},{hi}]"),
                (false, _, true) => format!("[{lo},{hi}]"),
                (false, _, false) => format!("[{lo},{hi})"),
            }
        })
        .collect();
    let values = x
        .iter()
        .map(|&v| {
            if !v.is_finite() {
                return Value::Na;
            }
            let k = (0..nb - 1).find(|&i| {
                let (lo, hi) = (breaks[i], breaks[i + 1]);
                if right {
                    (v > lo || (i == 0 && v >= lo)) && v <= hi
                } else {
                    v >= lo && (v < hi || (i == nb - 2 && v <= hi))
                }
            });
            k.map_or(Value::Na, |k| Value::Str(levels[k].clone()))
        })
        .collect();
    Cut { values, levels }
}

fn finite_range(x: &[f64]) -> Option<(f64, f64)> {
    let f = x.iter().copied().filter(|v| v.is_finite());
    let lo = f.clone().fold(f64::INFINITY, f64::min);
    let hi = f.fold(f64::NEG_INFINITY, f64::max);
    lo.is_finite().then_some((lo, hi))
}

/// `n` intervals of equal width over the range of `x` — ggplot2's
/// `cut_interval(x, n)`.
pub fn cut_interval(x: &[f64], n: usize) -> Cut {
    let Some((lo, hi)) = finite_range(x) else {
        return cut(x, &[], true);
    };
    let n = n.max(1);
    let breaks: Vec<f64> = (0..=n)
        .map(|i| lo + (hi - lo) * i as f64 / n as f64)
        .collect();
    cut(x, &breaks, true)
}

/// Intervals of width `length` on a grid of multiples of `length` covering
/// `x` — ggplot2's `cut_interval(x, length = …)`.
pub fn cut_interval_length(x: &[f64], length: f64) -> Cut {
    let Some((lo, hi)) = finite_range(x) else {
        return cut(x, &[], true);
    };
    if !(length.is_finite() && length > 0.0) {
        return cut(x, &[], true);
    }
    // scales::fullseq
    let breaks: Vec<f64> = if hi - lo < 1e-12 {
        vec![lo - length / 2.0, lo + length / 2.0]
    } else {
        let (a, b) = ((lo / length).floor(), (hi / length).ceil());
        (0..=(b - a) as i64)
            .map(|k| (a + k as f64) * length)
            .collect()
    };
    cut(x, &breaks, true)
}

/// `n` intervals with (about) equal numbers of observations — ggplot2's
/// `cut_number(x, n)` (R type-7 quantile breaks).
pub fn cut_number(x: &[f64], n: usize) -> Cut {
    let mut s: Vec<f64> = x.iter().copied().filter(|v| v.is_finite()).collect();
    if s.is_empty() {
        return cut(x, &[], true);
    }
    s.sort_by(f64::total_cmp);
    let n = n.max(1);
    let q = |p: f64| {
        let h = (s.len() - 1) as f64 * p;
        let (a, b) = (h.floor() as usize, h.ceil() as usize);
        s[a] + (h - a as f64) * (s[b] - s[a])
    };
    let breaks: Vec<f64> = (0..=n).map(|i| q(i as f64 / n as f64)).collect();
    cut(x, &breaks, true)
}

/// Options for [`cut_width`]: at most one of `center` / `boundary`, and
/// which side of each interval is closed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CutWidth {
    /// Centre of one bin (ggplot2's `center`).
    pub center: Option<f64>,
    /// An edge of one bin (ggplot2's `boundary`; default `width / 2`).
    pub boundary: Option<f64>,
    /// Close intervals on the left (`closed = "left"`); default right.
    pub closed_left: bool,
}

/// Intervals of width `width` — ggplot2's `cut_width(x, width, center,
/// boundary, closed)`.
pub fn cut_width(x: &[f64], width: f64, opts: CutWidth) -> Cut {
    let Some((lo, _)) = finite_range(x) else {
        return cut(x, &[], true);
    };
    if !(width.is_finite() && width > 0.0) {
        return cut(x, &[], true);
    }
    let boundary = match (opts.boundary, opts.center) {
        (Some(b), _) => b,
        (None, Some(c)) => c - width / 2.0,
        (None, None) => width / 2.0,
    };
    let min_x = boundary + ((lo - boundary) / width).floor() * width;
    let max_x = x
        .iter()
        .copied()
        .filter(|v| v.is_finite())
        .fold(f64::NEG_INFINITY, f64::max)
        + (1.0 - 1e-8) * width;
    let mut breaks = Vec::new();
    let mut b = min_x;
    while b <= max_x + 1e-12 {
        breaks.push(b);
        b = min_x + breaks.len() as f64 * width;
    }
    cut(x, &breaks, !opts.closed_left)
}

/// The smallest non-zero gap between the (unique) values of `x` — ggplot2's
/// `resolution(x, zero)`; `zero = true` also counts the distance to 0.
/// Constant data → 1.
pub fn resolution(x: &[f64], zero: bool) -> f64 {
    let mut v: Vec<f64> = x.iter().copied().filter(|v| v.is_finite()).collect();
    let Some((lo, hi)) = finite_range(&v) else {
        return 1.0;
    };
    if hi - lo < 1e-12 * hi.abs().max(1.0) {
        return 1.0;
    }
    if zero {
        v.push(0.0);
    }
    v.sort_by(f64::total_cmp);
    v.dedup();
    let tol = f64::EPSILON.sqrt();
    v.windows(2)
        .map(|w| w[1] - w[0])
        .filter(|d| *d > tol)
        .fold(f64::INFINITY, f64::min)
        .min(f64::MAX)
}

/// `(y, ymin, ymax)` = mean ± `mult` standard errors (`mean_se(x, mult)`).
pub fn mean_se(x: &[f64], mult: f64) -> (f64, f64, f64) {
    let (m, lo, _) = SummaryData::MeanSe.apply3(x);
    let se = (m - lo) * mult;
    (m, m - se, m + se)
}

/// Mean with a Student-t confidence interval (`mean_cl_normal(x, conf.int)`).
pub fn mean_cl_normal(x: &[f64], conf_int: f64) -> (f64, f64, f64) {
    SummaryData::MeanClNormal { level: conf_int }.apply3(x)
}

/// Mean with a percentile-bootstrap interval from `b` resamples
/// (`mean_cl_boot(x, conf.int, B)`; seeded, so reproducible — but not R's
/// random stream).
pub fn mean_cl_boot(x: &[f64], conf_int: f64, b: usize) -> (f64, f64, f64) {
    SummaryData::MeanClBoot { level: conf_int, b }.apply3(x)
}

/// Mean ± `mult` standard deviations (`mean_sdl(x, mult)`, default 2).
pub fn mean_sdl(x: &[f64], mult: f64) -> (f64, f64, f64) {
    SummaryData::MeanSdl { mult }.apply3(x)
}

/// Median with the outer `(1 ± conf_int) / 2` sample quantiles
/// (`median_hilow(x, conf.int)`).
pub fn median_hilow(x: &[f64], conf_int: f64) -> (f64, f64, f64) {
    SummaryData::MedianHilow { level: conf_int }.apply3(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    const X: [f64; 10] = [0.5, 1.2, 3.3, 4.7, 5.5, 7.25, 8.1, 9.9, 10.0, f64::NAN];

    fn labels(c: &Cut) -> Vec<String> {
        c.values
            .iter()
            .map(|v| v.as_str().map_or("NA".to_string(), str::to_string))
            .collect()
    }

    fn r(s: &str) -> Vec<String> {
        s.split(", ")
            .map(|t| t.trim_matches('"').to_string())
            .collect()
    }

    // Expected values: ggplot2 4.0.3 / R 4.6.0, printed with as.character().
    #[test]
    fn cut_labels_match_ggplot2() {
        assert_eq!(
            labels(&cut_interval(&X, 3)),
            r(
                r#""[0.5,3.67]", "[0.5,3.67]", "[0.5,3.67]", "(3.67,6.83]", "(3.67,6.83]", "(6.83,10]", "(6.83,10]", "(6.83,10]", "(6.83,10]", "NA""#
            )
        );
        assert_eq!(
            labels(&cut_interval_length(&X, 2.5)),
            r(
                r#""[0,2.5]", "[0,2.5]", "(2.5,5]", "(2.5,5]", "(5,7.5]", "(5,7.5]", "(7.5,10]", "(7.5,10]", "(7.5,10]", "NA""#
            )
        );
        assert_eq!(
            labels(&cut_number(&X, 3)),
            r(
                r#""[0.5,4.23]", "[0.5,4.23]", "[0.5,4.23]", "(4.23,7.53]", "(4.23,7.53]", "(4.23,7.53]", "(7.53,10]", "(7.53,10]", "(7.53,10]", "NA""#
            )
        );
        assert_eq!(
            labels(&cut_width(&X, 2.0, CutWidth::default())),
            r(
                r#""[-1,1]", "(1,3]", "(3,5]", "(3,5]", "(5,7]", "(7,9]", "(7,9]", "(9,11]", "(9,11]", "NA""#
            )
        );
        let b0 = CutWidth {
            boundary: Some(0.0),
            ..Default::default()
        };
        assert_eq!(
            labels(&cut_width(&X, 2.0, b0)),
            r(
                r#""[0,2]", "[0,2]", "(2,4]", "(4,6]", "(4,6]", "(6,8]", "(8,10]", "(8,10]", "(8,10]", "NA""#
            )
        );
        let left = CutWidth {
            center: Some(0.0),
            closed_left: true,
            ..Default::default()
        };
        assert_eq!(
            labels(&cut_width(&X, 2.0, left)),
            r(
                r#""[-1,1)", "[1,3)", "[3,5)", "[3,5)", "[5,7)", "[7,9)", "[7,9)", "[9,11]", "[9,11]", "NA""#
            )
        );
        assert_eq!(
            labels(&cut_interval(&[1000.0, 2500.0, 12345.0], 2)),
            r(r#""[1e+03,6.67e+03]", "[1e+03,6.67e+03]", "(6.67e+03,1.23e+04]""#)
        );
        assert_eq!(
            cut_interval(&X, 3).levels,
            ["[0.5,3.67]", "(3.67,6.83]", "(6.83,10]"]
        );
    }

    #[test]
    fn resolution_matches_ggplot2() {
        assert_eq!(resolution(&[1.0, 1.5, 3.0, 4.0], true), 0.5);
        assert_eq!(resolution(&[2.5, 3.5, 4.75], true), 1.0);
        assert_eq!(resolution(&[1.0, 2.0, 3.0, 4.0, 5.0], true), 1.0);
        assert_eq!(resolution(&[10.0, 20.0, 30.0], false), 10.0);
    }

    #[test]
    fn summary_functions_match_ggplot2() {
        let y = [2.1, 3.4, 1.9, 5.6, 4.4, 3.3, 2.8];
        let close = |got: (f64, f64, f64), want: (f64, f64, f64)| {
            for (a, b) in [(got.0, want.0), (got.1, want.1), (got.2, want.2)] {
                assert!((a - b).abs() < 1e-5, "{got:?} vs {want:?}");
            }
        };
        close(mean_se(&y, 1.0), (3.357143, 2.865858, 3.848427));
        close(mean_se(&y, 2.0), (3.357143, 2.374574, 4.339712));
        close(mean_cl_normal(&y, 0.95), (3.357143, 2.155013, 4.559273));
        close(mean_cl_normal(&y, 0.9), (3.357143, 2.402488, 4.311797));
        close(mean_sdl(&y, 2.0), (3.357143, 0.7575092, 5.956777));
        close(mean_sdl(&y, 1.0), (3.357143, 2.057326, 4.65696));
        close(median_hilow(&y, 0.95), (3.3, 1.93, 5.42));
        close(median_hilow(&y, 0.5), (3.3, 2.45, 3.9));
        let (m, lo, hi) = mean_cl_boot(&y, 0.95, 1000);
        assert!((m - 3.357143).abs() < 1e-5 && lo < m && hi > m);
    }
}
