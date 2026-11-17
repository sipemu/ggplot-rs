//! Shared drawing helpers for geoms: row grouping, dodge-aware x mapping and
//! polyline clipping to the panel (the native SVG backend has no clip paths,
//! so curves that can leave the panel are clipped geometrically).

use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::position::DODGE_OFFSET_COL;
use crate::scale::{Scale, ScaleSet};

/// Split row indices into drawing groups keyed on `group`, `color`, `fill`
/// and `linetype` (whichever exist), in first-appearance order. A frame
/// without any of these columns is one group.
pub(crate) fn row_groups(data: &DataFrame) -> Vec<Vec<usize>> {
    let cols: Vec<&[Value]> = ["group", "color", "fill", "linetype"]
        .iter()
        .filter_map(|c| data.column(c))
        .collect();
    let n = data.nrows();
    if cols.is_empty() {
        return if n == 0 {
            vec![]
        } else {
            vec![(0..n).collect()]
        };
    }
    let mut groups: indexmap::IndexMap<Vec<crate::data::GroupKey<'_>>, Vec<usize>> =
        indexmap::IndexMap::new();
    for i in 0..n {
        let key = cols.iter().map(|c| c[i].group_key()).collect();
        groups.entry(key).or_default().push(i);
    }
    groups.into_values().collect()
}

/// Maps a row's x to a normalized panel position, honouring a dodge offset
/// that a position adjustment stored for a *discrete* x axis (in category
/// units; one category = `1 / n_levels` of the panel).
pub(crate) struct XMapper<'a> {
    scale: Option<&'a dyn Scale>,
    offsets: Option<&'a [Value]>,
    counts: Option<&'a [Value]>,
    band: f64,
}

impl<'a> XMapper<'a> {
    pub(crate) fn new(data: &'a DataFrame, scales: &'a ScaleSet) -> Self {
        let scale = scales.get(&Aesthetic::X);
        let offsets = data.column(DODGE_OFFSET_COL);
        let band = match (scale, offsets) {
            (Some(s), Some(_)) if s.is_discrete() => 1.0 / s.breaks().len().max(1) as f64,
            _ => 0.0,
        };
        XMapper {
            scale,
            offsets,
            counts: data.column(crate::position::DODGE_N_COL),
            band,
        }
    }

    /// How many groups share row `i`'s slot on a dodged *discrete* axis
    /// (1 otherwise): slot-filling geoms divide their element width by it.
    pub(crate) fn slot_n(&self, i: usize) -> f64 {
        if self.band == 0.0 {
            return 1.0;
        }
        self.counts
            .and_then(|c| c.get(i))
            .and_then(|v| v.as_f64())
            .filter(|n| n.is_finite() && *n >= 1.0)
            .unwrap_or(1.0)
    }

    /// Normalized x of `value` at row `i` (the dodge offset of row `i` added).
    pub(crate) fn map(&self, value: &Value, i: usize) -> f64 {
        let base = self.scale.map(|s| s.map(value)).unwrap_or(0.0);
        let off = self
            .offsets
            .and_then(|o| o.get(i))
            .and_then(|v| v.as_f64())
            .filter(|v| v.is_finite())
            .unwrap_or(0.0);
        base + off * self.band
    }
}

