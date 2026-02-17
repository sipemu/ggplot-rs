use crate::data::DataFrame;

use super::{Position, PositionParams};

/// No position adjustment.
pub struct PositionIdentity;

impl Position for PositionIdentity {
    fn compute(&self, _data: &mut DataFrame, _params: &PositionParams) {
        // Passthrough
    }

    fn name(&self) -> &str {
        "identity"
    }
}
