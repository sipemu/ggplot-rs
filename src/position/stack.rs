use std::borrow::Cow;
use std::collections::HashMap;

use crate::data::{DataFrame, Value};

use super::{Position, PositionParams};

/// Stack bars/areas on top of each other (ggplot2's `position_stack(vjust
/// = )`).
///
/// `vjust` places the row's `y` within its stacked segment: `1` (the
/// default) puts it at the segment's top — the pre-B3-4 behaviour, and
/// byte-identical (no `ymax` column is written). Anything else writes
/// `y = (1 − vjust)·ymin + vjust·ymax` and an `ymax` column holding the
/// segment's top, so e.g. `vjust(0.5)` centres a label in its segment.
///
/// `PositionStack` is also usable as a bare value (`vjust` 1, as before
/// the struct gained fields) via the [`PositionStack`](const@PositionStack)
/// const.
#[derive(Clone, Debug, PartialEq)]
pub struct PositionStack {
    pub vjust: f64,
}

impl Default for PositionStack {
    fn default() -> Self {
        PositionStack { vjust: 1.0 }
    }
}

/// The default stack (`vjust` 1): lets `.position(PositionStack)` keep
/// compiling now that the type has fields.
#[allow(non_upper_case_globals)]
pub const PositionStack: PositionStack = PositionStack { vjust: 1.0 };

impl PositionStack {
    /// Place `y` within each stacked segment (`0` = bottom, `0.5` =
    /// middle, `1` = top, the default).
    pub fn with_vjust(mut self, vjust: f64) -> Self {
        self.vjust = vjust;
        self
    }
}

/// `position_stack()` / `position_stack().with_vjust(v)` — see
/// [`PositionStack`](struct@PositionStack).
pub fn position_stack() -> PositionStack {
    PositionStack::default()
}

impl Position for PositionStack {
    fn compute(&self, data: &mut DataFrame, _params: &PositionParams) {
        // Group by x, accumulate y values
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

        // ggplot2 stacks a group's positive and negative values
        // independently within each x (#154, ggplot2's internal
        // `pos`/`neg` split in `collide`/`pos_stack`): run the same top-
        // down, legend-order accumulation twice, once over the rows
        // with y >= 0 and once over y < 0, so e.g. a bar chart mixing a
        // group's positive and negative segments at the same x gets two
        // independent stacks (one above zero, one below) instead of a
        // single pooled running total that lets a negative row cancel
        // into a positive one's segment. ggplot2 stacks the first group
        // at the top of its bucket (so top-to-bottom matches the
        // legend), so each bucket still accumulates downward from its
        // own total rather than upward from 0 — "top"/"bottom" here are
        // in that accumulation order, not value order (a negative
        // bucket's "top", nearer 0, has the smaller `tops[i]`), so
        // `ymin`/`ymax`/`y` below always take the min/max of the pair
        // rather than assume `tops[i] >= bottoms[i]`.
        // Per-x accumulators keyed by the borrowed x key: O(1) per row
        // instead of a linear scan over the distinct x values.
        //
        // A missing row (`super::stack_row`, review #167 finding 2 on
        // #171) joins the positive bucket but contributes 0 to its
        // total, and its own top/bottom are left NaN below — so it
        // doesn't poison any other row's stack, matching ggplot2's
        // `pos_stack` (`y <- ifelse(is.na(df$y), 0, df$y)`).
        for positive in [true, false] {
            let mut totals: HashMap<Cow<'_, str>, f64> = HashMap::new();
            for (x, y) in x_col.iter().zip(y_col.iter()) {
                let row = super::stack_row(y);
                if row.is_positive_bucket() != positive {
                    continue;
                }
                *totals.entry(x.key_str()).or_insert(0.0) += row.contribution();
            }

            let mut consumed: HashMap<Cow<'_, str>, f64> = HashMap::new();
            for (i, (x, y)) in x_col.iter().zip(y_col.iter()).enumerate() {
                let row = super::stack_row(y);
                if row.is_positive_bucket() != positive {
                    continue;
                }
                let x_key = x.key_str();
                let total = totals.get(&x_key).copied().unwrap_or(0.0);
                let run = consumed.get(&x_key).copied().unwrap_or(0.0);

                let Some(y_val) = row.value() else {
                    // Missing: draw nothing for this row (NA in
                    // ggplot2), but don't advance `consumed` — it
                    // contributed 0, so the next row in this bucket
                    // still starts from the same `run`.
                    tops[i] = f64::NAN;
                    bottoms[i] = f64::NAN;
                    continue;
                };

                // This group occupies [total - run - y, total - run]
                // within its own (same-sign) bucket.
                tops[i] = total - run;
                bottoms[i] = total - run - y_val;

                *consumed.entry(x_key).or_insert(0.0) += y_val;
            }
        }

        // A non-finite vjust is validated (and warned on) separately; here
        // it falls back to 1 (the byte-identical top-anchored path).
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
        "stack"
    }

    fn per_panel(&self) -> bool {
        true
    }