/// Hover metadata for an interval mark (errorbar / linerange / pointrange):
/// `data-x` = the x key, `data-series`, `data-value` = the point estimate
/// `y` when present, else `"ymin ymax"`; tooltip `"<series>: <x>: y [lo, hi]"`.
pub(crate) fn mark_interval(
    backend: &mut dyn crate::render::backend::DrawBackend,
    scales: &crate::scale::ScaleSet,
    data: &DataFrame,
    i: usize,
    x: &Value,
    lo: &Value,
    hi: &Value,
) {
    let series = super::series_key(data, i);
    let y = data.column("y").and_then(|c| c.get(i));
    let range = format!("[{}, {}]", super::tip_value(lo), super::tip_value(hi));
    let body = match y {
        Some(y) => format!("{}: {} {range}", super::tip_value(x), super::tip_value(y)),
        None => format!("{}: {range}", super::tip_value(x)),
    };
    let tip = Some(match &series {
        Some(s) => format!("{s}: {body}"),
        None => body,
    });
    let value = match y {
        Some(y) => super::raw_value(y),
        None => super::raw_value(lo)
            .zip(super::raw_value(hi))
            .map(|(a, b)| format!("{a} {b}")),
    };
    super::set_mark(backend, tip, Some(super::tip_value(x)), series, value);
    // A [lo, hi] range (no single `y`) isn't a single raw value
    // `Scale::format_value` can take, so `data-fvalue` only applies when a
    // plain `y` exists.
    super::set_mark_formatted(backend, scales, Some(x), &crate::aes::Aesthetic::Y, y);
}

/// Map a numeric position through a position scale. A numeric value on a
/// *discrete* scale is read as ggplot2 does — category `k` sits at `k`
/// (1-based), so `1.5` falls between the first two categories.
pub(crate) fn map_position(scale: Option<&dyn Scale>, value: &Value) -> Option<f64> {
    let s = scale?;
    if s.is_discrete() {
        if let Some(f) = value.as_f64() {
            let n = s.breaks().len().max(1) as f64;
            return Some((f - 0.5) / n);
        }
    }
    Some(s.map(value))
}

/// Clip the segment `a → b` to the unit square (Liang–Barsky). Returns the
/// visible part, or `None` when the segment lies entirely outside.
pub(crate) fn clip_segment_unit(a: (f64, f64), b: (f64, f64)) -> Option<((f64, f64), (f64, f64))> {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let mut t0: f64 = 0.0;
    let mut t1: f64 = 1.0;
    for (p, q) in [(-dx, a.0), (dx, 1.0 - a.0), (-dy, a.1), (dy, 1.0 - a.1)] {
        if p.abs() < 1e-15 {
            if q < 0.0 {
                return None;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                t0 = t0.max(r);
            } else {
                t1 = t1.min(r);
            }
        }
    }
    if t0 > t1 || !(t0.is_finite() && t1.is_finite()) {
        return None;
    }
    Some((
        (a.0 + t0 * dx, a.1 + t0 * dy),
        (a.0 + t1 * dx, a.1 + t1 * dy),
    ))
}

