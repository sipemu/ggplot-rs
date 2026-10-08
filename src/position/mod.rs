pub mod dodge;
pub mod dodge2;
pub mod fill;
pub mod identity;
pub mod jitter;
pub mod jitterdodge;
pub mod nudge;
pub mod stack;

use crate::data::DataFrame;

/// Parameters for position adjustments.
#[derive(Clone, Debug)]
pub struct PositionParams {
    pub width: f64,
    pub height: f64,
}

impl Default for PositionParams {
    fn default() -> Self {
        PositionParams {
            width: 0.9,
            height: 0.0,
        }
    }
}

/// Trait for position adjustments.
pub trait Position: Send + Sync {
    /// Adjust positions for data.
    fn compute(&self, data: &mut DataFrame, params: &PositionParams);

    fn name(&self) -> &str;
}

/// Column in which stacking positions keep each row's original (pre-stack)
/// `y`, so hover metadata (`data-value`, tooltips) can report the measured
/// value rather than the cumulative stack top.
pub const RAW_Y_COL: &str = ".y_raw";

/// Remember the pre-position `y` in [`RAW_Y_COL`] (first adjustment wins).
pub(crate) fn preserve_raw_y(data: &mut DataFrame, y: &[crate::data::Value]) {
    if !data.has_column(RAW_Y_COL) {
        data.add_column(RAW_Y_COL.to_string(), y.to_vec());
    }
}
