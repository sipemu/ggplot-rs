use crate::data::{DataFrame, Value};

use super::{Position, PositionParams};

/// Nudge position — adds a fixed offset to x and/or y.
pub struct PositionNudge {
    pub x: f64,
    pub y: f64,
}

impl PositionNudge {
    pub fn new(x: f64, y: f64) -> Self {
        PositionNudge { x, y }
    }
}

impl Default for PositionNudge {
    fn default() -> Self {
        PositionNudge { x: 0.0, y: 0.0 }
    }
}

impl Position for PositionNudge {
    fn compute(&self, data: &mut DataFrame, _params: &PositionParams) {
        if self.x.abs() > f64::EPSILON {
            if let Some(col) = data.column_mut("x") {
                for v in col.iter_mut() {
                    if let Some(f) = v.as_f64() {
                        *v = Value::Float(f + self.x);
                    }
                }
            }
        }

        if self.y.abs() > f64::EPSILON {
            if let Some(col) = data.column_mut("y") {
                for v in col.iter_mut() {
                    if let Some(f) = v.as_f64() {
                        *v = Value::Float(f + self.y);
                    }
                }
            }
        }
    }

    fn name(&self) -> &str {
        "nudge"
    }
}