/// Clip a polyline in normalized panel space to the unit square, returning
/// the visible runs (each with ≥ 2 points). Non-finite vertices break the
/// line.
pub(crate) fn clip_polyline_unit(pts: &[(f64, f64)]) -> Vec<Vec<(f64, f64)>> {
    let mut runs: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut cur: Vec<(f64, f64)> = Vec::new();
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let finite = a.0.is_finite() && a.1.is_finite() && b.0.is_finite() && b.1.is_finite();
        match clip_segment_unit(a, b).filter(|_| finite) {
            Some((p, q)) => {
                let joins = cur
                    .last()
                    .is_some_and(|l| (l.0 - p.0).abs() < 1e-12 && (l.1 - p.1).abs() < 1e-12);
                if !joins {
                    if cur.len() >= 2 {
                        runs.push(std::mem::take(&mut cur));
                    }
                    cur.clear();
                    cur.push(p);
                }
                cur.push(q);
            }
            None => {
                if cur.len() >= 2 {
                    runs.push(std::mem::take(&mut cur));
                }
                cur.clear();
            }
        }
    }
    if cur.len() >= 2 {
        runs.push(cur);
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_inside_segment_is_unchanged() {
        let (p, q) = clip_segment_unit((0.1, 0.2), (0.8, 0.9)).unwrap();
        for (got, want) in [(p.0, 0.1), (p.1, 0.2), (q.0, 0.8), (q.1, 0.9)] {
            assert!((got - want).abs() < 1e-12);
        }
    }

    #[test]
    fn clip_crossing_segment() {
        let (p, q) = clip_segment_unit((-1.0, 0.5), (2.0, 0.5)).unwrap();
        assert!((p.0 - 0.0).abs() < 1e-12 && (q.0 - 1.0).abs() < 1e-12);
        assert!(clip_segment_unit((-1.0, 2.0), (2.0, 2.0)).is_none());
    }

    #[test]
    fn clip_polyline_splits_runs() {
        // In, out (above), in again → two runs.
        let pts = [(0.1, 0.5), (0.3, 0.5), (0.5, 1.5), (0.7, 0.5), (0.9, 0.5)];
        let runs = clip_polyline_unit(&pts);
        assert_eq!(runs.len(), 2);
        for r in &runs {
            assert!(r
                .iter()
                .all(|(x, y)| (0.0..=1.0).contains(x) && (0.0..=1.0).contains(y)));
        }
    }

    #[test]
    fn clip_polyline_breaks_on_nan() {
        let pts = [
            (0.1, 0.1),
            (0.2, 0.2),
            (f64::NAN, 0.3),
            (0.4, 0.4),
            (0.5, 0.5),
        ];
        let runs = clip_polyline_unit(&pts);
        assert_eq!(runs.len(), 2);
    }

    #[test]
    fn row_groups_by_colour() {
        let mut df = DataFrame::new();
        df.add_column(
            "color".into(),
            vec!["a", "b", "a"]
                .into_iter()
                .map(|s| Value::Str(s.into()))
                .collect(),
        );
        assert_eq!(row_groups(&df), vec![vec![0, 2], vec![1]]);
        let mut plain = DataFrame::new();
        plain.add_column("x".into(), vec![Value::Float(1.0); 3]);
        assert_eq!(row_groups(&plain), vec![vec![0, 1, 2]]);
    }
}

/// How a line/area/ribbon joins consecutive points (Highcharts' `spline`
/// series, d3-shape's curves).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Interpolate {
    /// Straight segments.
    #[default]
    Linear,
    /// Monotone cubic in x (d3's `curveMonotoneX`): a smooth curve through
    /// every point that never overshoots between them. Points must be
    /// ordered by x.
    MonotoneCubic,
    /// Uniform Catmull–Rom spline through every point (may overshoot).
    CatmullRom,
}

/// Samples per original segment when densifying a curve.
const CURVE_STEPS: usize = 16;

/// Densify `points` (normalized panel coords) along `method`. Returns the
/// new points and, for each, the index of the input segment it lies on (so
/// per-vertex attributes such as linewidth can follow). Non-finite points
/// are kept as breaks: each finite run is interpolated on its own.
pub(crate) fn interpolate(
    points: &[(f64, f64)],
    method: Interpolate,
) -> (Vec<(f64, f64)>, Vec<usize>) {
    if method == Interpolate::Linear || points.len() < 3 {
        return (points.to_vec(), (0..points.len()).collect());
    }
    let mut out = Vec::with_capacity(points.len() * CURVE_STEPS);
    let mut origin = Vec::with_capacity(points.len() * CURVE_STEPS);
    let finite = |p: &(f64, f64)| p.0.is_finite() && p.1.is_finite();
    let mut start = 0;
    while start < points.len() {
        if !finite(&points[start]) {
            out.push(points[start]);
            origin.push(start);
            start += 1;
            continue;
        }
        let mut end = start;
        while end < points.len() && finite(&points[end]) {
            end += 1;
        }
        let run = &points[start..end];
        let curve = match method {
            Interpolate::MonotoneCubic => monotone_run(run),
            _ => catmull_rom_run(run),
        };
        for (p, k) in curve {
            out.push(p);
            origin.push(start + k);
        }
        start = end;
    }
    (out, origin)
}

