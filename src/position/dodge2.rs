use crate::data::DataFrame;

use super::dodge::dodge_rows;
use super::{Position, PositionParams};

/// Like position_dodge but preserves total width and adds padding between
/// groups (ggplot2's `position_dodge2(width, padding, reverse)`). Elements
/// are centred exactly as [`PositionDodge`](struct@super::dodge::PositionDodge)
/// places them; on a continuous axis `xmin`/`xmax` shrink to
/// `(1 − padding)` of each group's share.
#[derive(Clone, Debug, PartialEq)]
pub struct PositionDodge2 {
    pub padding: f64,
    /// Total dodge width; `None` = the layer default (0.9).
    pub width: Option<f64>,
    /// Reverse the group order.
    pub reverse: bool,
}

impl PositionDodge2 {
    pub fn new(padding: f64) -> Self {
        PositionDodge2 {
            padding,
            ..Default::default()
        }
    }

    /// Set the total dodge width.
    pub fn with_width(mut self, width: f64) -> Self {
        self.width = Some(width);
        self
    }

    /// Reverse the order of the dodged groups (`reverse = TRUE`).
    pub fn with_reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }
}

impl Default for PositionDodge2 {
    fn default() -> Self {
        PositionDodge2 {
            padding: 0.1,
            width: None,
            reverse: false,
        }
    }
}

impl Position for PositionDodge2 {
    fn compute(&self, data: &mut DataFrame, params: &PositionParams) {
        dodge_rows(
            data,
            self.width.unwrap_or(params.width),
            Some(self.padding),
            self.reverse,
        );
    }

    fn name(&self) -> &str {
        "dodge2"
    }
}
