use rand::{Rng, SeedableRng};

use crate::data::{DataFrame, Value};

use super::{Position, PositionParams};

const JITTER_SEED: u64 = 0x6A17_7E2D;

/// Add random noise to x and y positions to reduce overplotting.
pub struct PositionJitter {
    pub width: f64,
    pub height: f64,
}

impl Default for PositionJitter {
    fn default() -> Self {
        PositionJitter {
            width: 0.4,
            height: 0.4,
        }
    }
}

impl Position for PositionJitter {
    fn compute(&self, data: &mut DataFrame, _params: &PositionParams) {
        // Fixed seed: the same data must render the same SVG (stable snapshots,
        // cacheable output, no flicker when a host re-renders), as with
        // ggplot2's `position_jitter(seed = …)`.
        let mut rng = rand::rngs::StdRng::seed_from_u64(JITTER_SEED);

        if let Some(x_col) = data.column_mut("x") {
            for v in x_col.iter_mut() {
                if let Some(x) = v.as_f64() {
                    let jitter = rng.gen_range(-self.width..self.width);
                    *v = Value::Float(x + jitter);
                }
            }
        }

        if self.height > 0.0 {
            if let Some(y_col) = data.column_mut("y") {
                for v in y_col.iter_mut() {
                    if let Some(y) = v.as_f64() {
                        let jitter = rng.gen_range(-self.height..self.height);
                        *v = Value::Float(y + jitter);
                    }
                }
            }
        }
    }

    fn name(&self) -> &str {
        "jitter"
    }
}
