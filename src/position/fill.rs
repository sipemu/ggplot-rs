use std::borrow::Cow;
use std::collections::HashMap;

use crate::data::{DataFrame, Value};

use super::{Position, PositionParams};

/// Normalized stacking to 100% (ggplot2's `position_fill(vjust = )`) —
/// like [`PositionStack`](struct@crate::position::stack::PositionStack) but
/// scaled to `[0, 1]` per x group.
///
/// `vjust` places the row's `y` within its stacked segment, same as
/// `PositionStack`: `1` (the default) is the segment's top, byte-
/// identical to before B3-4 (no `ymax` column). Anything else writes
/// `y = (1 − vjust)·ymin + vjust·ymax` plus the `ymax` column.
#[derive(Clone, Debug, PartialEq)]
pub struct PositionFill {
    pub vjust: f64,
}

impl Default for PositionFill {
    fn default() -> Self {
        PositionFill { vjust: 1.0 }
    }
}

/// The default fill (`vjust` 1): lets `.position(PositionFill)` keep
/// compiling now that the type has fields.
#[allow(non_upper_case_globals)]
pub const PositionFill: PositionFill = PositionFill { vjust: 1.0 };

impl PositionFill {
    /// Place `y` within each stacked segment (`0` = bottom, `0.5` =
    /// middle, `1` = top, the default).
    pub fn with_vjust(mut self, vjust: f64) -> Self {
        self.vjust = vjust;
        self
    }
}

/// `position_fill()` / `position_fill().with_vjust(v)` — see
/// [`PositionFill`](struct@PositionFill).
pub fn position_fill() -> PositionFill {
    PositionFill::default()
}

impl Position for PositionFill {
    fn compute(&self, data: &mut DataFrame, _params: &PositionParams) {
        let x_col = match data.column("x") {
            Some(c) => c.to_vec(),
            None => return,
        };
        let y_col = match data.column("y") {
            Some(c) => c.to_vec(),
            None => return,
        };
        super::preserve_raw_y(data, &y_col);

        let n = y_col.len();
        let mut tops = vec![0.0_f64; n];
        let mut bottoms = vec![0.0_f64; n];

        // ggplot2 normalizes a group's positive and negative values
        // independently within each x (#154, same split as
        // `PositionStack`): the positive rows at an x fill `[0, 1]` using
        // only their own sum as the denominator, and the negative rows
        // fill `[-1, 0]` using their own (absolute) sum — each bucket's
        // first group (in row/legend order) lands farthest from zero, as
        // `PositionStack` already does. "top"/"bottom" below are in that
        // accumulation order, not value order, so the final `ymin`/
        // `ymax`/`y` take the min/max of the pair.
        for positive in [true, false] {
            let sign_total = if positive { 1.0 } else { -1.0 };
            let mut totals: HashMap<Cow<'_, str>, f64> = HashMap::new();
            for (x, y) in x_col.iter().zip(y_col.iter()) {
                let y_val = y.as_f64().unwrap_or(0.0);
                if (y_val >= 0.0) != positive {
                    continue;
                }
                *totals.entry(x.key_str()).or_insert(0.0) += y_val;
            }

            let mut consumed: HashMap<Cow<'_, str>, f64> = HashMap::new();
            for (i, (x, y)) in x_col.iter().zip(y_col.iter()).enumerate() {
                let y_val = y.as_f64().unwrap_or(0.0);
                if (y_val >= 0.0) != positive {
                    continue;
                }
                let x_key = x.key_str();
                let total = totals.get(&x_key).copied().unwrap_or(0.0);
                let denom = if total.abs() < f64::EPSILON {
                    1.0
                } else {
                    total.abs()
                };
                let run = consumed.get(&x_key).copied().unwrap_or(0.0);

                let norm_y = y_val / denom;
                tops[i] = sign_total - run;
                bottoms[i] = sign_total - run - norm_y;

                *consumed.entry(x_key).or_insert(0.0) += norm_y;
            }
        }

        let vjust = if self.vjust.is_finite() {
            self.vjust
        } else {
            1.0
        };

        let write_ymax = (vjust - 1.0).abs() > f64::EPSILON;
        let lo_hi: Vec<(f64, f64)> = tops
            .iter()
            .zip(bottoms.iter())
            .map(|(&t, &b)| (t.min(b), t.max(b)))
            .collect();
        let new_y: Vec<Value> = if write_ymax {
            lo_hi
                .iter()
                .map(|&(lo, hi)| Value::Float((1.0 - vjust) * lo + vjust * hi))
                .collect()
        } else {
            lo_hi.iter().map(|&(_, hi)| Value::Float(hi)).collect()
        };
        let ymin_vals: Vec<Value> = lo_hi.iter().map(|&(lo, _)| Value::Float(lo)).collect();

        if let Some(col) = data.column_mut("y") {
            *col = new_y;
        }
        if !data.has_column("ymin") {
            data.add_column("ymin".to_string(), ymin_vals);
        } else if let Some(col) = data.column_mut("ymin") {
            *col = ymin_vals;
        }
        if write_ymax {
            let ymax_vals: Vec<Value> = lo_hi.iter().map(|&(_, hi)| Value::Float(hi)).collect();
            if !data.has_column("ymax") {
                data.add_column("ymax".to_string(), ymax_vals);
            } else if let Some(col) = data.column_mut("ymax") {
                *col = ymax_vals;
            }
        }
    }

