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

        // ggplot2 stacks the first group at the TOP (so the stack order top-to-
        // bottom matches the legend), so accumulate downward from each x's total
        // rather than upward from 0.
        // Per-x accumulators keyed by the borrowed x key: O(1) per row instead
        // of a linear scan over the distinct x values.
        let mut totals: HashMap<Cow<'_, str>, f64> = HashMap::new();
        for (x, y) in x_col.iter().zip(y_col.iter()) {
            *totals.entry(x.key_str()).or_insert(0.0) += y.as_f64().unwrap_or(0.0);
        }

        let mut consumed: HashMap<Cow<'_, str>, f64> = HashMap::new();
        let mut tops = Vec::with_capacity(y_col.len());
        let mut bottoms = Vec::with_capacity(y_col.len());

        for (x, y) in x_col.iter().zip(y_col.iter()) {
            let x_key = x.key_str();
            let y_val = y.as_f64().unwrap_or(0.0);
            let total = totals.get(&x_key).copied().unwrap_or(0.0);
            let run = consumed.get(&x_key).copied().unwrap_or(0.0);

            // This group occupies [total - run - y, total - run] (top-down).
            tops.push(total - run);
            bottoms.push(total - run - y_val);

            *consumed.entry(x_key).or_insert(0.0) += y_val;
        }

        // A non-finite vjust is validated (and warned on) separately; here
        // it falls back to 1 (the byte-identical top-anchored path).
        let vjust = if self.vjust.is_finite() {
            self.vjust
        } else {
            1.0
        };

        let write_ymax = (vjust - 1.0).abs() > f64::EPSILON;
        let new_y: Vec<Value> = if write_ymax {
            bottoms
                .iter()
                .zip(tops.iter())
                .map(|(&b, &t)| Value::Float((1.0 - vjust) * b + vjust * t))
                .collect()
        } else {
            tops.iter().map(|&t| Value::Float(t)).collect()
        };
        let ymin_vals: Vec<Value> = bottoms.into_iter().map(Value::Float).collect();

        if let Some(col) = data.column_mut("y") {
            *col = new_y;
        }
        if !data.has_column("ymin") {
            data.add_column("ymin".to_string(), ymin_vals);
        } else if let Some(col) = data.column_mut("ymin") {
            *col = ymin_vals;
        }
        if write_ymax {
            let ymax_vals: Vec<Value> = tops.into_iter().map(Value::Float).collect();
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
