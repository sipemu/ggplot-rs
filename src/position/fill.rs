use std::borrow::Cow;
use std::collections::HashMap;

use crate::data::{DataFrame, Value};

use super::{Position, PositionParams};

/// Normalized stacking to 100% (ggplot2's `position_fill(vjust = )`) —
/// like [`PositionStack`](crate::position::stack::PositionStack) but
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

        // First compute totals per x group
        let mut x_totals: HashMap<Cow<'_, str>, f64> = HashMap::new();
        for (x, y) in x_col.iter().zip(y_col.iter()) {
            *x_totals.entry(x.key_str()).or_insert(0.0) += y.as_f64().unwrap_or(0.0);
        }

        // Then compute normalized stacked positions
        let mut x_cumsum: HashMap<Cow<'_, str>, f64> = HashMap::new();
        let mut tops = Vec::with_capacity(y_col.len());
        let mut bottoms = Vec::with_capacity(y_col.len());

        for (x, y) in x_col.iter().zip(y_col.iter()) {
            let x_key = x.key_str();
            let y_val = y.as_f64().unwrap_or(0.0);

            let total = x_totals.get(&x_key).copied().unwrap_or(1.0);
            let total = if total.abs() < f64::EPSILON {
                1.0
            } else {
                total
            };

            // ggplot2 puts the first group at the top, so fill downward from 1.
            let consumed = x_cumsum.get(&x_key).copied().unwrap_or(0.0);

            let norm_y = y_val / total;
            tops.push(1.0 - consumed);
            bottoms.push(1.0 - consumed - norm_y);

            *x_cumsum.entry(x_key).or_insert(0.0) += norm_y;
        }

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