    fn validate(&self, warnings: &mut Vec<String>) {
        if !self.vjust.is_finite() {
            warnings.push(format!(
                "position_stack: vjust({}) must be finite; falling back to 1",
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
    /// with y = 3, -2, 4, -1 and `position_stack(vjust = 0.5)` gives
    /// y/ymin/ymax = (5.5, 4, 7), (-2, -3, -1), (2, 0, 4), (-0.5, -1, 0)
    /// (#154) — positives (a, c) stack above 0 using only their own sum
    /// (7) as the total; negatives (b, d) stack below 0 using only their
    /// own sum (-3), independently.
    #[test]
    fn negative_and_positive_values_stack_independently() {
        let mut df = frame(&[1.0; 4], &["a", "b", "c", "d"], &[3.0, -2.0, 4.0, -1.0]);
        PositionStack::default()
            .with_vjust(0.5)
            .compute(&mut df, &PositionParams::default());
        let y = col(&df, "y");
        let ymin = col(&df, "ymin");
        let ymax = col(&df, "ymax");
        for (i, (want_y, want_min, want_max)) in [
            (5.5, 4.0, 7.0),
            (-2.0, -3.0, -1.0),
            (2.0, 0.0, 4.0),
            (-0.5, -1.0, 0.0),
        ]
        .into_iter()
        .enumerate()
        {
            assert!((y[i] - want_y).abs() < 1e-9, "y[{i}]={}", y[i]);
            assert!((ymin[i] - want_min).abs() < 1e-9, "ymin[{i}]={}", ymin[i]);
            assert!((ymax[i] - want_max).abs() < 1e-9, "ymax[{i}]={}", ymax[i]);
        }
    }

    /// All-positive data (the common case) is byte-identical to the
    /// pre-#154 pooled algorithm: the negative bucket is simply empty.
    #[test]
    fn all_positive_values_are_unaffected() {
        let mut df = frame(&[1.0, 1.0], &["a", "b"], &[3.0, 5.0]);
        PositionStack.compute(&mut df, &PositionParams::default());
        assert_eq!(col(&df, "y"), vec![8.0, 5.0]);
        assert_eq!(col(&df, "ymin"), vec![5.0, 0.0]);
    }

    /// Review #167 finding 2 on sipemu/ggplot-rs#171 (originally #170's
    /// NaN bucket fix, which got the *bucket* right but the *arithmetic*
    /// wrong): a missing `y` joins the positive bucket, like ggplot2,
    /// but contributes **0** to its running total — not the raw NaN —
    /// so the *other* row sharing that bucket (`a`) stacks exactly as if
    /// `b` weren't there at all, instead of turning into NaN too.
    /// ggplot2 4.0.3 (`layer_data()`, `pos_stack`'s `y <-
    /// ifelse(is.na(df$y), 0, df$y)`): a=3, b=NaN, c=-2 gives a 0..3,
    /// c -2..0, b's own y/ymin left `NA`. `c` (the negative bucket) was
    /// already unaffected either way — it shares no bucket with `b`.
    #[test]
    fn missing_y_contributes_zero_and_does_not_poison_its_bucket() {
        let mut df = frame(&[1.0, 1.0, 1.0], &["a", "b", "c"], &[3.0, f64::NAN, -2.0]);
        PositionStack.compute(&mut df, &PositionParams::default());
        let y = col(&df, "y");
        let ymin = col(&df, "ymin");
        assert_eq!(
            (y[0], ymin[0]),
            (3.0, 0.0),
            "a must stack as if b's row weren't there at all: y={y:?} ymin={ymin:?}"
        );
        assert!(
            y[1].is_nan() && ymin[1].is_nan(),
            "b's own y/ymin must stay NaN (not drawn): y={y:?} ymin={ymin:?}"
        );
        assert_eq!(
            (y[2], ymin[2]),
            (0.0, -2.0),
            "c (negative bucket) must be unaffected: y={y:?} ymin={ymin:?}"
        );
    }

    /// The pre-existing bug finding 2 also fixes: a `Value::Na` `y` (not
    /// just a `Float(NaN)`) used to fall back to `0.0` via
    /// `as_f64().unwrap_or(0.0)` and get stacked as a *real* point at
    /// the top of its bucket, drawing a line straight through a
    /// supposedly-removed row. It must now be treated exactly like a
    /// `Float(NaN)` — contributing 0, with its own output left NaN.
    #[test]
    fn value_na_is_treated_like_float_nan_not_zero() {
        let mut df = DataFrame::new();
        df.add_column(
            "x".into(),
            vec![Value::Float(1.0), Value::Float(1.0), Value::Float(1.0)],
        );
        df.add_column(
            "fill".into(),
            vec![
                Value::Str("a".into()),
                Value::Str("b".into()),
                Value::Str("c".into()),
            ],
        );
        df.add_column(
            "y".into(),
            vec![Value::Float(3.0), Value::Na, Value::Float(-2.0)],
        );
        PositionStack.compute(&mut df, &PositionParams::default());
        let y = col(&df, "y");
        let ymin = col(&df, "ymin");
        assert_eq!(
            (y[0], ymin[0]),
            (3.0, 0.0),
            "a must be unaffected by b's Value::Na: y={y:?} ymin={ymin:?}"
        );
        assert!(
            y[1].is_nan() && ymin[1].is_nan(),
            "a Value::Na row must not become a real point at the stack top: \
             y={y:?} ymin={ymin:?}"
        );
    }
}
