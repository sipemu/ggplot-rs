//! `StatRenko` / `StatPointFigure`: time-independent price charts built
//! from a box size. Prices are read from `y` in data order (time order).
//!
//! * Renko: a new brick of height `box_size` whenever the price closes a
//!   full box above the last brick's top or below its bottom (so a
//!   reversal needs a two-box move). Output: one row per brick with `x`
//!   (brick number), `ymin`/`ymax`, `y` (= ymax) and `direction`
//!   (`"up"`/`"down"`).
//! * Point & figure: columns of X (rising) and O (falling) boxes; a column
//!   extends while the price moves on by whole boxes and reverses after a
//!   `reversal`-box move against it. Output: one row per box with `x`
//!   (column number), `y` (box centre), `ymin`/`ymax` and `mark`
//!   (`"X"`/`"O"`).

use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;

use super::Stat;

/// A Renko brick: `(low, high, up)`.
type Brick = (f64, f64, bool);

/// Renko bricks `(low, high, up)` for `prices` with box size `b`.
pub fn renko_bricks(prices: &[f64], b: f64) -> Vec<(f64, f64, bool)> {
    let mut out: Vec<(f64, f64, bool)> = Vec::new();
    let Some(&first) = prices.first() else {
        return out;
    };
    if b.is_nan() || b <= 0.0 {
        return out;
    }
    let (mut lo, mut hi) = (first, first);
    for &p in &prices[1..] {
        while p >= hi + b {
            out.push((hi, hi + b, true));
            lo = hi;
            hi += b;
        }
        while p <= lo - b {
            out.push((lo - b, lo, false));
            hi = lo;
            lo -= b;
        }
    }
    out
}

/// Point-and-figure columns: `(column, box low, is_x)` per box, for box
/// size `b` and a `reversal`-box reversal.
pub fn point_figure_boxes(prices: &[f64], b: f64, reversal: usize) -> Vec<(usize, f64, bool)> {
    let mut out = Vec::new();
    let Some(&first) = prices.first() else {
        return out;
    };
    if b.is_nan() || b <= 0.0 {
        return out;
    }
    let k = reversal.max(1) as f64;
    // Work in whole boxes from a floor-aligned base.
    let base = (first / b).floor() * b;
    let level = |p: f64| ((p - base) / b).floor();
    // (direction up?, low box, high box) of the current column.
    let mut col: Option<(bool, f64, f64)> = None;
    let mut idx = 0usize;
    let start = level(first);
    let push = |out: &mut Vec<(usize, f64, bool)>, idx: usize, from: f64, to: f64, up: bool| {
        let mut l = from;
        while l <= to {
            out.push((idx, base + l * b, up));
            l += 1.0;
        }
    };
    for &p in &prices[1..] {
        let lv = level(p);
        match col {
            None => {
                if lv >= start + 1.0 {
                    col = Some((true, start, lv));
                } else if lv <= start - 1.0 {
                    col = Some((false, lv, start));
                }
            }
            Some((true, low, high)) => {
                if lv > high {
                    col = Some((true, low, lv));
                } else if lv <= high - k {
                    push(&mut out, idx, low, high, true);
                    idx += 1;
                    col = Some((false, lv, high - 1.0));
                }
            }
            Some((false, low, high)) => {
                if lv < low {
                    col = Some((false, lv, high));
                } else if lv >= low + k {
                    push(&mut out, idx, low, high, false);
                    idx += 1;
                    col = Some((true, low + 1.0, lv));
                }
            }
        }
    }
    if let Some((up, low, high)) = col {
        push(&mut out, idx, low, high, up);
    }
    out
}

fn prices(data: &DataFrame) -> Vec<f64> {
    data.column("y")
        .map(|c| {
            c.iter()
                .filter_map(|v| v.as_f64())
                .filter(|v| v.is_finite())
                .collect()
        })
        .unwrap_or_default()
}