/// d3-shape's `slope3`: the monotone tangent at the middle of three points.
fn slope3(p0: (f64, f64), p1: (f64, f64), p2: (f64, f64)) -> f64 {
    let (h0, h1) = (p1.0 - p0.0, p2.0 - p1.0);
    if h0 <= 0.0 || h1 <= 0.0 {
        return 0.0;
    }
    let (s0, s1) = ((p1.1 - p0.1) / h0, (p2.1 - p1.1) / h1);
    let p = (s0 * h1 + s1 * h0) / (h0 + h1);
    (s0.signum() + s1.signum()) * s0.abs().min(s1.abs()).min(0.5 * p.abs())
}

fn monotone_run(run: &[(f64, f64)]) -> Vec<((f64, f64), usize)> {
    let n = run.len();
    if n < 3 {
        return run.iter().enumerate().map(|(k, &p)| (p, k)).collect();
    }
    let mut m = vec![0.0; n];
    for k in 1..n - 1 {
        m[k] = slope3(run[k - 1], run[k], run[k + 1]);
    }
    // d3's slope2 at the ends: the secant pulled toward the inner tangent.
    let end_slope = |a: (f64, f64), b: (f64, f64), t: f64| {
        let h = b.0 - a.0;
        if h > 0.0 {
            (3.0 * (b.1 - a.1) / h - t) / 2.0
        } else {
            t
        }
    };
    m[0] = end_slope(run[0], run[1], m[1]);
    m[n - 1] = end_slope(run[n - 2], run[n - 1], m[n - 2]);
    let mut out = vec![(run[0], 0)];
    for k in 0..n - 1 {
        let ((x0, y0), (x1, y1)) = (run[k], run[k + 1]);
        let h = x1 - x0;
        if h <= 0.0 {
            out.push((run[k + 1], k));
            continue;
        }
        for s in 1..=CURVE_STEPS {
            let t = s as f64 / CURVE_STEPS as f64;
            let (t2, t3) = (t * t, t * t * t);
            let y = (2.0 * t3 - 3.0 * t2 + 1.0) * y0
                + (t3 - 2.0 * t2 + t) * h * m[k]
                + (-2.0 * t3 + 3.0 * t2) * y1
                + (t3 - t2) * h * m[k + 1];
            out.push(((x0 + h * t, y), k));
        }
    }
    out
}

fn catmull_rom_run(run: &[(f64, f64)]) -> Vec<((f64, f64), usize)> {
    let n = run.len();
    if n < 3 {
        return run.iter().enumerate().map(|(k, &p)| (p, k)).collect();
    }
    let at = |i: isize| run[i.clamp(0, n as isize - 1) as usize];
    let mut out = vec![(run[0], 0)];
    for k in 0..n - 1 {
        let (p0, p1, p2, p3) = (
            at(k as isize - 1),
            at(k as isize),
            at(k as isize + 1),
            at(k as isize + 2),
        );
        for s in 1..=CURVE_STEPS {
            let t = s as f64 / CURVE_STEPS as f64;
            let (t2, t3) = (t * t, t * t * t);
            let c = |a: f64, b: f64, c: f64, d: f64| {
                0.5 * (2.0 * b
                    + (-a + c) * t
                    + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2
                    + (-a + 3.0 * b - 3.0 * c + d) * t3)
            };
            out.push(((c(p0.0, p1.0, p2.0, p3.0), c(p0.1, p1.1, p2.1, p3.1)), k));
        }
    }
    out
}

#[cfg(test)]
mod interpolate_tests {
    use super::*;

    #[test]
    fn monotone_passes_through_points_without_overshoot() {
        let pts = [(0.0, 0.0), (1.0, 1.0), (2.0, 1.0), (3.0, 5.0), (4.0, 0.0)];
        let (out, origin) = interpolate(&pts, Interpolate::MonotoneCubic);
        assert_eq!(out.len(), 1 + 4 * CURVE_STEPS);
        assert_eq!(origin.len(), out.len());
        for p in pts {
            assert!(out
                .iter()
                .any(|q| (q.0 - p.0).abs() < 1e-12 && (q.1 - p.1).abs() < 1e-12));
        }
        // Between each pair the curve stays within the pair's y range.
        for (q, &k) in out.iter().zip(&origin) {
            let (a, b) = (pts[k].1, pts[k + 1].1);
            assert!(
                q.1 >= a.min(b) - 1e-9 && q.1 <= a.max(b) + 1e-9,
                "{q:?} on {k}"
            );
        }
    }

