use crate::data::{DataFrame, Value};

use super::{Position, PositionParams};

/// Combined jitter and dodge position adjustment (ggplot2's
/// `position_jitterdodge()`): useful for showing individual points within
/// dodged groups (e.g. over boxplots).
///
/// As in ggplot2, points are first dodged like [`PositionDodge`]
/// (groups in legend order, counted per x slot), then jittered by
/// `jitter_width / (ndodge + 2)` either side, where `ndodge` is the most
/// groups any x slot holds, so each group's points stay inside its own
/// slot (#429). [`Default`] uses ggplot2's `jitter.width = NULL`: 40 % of
/// the x resolution (0.4 on a discrete axis).
///
/// [`PositionDodge`]: super::dodge::PositionDodge
pub struct PositionJitterDodge {
    pub jitter_width: f64,
    pub jitter_height: f64,
    pub dodge_width: f64,
    seed: u64,
    /// `jitter.width = NULL`: 0.4 × the x resolution, set by [`Default`].
    auto_width: bool,
}

impl PositionJitterDodge {
    /// Explicit `jitter.width` (x data units, before the `ndodge + 2`
    /// division) and `jitter.height` (the y spread).
    pub fn new(jitter_width: f64, jitter_height: f64) -> Self {
        PositionJitterDodge {
            jitter_width,
            jitter_height,
            dodge_width: 0.9,
            seed: 42,
            auto_width: false,
        }
    }

    pub fn with_dodge_width(mut self, width: f64) -> Self {
        self.dodge_width = width;
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    fn pseudo_random(seed: u64, i: usize) -> f64 {
        // Simple deterministic hash → uniform in [-0.5, 0.5]
        let h = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(i as u64)
            .wrapping_mul(1442695040888963407);
        let h = h ^ (h >> 33);
        let h = h.wrapping_mul(0xff51afd7ed558ccd);
        (h as f64 / u64::MAX as f64) - 0.5
    }
}

impl Default for PositionJitterDodge {
    fn default() -> Self {
        PositionJitterDodge {
            auto_width: true,
            ..PositionJitterDodge::new(0.4, 0.0)
        }
    }
}

/// ggplot2's `resolution(x, zero = FALSE)`: the smallest gap between
/// distinct x values (1 for a discrete or single-valued x).
fn resolution(x: &[Value]) -> f64 {
    if super::is_discrete_x(x) {
        return 1.0;
    }
    let mut v: Vec<f64> = x
        .iter()
        .filter_map(Value::as_f64)
        .filter(|f| f.is_finite())
        .collect();
    v.sort_by(f64::total_cmp);
    v.dedup();
    v.windows(2)
        .map(|w| w[1] - w[0])
        .fold(None, |m: Option<f64>, d| Some(m.map_or(d, |m| m.min(d))))
        .unwrap_or(1.0)
}

impl Position for PositionJitterDodge {
    fn compute(&self, data: &mut DataFrame, params: &PositionParams) {
        let x_col = match data.column("x") {
            Some(c) => c.to_vec(),
            None => return,
        };
        // ndodge: the most dodge groups sharing one x slot (1 without groups).
        let keys = super::dodge::group_keys(data);
        let mut slots: indexmap::IndexMap<String, indexmap::IndexSet<&str>> =
            indexmap::IndexMap::new();
        if let Some(keys) = &keys {
            for (x, k) in x_col.iter().zip(keys) {
                if !x.is_na() {
                    slots.entry(x.to_group_key()).or_default().insert(k);
                }
            }
        }
        let ndodge = slots.values().map(|s| s.len()).max().unwrap_or(1).max(1) as f64;
        let width = if self.auto_width {
            0.4 * resolution(&x_col)
        } else {
            self.jitter_width
        };
        // ggplot2 jitters ±(width / (ndodge + 2)); `pseudo_random` is ±0.5.
        let amount = 2.0 * width / (ndodge + 2.0);

        super::dodge::dodge_rows(
            data,
            self.dodge_width,
            None,
            false,
            false,
            params.group_order.as_deref(),
        );

        let seed = self.seed;
        if super::is_discrete_x(&x_col) {
            // Categories can't move: the jitter joins the dodge offset.
            super::add_x_offsets(data, |i| Self::pseudo_random(seed, i) * amount);
        } else if let Some(col) = data.column_mut("x") {
            for (i, v) in col.iter_mut().enumerate() {
                if let Some(f) = v.as_f64() {
                    *v = Value::Float(f + Self::pseudo_random(seed, i) * amount);
                }
            }
        }

        // Y jitter
        if self.jitter_height.abs() > f64::EPSILON {
            if let Some(col) = data.column_mut("y") {
                for (i, v) in col.iter_mut().enumerate() {
                    if let Some(f) = v.as_f64() {
                        *v = Value::Float(
                            f + Self::pseudo_random(self.seed.wrapping_add(1), i)
                                * self.jitter_height,
                        );
                    }
                }
            }
        }
    }

    /// Dodge groups ranked like `PositionDodge` (legend order, #428), so
    /// the points line up with dodged boxes / violins.
    fn group_order(&self, data: &DataFrame) -> Option<Vec<String>> {
        super::dodge::group_keys(data).map(|keys| super::dodge::dedup_order(&keys))
    }

    fn name(&self) -> &str {
        "jitterdodge"
    }
}
