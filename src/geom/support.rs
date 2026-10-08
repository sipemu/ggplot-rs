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
            band,
        }
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