    #[test]
    fn catmull_rom_passes_through_points_and_breaks_on_nan() {
        let pts = [
            (0.0, 0.0),
            (1.0, 2.0),
            (2.0, 0.0),
            (f64::NAN, f64::NAN),
            (3.0, 1.0),
            (4.0, 3.0),
            (5.0, 2.0),
        ];
        let (out, origin) = interpolate(&pts, Interpolate::CatmullRom);
        assert_eq!(out.iter().filter(|p| p.0.is_nan()).count(), 1);
        assert!(out
            .iter()
            .any(|q| (q.0 - 1.0).abs() < 1e-12 && (q.1 - 2.0).abs() < 1e-12));
        assert_eq!(origin[out.iter().position(|p| p.0.is_nan()).unwrap()], 3);
        let (lin, _) = interpolate(&pts, Interpolate::Linear);
        assert_eq!(lin.len(), pts.len());
    }
}

/// Which axis a [`SignSplit`] cuts along.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SplitAxis {
    /// Cut where the curve crosses `y = at` (a time series above / below a
    /// target): each part fills between the curve and the threshold.
    #[default]
    Y,
    /// Cut at `x = at` (a distribution of outcomes: losses left of 0, gains
    /// right of it).
    X,
}

/// Colour an area, density or line by the side of a threshold it is on —
/// red below, green above — cut at the exact (interpolated) crossing, so the
/// parts meet without a gap or an overlap (DataZooDE/anofox-visualization#1256).
/// `at` is in data units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SignSplit {
    pub axis: SplitAxis,
    pub at: f64,
    pub below: (u8, u8, u8),
    pub above: (u8, u8, u8),
}

impl SignSplit {
    /// The default: along y at 0, red below, green above.
    pub fn new(below: (u8, u8, u8), above: (u8, u8, u8)) -> Self {
        SignSplit {
            axis: SplitAxis::Y,
            at: 0.0,
            below,
            above,
        }
    }

    pub fn axis(mut self, axis: SplitAxis) -> Self {
        self.axis = axis;
        self
    }

    pub fn at(mut self, at: f64) -> Self {
        self.at = at;
        self
    }

    /// The colour of a run on that side.
    pub fn colour(&self, above: bool) -> (u8, u8, u8) {
        if above {
            self.above
        } else {
            self.below
        }
    }
}

/// A polyline cut into runs on either side of `at` (along `axis`), each run
/// with its side (`true`: at or above the threshold) and its points; a
/// crossing is interpolated and ends one run and starts the next, so
/// neighbouring runs share it. A non-finite point breaks the line. The
/// points and `at` are in the same (scale) space.
pub fn split_runs(pts: &[(f64, f64)], axis: SplitAxis, at: f64) -> Vec<(bool, Vec<(f64, f64)>)> {
    let key = |p: (f64, f64)| match axis {
        SplitAxis::Y => p.1,
        SplitAxis::X => p.0,
    };
    let mut runs: Vec<(bool, Vec<(f64, f64)>)> = Vec::new();
    let mut cur: Option<(bool, Vec<(f64, f64)>)> = None;
    let mut prev: Option<(f64, f64)> = None;
    for &p in pts {
        if !(p.0.is_finite() && p.1.is_finite()) {
            if let Some(r) = cur.take() {
                runs.push(r);
            }
            prev = None;
            continue;
        }
        let k = key(p);
        // on the threshold, a point keeps the side of the run it continues
        let side = match &cur {
            Some((s, _)) if k == at => *s,
            _ => k >= at,
        };
        match (&mut cur, prev) {
            // a run that so far only touches the threshold takes the side
            // of its first point off it
            (Some((s, run)), _) if *s != side && run.iter().all(|&r| key(r) == at) => {
                *s = side;
                run.push(p);
            }
            (Some((s, run)), Some(q)) if *s != side => {
                // the exact crossing between q and p
                let (kq, kp) = (key(q), k);
                let t = if kp != kq { (at - kq) / (kp - kq) } else { 0.0 };
                let c = (q.0 + (p.0 - q.0) * t, q.1 + (p.1 - q.1) * t);
                let c = match axis {
                    SplitAxis::Y => (c.0, at),
                    SplitAxis::X => (at, c.1),
                };
                run.push(c);
                let done = cur.take().unwrap();
                runs.push(done);
                cur = Some((side, vec![c, p]));
            }
            (Some((_, run)), _) => run.push(p),
            (None, _) => cur = Some((side, vec![p])),
        }
        prev = Some(p);
    }
    if let Some(r) = cur {
        runs.push(r);
    }
    runs
}

