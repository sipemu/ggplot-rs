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
