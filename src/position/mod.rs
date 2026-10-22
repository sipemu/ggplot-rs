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
    /// The whole layer's group rank order (review #167 finding 2), set by
    /// `build::compute_position_per_panel` from [`Position::group_order`]
    /// *before* splitting the layer's rows into panels. A per-panel
    /// position (`PositionDodge`/`PositionDodge2`) ranks groups from this
    /// — the whole layer's first-appearance order — rather than
    /// recomputing it from whatever order each panel's own rows happen to
    /// list groups in, so every panel draws a given group on the same
    /// side (matching 0.22.2 and ggplot2 4.0.3's global `group` id).
    /// `None` outside the per-panel path (or for positions that don't
    /// override `group_order`), in which case each position falls back
    /// to its previous, locally-computed order.
    pub group_order: Option<Vec<String>>,
}

impl Default for PositionParams {
    fn default() -> Self {
        PositionParams {
            width: 0.9,
            height: 0.0,
            group_order: None,
        }
    }
}

/// Trait for position adjustments.
pub trait Position: Send + Sync {
    /// Adjust positions for data.
    fn compute(&self, data: &mut DataFrame, params: &PositionParams);

    fn name(&self) -> &str;

    /// Whether this position must run separately per facet panel rather
    /// than once on the pooled rows before facets are split (B3-3, D8).
    /// Default `false`. `PositionStack`/`PositionFill` override this to
    /// `true` — a shared x category's stack height must not mix rows from
    /// different panels. `PositionDodge`/`PositionDodge2`/`PositionJitter`
    /// also override this to `true` (sipemu/ggplot-rs#153): ggplot2
    /// computes a dodge's group count/offset via `compute_panel` (per
    /// panel), and `position_jitter`'s default width/height — 40% of
    /// `resolution()` — is likewise measured from each panel's own x/y,
    /// not the rows pooled across panels (verified against ggplot2 4.0.3
    /// with `layer_data()` on faceted plots; see `tests/position_panels_r.rs`).
    fn per_panel(&self) -> bool {
        false
    }

    /// The whole layer's group rank order (first-appearance, pre-panel-
    /// split), for a `per_panel` position whose group *ranking* must come
    /// from the whole layer even though its per-x *counting* stays
    /// per-panel (review #167 finding 2; sipemu/ggplot-rs#153).
    /// `build::compute_position_per_panel` calls this on the pooled data
    /// before splitting and threads the result through
    /// [`PositionParams::group_order`] to every panel's `compute` call.
    /// Default `None` — positions that don't need this (stack/fill/
    /// jitter) are unaffected.
    fn group_order(&self, _data: &DataFrame) -> Option<Vec<String>> {
        None
    }

    /// Validate this position's own parameters once per layer (D2);
    /// default no-op. Implementations push a warning and fall back to a
    /// sane default rather than panicking or silently miscomputing.
    fn validate(&self, _warnings: &mut Vec<String>) {}
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

/// A row's `y`, classified for `PositionStack`/`PositionFill`'s #154
/// positive/negative bucket split — shared so the two positions handle a
/// missing value identically (sipemu/ggplot-rs#171 review, finding 3).
///
/// `value` is `None` for a *missing* row: `Value::Na`, or a `Value::Float`
/// that isn't finite (NaN or an infinity). ggplot2's `pos_stack`/
/// `position_fill` (verified against ggplot2 4.0.3's R source —
/// `PositionStack$compute_panel` sends `negative[is.na(negative)] <-
/// FALSE`, then `pos_stack`/`position_fill` use `y <- ifelse(is.na(df$y),
/// 0, df$y)`) treat a missing row as joining the *positive* bucket but
/// contributing **0** to its running total, and leave the row's own
/// `y`/`ymin`/`ymax` as `NA` — so it draws nothing, but every *other* row
/// at that x is unaffected. [`StackRow::is_positive_bucket`] and
/// [`StackRow::contribution`] give exactly that: summing `contribution()`
/// into a bucket's total never introduces a NaN (review #167 finding 2 on
/// #171 — the pre-#171 code instead summed the raw, possibly-NaN `y`,
/// poisoning every other row sharing that row's bucket at that x).
#[derive(Clone, Copy)]
pub(crate) struct StackRow {
    value: Option<f64>,
}

/// Classify a row's `y` for stacking — see [`StackRow`].
pub(crate) fn stack_row(y: &crate::data::Value) -> StackRow {
    StackRow {
        value: y.as_f64().filter(|v| v.is_finite()),
    }
}

impl StackRow {
    /// Which of the two independent buckets this row stacks into — a
    /// missing row always joins the positive one, matching ggplot2.
    pub(crate) fn is_positive_bucket(&self) -> bool {
        self.value.is_none_or(|v| v >= 0.0)
    }

    /// What this row contributes to its bucket's running total: `0` for
    /// a missing row, so summing it can never poison the total with NaN.
    pub(crate) fn contribution(&self) -> f64 {
        self.value.unwrap_or(0.0)
    }

    /// This row's own finite value, or `None` if it's missing — the
    /// caller writes `f64::NAN` for `y`/`ymin`/`ymax` in the `None` case
    /// (so the row draws nothing, matching ggplot2's `NA`).
    pub(crate) fn value(&self) -> Option<f64> {
        self.value
    }
}

/// Column in which dodging positions store each row's offset (in category
/// units) when x is *discrete*: the category itself can't move, so geoms add
/// `offset / n_levels` to the mapped x (see `geom::support::XMapper`).
pub const DODGE_OFFSET_COL: &str = ".x_dodge_offset";
/// Number of dodged groups sharing a row's discrete x position (stored next to
/// [`DODGE_OFFSET_COL`]); bar-like geoms divide their width by it.
pub const DODGE_N_COL: &str = ".x_dodge_n";