/// Renko stat behind `geom_renko`.
#[derive(Clone, Debug)]
pub struct StatRenko {
    pub box_size: f64,
}

impl Stat for StatRenko {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let bricks = renko_bricks(&prices(data), self.box_size);
        let f = |g: &dyn Fn(usize, &Brick) -> f64| {
            bricks
                .iter()
                .enumerate()
                .map(|(i, b)| Value::Float(g(i, b)))
                .collect::<Vec<_>>()
        };
        let mut out = DataFrame::new();
        out.add_column("x".into(), f(&|i, _| i as f64 + 1.0));
        out.add_column("ymin".into(), f(&|_, b| b.0));
        out.add_column("ymax".into(), f(&|_, b| b.1));
        out.add_column("y".into(), f(&|_, b| b.1));
        out.add_column(
            "direction".into(),
            bricks
                .iter()
                .map(|b| Value::Str(if b.2 { "up" } else { "down" }.into()))
                .collect(),
        );
        out
    }
    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Y]
    }
    fn panelwise(&self) -> bool {
        true
    }
    fn name(&self) -> &str {
        "renko"
    }
}

/// Point-and-figure stat behind `geom_point_figure`.
#[derive(Clone, Debug)]
pub struct StatPointFigure {
    pub box_size: f64,
    pub reversal: usize,
}

impl Stat for StatPointFigure {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let boxes = point_figure_boxes(&prices(data), self.box_size, self.reversal);
        let b = self.box_size;
        let mut out = DataFrame::new();
        out.add_column(
            "x".into(),
            boxes
                .iter()
                .map(|x| Value::Float(x.0 as f64 + 1.0))
                .collect(),
        );
        out.add_column(
            "y".into(),
            boxes.iter().map(|x| Value::Float(x.1 + b / 2.0)).collect(),
        );
        out.add_column(
            "ymin".into(),
            boxes.iter().map(|x| Value::Float(x.1)).collect(),
        );
        out.add_column(
            "ymax".into(),
            boxes.iter().map(|x| Value::Float(x.1 + b)).collect(),
        );
        out.add_column(
            "mark".into(),
            boxes
                .iter()
                .map(|x| Value::Str(if x.2 { "X" } else { "O" }.into()))
                .collect(),
        );
        out
    }
    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Y]
    }
    fn panelwise(&self) -> bool {
        true
    }
    fn name(&self) -> &str {
        "point_figure"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renko_needs_a_full_box_and_two_to_reverse() {
        let b = renko_bricks(&[10.0, 10.9, 11.0, 12.5, 11.2, 10.0, 9.0], 1.0);
        // 11 → [10,11]; 12.5 → [11,12]; 11.2 no; 10 → [10,11] down; 9 → [9,10].
        assert_eq!(
            b,
            vec![
                (10.0, 11.0, true),
                (11.0, 12.0, true),
                (10.0, 11.0, false),
                (9.0, 10.0, false)
            ]
        );
        assert!(renko_bricks(&[1.0, 2.0], 0.0).is_empty());
    }

    #[test]
    fn point_figure_columns_reverse_after_k_boxes() {
        // Box 1, reversal 3: up to 15, back to 12 (3 boxes) → O column, up to 16.
        let p = point_figure_boxes(&[10.0, 12.0, 15.0, 14.0, 12.0, 11.0, 14.0, 16.0], 1.0, 3);
        let col = |c: usize| {
            p.iter()
                .filter(|b| b.0 == c)
                .map(|b| (b.1, b.2))
                .collect::<Vec<_>>()
        };
        assert_eq!(col(0).len(), 6, "X from 10 to 15");
        assert!(col(0).iter().all(|b| b.1));
        assert_eq!(
            col(1),
            vec![(11.0, false), (12.0, false), (13.0, false), (14.0, false)]
        );
        assert!(col(2).iter().all(|b| b.1) && col(2).last().unwrap().0 == 16.0);
    }
}