    fn name(&self) -> &str {
        "fill"
    }

    fn per_panel(&self) -> bool {
        true
    }

    fn validate(&self, warnings: &mut Vec<String>) {
        if !self.vjust.is_finite() {
            warnings.push(format!(
                "position_fill: vjust({}) must be finite; falling back to 1",
                self.vjust
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(x: &[f64], g: &[&str], y: &[f64]) -> DataFrame {
        let mut df = DataFrame::new();
        df.add_column("x".into(), x.iter().map(|v| Value::Float(*v)).collect());
        df.add_column(
            "fill".into(),
            g.iter().map(|s| Value::Str(s.to_string())).collect(),
        );
        df.add_column("y".into(), y.iter().map(|v| Value::Float(*v)).collect());
        df
    }

    fn col(df: &DataFrame, c: &str) -> Vec<f64> {
        df.column(c)
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect()
    }

    /// R parity (ggplot2 4.0.3, `layer_data()`): x=1, fill groups a/b/c/d
    /// with y = 3, -2, 4, -1 and `position_fill(vjust = 0)` gives
    /// y/ymin/ymax = (0.571429, 0.571429, 1), (-1, -1, -0.333333),
    /// (0, 0, 0.571429), (-0.333333, -0.333333, 0) (#154) — positives
    /// (a, c) normalize to `[0, 1]` using only their own sum (7) as the
    /// denominator; negatives (b, d) normalize to `[-1, 0]` using only
    /// their own (absolute) sum (3), independently.
    #[test]
    fn negative_and_positive_values_fill_independently() {
        let mut df = frame(&[1.0; 4], &["a", "b", "c", "d"], &[3.0, -2.0, 4.0, -1.0]);
        PositionFill::default()
            .with_vjust(0.0)
            .compute(&mut df, &PositionParams::default());
        let y = col(&df, "y");
        let ymin = col(&df, "ymin");
        let ymax = col(&df, "ymax");
        for (i, (want_y, want_min, want_max)) in [
            (4.0 / 7.0, 4.0 / 7.0, 1.0),
            (-1.0, -1.0, -1.0 / 3.0),
            (0.0, 0.0, 4.0 / 7.0),
            (-1.0 / 3.0, -1.0 / 3.0, 0.0),
        ]
        .into_iter()
        .enumerate()
        {
            assert!((y[i] - want_y).abs() < 1e-6, "y[{i}]={}", y[i]);
            assert!((ymin[i] - want_min).abs() < 1e-6, "ymin[{i}]={}", ymin[i]);
            assert!((ymax[i] - want_max).abs() < 1e-6, "ymax[{i}]={}", ymax[i]);
        }
    }

    /// All-positive data (the common case) is byte-identical to the
    /// pre-#154 pooled algorithm: the negative bucket is simply empty.
    #[test]
    fn all_positive_values_are_unaffected() {
        let mut df = frame(&[1.0, 1.0], &["a", "b"], &[3.0, 5.0]);
        PositionFill.compute(&mut df, &PositionParams::default());
        let y = col(&df, "y");
        assert!((y[0] - 1.0).abs() < 1e-9);
        assert!((y[1] - 0.625).abs() < 1e-9);
    }
}