#[cfg(test)]
mod split_tests {
    use super::*;

    #[test]
    fn a_line_crossing_zero_is_cut_at_the_crossing() {
        let pts = [(0.0, -2.0), (1.0, 2.0), (2.0, 4.0), (3.0, -4.0)];
        let runs = split_runs(&pts, SplitAxis::Y, 0.0);
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0], (false, vec![(0.0, -2.0), (0.5, 0.0)]));
        assert_eq!(
            runs[1],
            (true, vec![(0.5, 0.0), (1.0, 2.0), (2.0, 4.0), (2.5, 0.0)])
        );
        assert_eq!(runs[2], (false, vec![(2.5, 0.0), (3.0, -4.0)]));
    }

    #[test]
    fn a_distribution_is_cut_at_x() {
        let pts = [(-2.0, 0.1), (-1.0, 0.3), (1.0, 0.5), (2.0, 0.1)];
        let runs = split_runs(&pts, SplitAxis::X, 0.0);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].1.last(), Some(&(0.0, 0.4)));
        assert_eq!(runs[1].1.first(), Some(&(0.0, 0.4)));
        assert!(!runs[0].0 && runs[1].0);
    }

    #[test]
    fn a_gap_breaks_the_line_and_a_point_on_the_threshold_keeps_its_side() {
        let pts = [
            (0.0, 1.0),
            (1.0, 0.0),
            (2.0, f64::NAN),
            (3.0, 0.0),
            (4.0, -1.0),
        ];
        let runs = split_runs(&pts, SplitAxis::Y, 0.0);
        assert_eq!(runs.len(), 2, "{runs:?}");
        assert_eq!(runs[0], (true, vec![(0.0, 1.0), (1.0, 0.0)]));
        // a run that starts on the threshold takes the side it then goes to
        assert_eq!(runs[1], (false, vec![(3.0, 0.0), (4.0, -1.0)]));
    }

    #[test]
    fn colours_by_side_and_breaks_on_gaps_along_x() {
        let sp = SignSplit::new((1, 0, 0), (0, 1, 0))
            .axis(SplitAxis::X)
            .at(2.0);
        assert_eq!((sp.colour(false), sp.colour(true)), ((1, 0, 0), (0, 1, 0)));
        let runs = split_runs(
            &[(0.0, 1.0), (f64::INFINITY, 1.0), (1.0, 1.0), (3.0, 2.0)],
            SplitAxis::X,
            2.0,
        );
        assert_eq!(runs.len(), 3, "{runs:?}");
        assert_eq!(runs[2].1[0], (2.0, 1.5));
        // a flat segment along the threshold doesn't divide by zero
        let flat = split_runs(&[(0.0, 0.0), (1.0, 0.0), (2.0, 1.0)], SplitAxis::Y, 0.0);
        assert_eq!(flat.len(), 1);
    }
}
