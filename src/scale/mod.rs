pub mod alpha;
pub mod color;
pub mod continuous;
pub mod datetime;
pub mod discrete;
pub mod format;
pub mod gradient;
pub mod gradient_n;
pub mod grey;
pub mod identity;
pub mod linetype;
pub mod linetype_manual;
pub mod linewidth;
pub mod manual;
pub mod modified;
pub mod palettes;
pub mod scale_set;
pub mod sec_axis;
pub mod shape;
pub mod shape_manual;
pub mod size;
pub mod steps;
pub mod transform;
pub mod util;

pub use scale_set::ScaleSet;

use crate::aes::Aesthetic;
use crate::data::Value;
use crate::render::backend::{Linetype, PointShape};

/// Trait for scales that map data values to visual properties.
pub trait Scale: Send + Sync {
    /// Which aesthetic this scale is for.
    fn aesthetic(&self) -> Aesthetic;

    /// Incorporate data values to determine domain.
    fn train(&mut self, values: &[Value]);

    /// Map a data value to a [0, 1] normalized position (position scales)
    /// or to a concrete visual value index (color/size scales).
    fn map(&self, value: &Value) -> f64;

    /// Generate break positions and labels for axis/legend.
    fn breaks(&self) -> Vec<(f64, String)>;

    /// Human-readable name (axis title).
    fn name(&self) -> &str;

    /// Set the scale name.
    fn set_name(&mut self, name: &str);

    /// Apply transformation to raw data (e.g., log10).
    fn transform(&self, value: &Value) -> Value {
        value.clone()
    }

    /// Whether this is a discrete scale.
    fn is_discrete(&self) -> bool {
        false
    }

    /// Map a data value to an RGB color. Default returns None.
    fn map_to_color(&self, _value: &Value) -> Option<(u8, u8, u8)> {
        None
    }

    /// Map a data value to a point shape. Default returns None.
    fn map_to_shape(&self, _value: &Value) -> Option<PointShape> {
        None
    }

    /// Map a data value to a linetype. Default returns None.
    fn map_to_linetype(&self, _value: &Value) -> Option<Linetype> {
        None
    }

    /// Map a data value to a point size (radius in pixels). Default returns None.
    fn map_to_size(&self, _value: &Value) -> Option<f64> {
        None
    }

    /// Map a data value to an alpha (opacity) value. Default returns None.
    fn map_to_alpha(&self, _value: &Value) -> Option<f64> {
        None
    }

    /// Get the secondary axis specification, if any.
    fn sec_axis(&self) -> Option<&sec_axis::SecAxis> {
        None
    }

    /// Adopt a plot-level default [`Locale`](crate::locale::Locale) for the
    /// default break labels (number marks, month names, date formats), unless
    /// the scale already has its own. Called by the plot builder when
    /// [`GGPlot::locale`](crate::GGPlot::locale) is set. Default: ignored
    /// (scales without numeric/date labels).
    fn set_default_locale(&mut self, _locale: &crate::locale::Locale) {}

    /// The locale this scale's default labels are rendered with, if any.
    fn locale(&self) -> Option<&crate::locale::Locale> {
        None
    }

    /// Format a single raw (pre-scale-transform) data `value` for a
    /// per-mark attribute (`data-fvalue`/`data-fx`, B2-4) or a tooltip
    /// template placeholder — e.g. a de-DE currency string, or a datetime
    /// x's ISO/locale date, independent of how the axis itself abbreviates
    /// its break labels. `None` for a missing/non-finite value, or a scale
    /// with nothing meaningful to format (default: every scale except
    /// continuous position/size/linewidth and datetime/colour-continuous,
    /// which override this).
    fn format_value(&self, _value: &Value) -> Option<String> {
        None
    }

    /// Override the trained domain limits (used by coord_cartesian zoom).
    fn set_limits(&mut self, _min: f64, _max: f64) {
        // Default no-op. Continuous scales override this.
    }

    /// Return OOB filter limits if this scale was created with explicit limits
    /// (e.g., via xlim/ylim). Data outside these limits should be removed before stats.
    fn filter_limits(&self) -> Option<(f64, f64)> {
        None
    }

    /// Return the trained data domain (min, max) for continuous scales.
    /// Used by the colorbar legend to pass data-space values to map_to_color().
    fn domain(&self) -> Option<(f64, f64)> {
        None
    }

    /// The data values at the panel edges (normalized 0 and 1) for a trained
    /// continuous position scale — the limits after expansion, in the scale's
    /// transformed space. `None` for discrete / untrained / non-position scales.
    fn expanded_domain(&self) -> Option<(f64, f64)> {
        None
    }

    /// Whether this axis is drawn on the opposite side (x → top, y → right).
    fn axis_position_opposite(&self) -> bool {
        false
    }

    /// Set the expansion multiplier/additive constant (R's `expand =
    /// c(mult, add)`), applied symmetrically to both ends. Used by
    /// `CoordSf::train_scales` (`expand(false)` passes `(0.0, 0.0)`) to pin
    /// an explicit `xlim`/`ylim` to the panel exactly. Default: no-op
    /// (discrete / non-position scales have no expansion to set).
    fn set_expand(&mut self, _mult: f64, _add: f64) {}

    /// Map an already-computed `[0, 1]` scale fraction (as returned
    /// alongside each label by [`breaks`](Self::breaks)) directly to an
    /// output size, bypassing the round trip through [`map`](Self::map) — a
    /// discrete-style legend key only has the break's rendered *label*, not
    /// its original data value, so a continuous scale can't recover the
    /// fraction by re-mapping the label text. Default: `None`.
    fn map_fraction_to_size(&self, _fraction: f64) -> Option<f64> {
        None
    }

    /// Clone this scale into a boxed trait object.
    fn clone_box(&self) -> Box<dyn Scale>;

    /// Reset training state so the scale can be retrained on new data.
    fn reset_training(&mut self) {}

    /// Validate this scale's configuration once training is complete (e.g.
    /// a transform undefined at the domain's own limits) and push a
    /// human-readable warning for each problem found. Called once per scale
    /// after the final training pass, with the plot's warnings sink —
    /// `map`/`map_to_size` run per mark at draw time and have no such
    /// access, so a scale-level problem must be reported here instead.
    /// Default: no-op.
    fn validate(&self, _warnings: &mut Vec<String>) {}
}

#[cfg(test)]
mod default_trait_method_tests {
    use super::Scale;
    use crate::aes::Aesthetic;
    use crate::data::Value;

    /// Scales that don't override `set_expand`/`map_fraction_to_size`
    /// (e.g. an identity scale, which has no expansion or numeric output
    /// at all) get the documented no-op/`None` defaults rather than a
    /// missing-method compile error.
    #[test]
    fn set_expand_and_map_fraction_to_size_defaults_are_harmless() {
        let mut scale = crate::scale::identity::ScaleColorIdentity::new(Aesthetic::Color);
        // No panic, no effect — there's no expansion concept for a
        // colour-identity scale.
        scale.set_expand(0.1, 5.0);
        assert_eq!(scale.map_fraction_to_size(0.5), None);
        // validate() is also a no-op default; an identity scale never has
        // anything to warn about.
        let mut warnings = Vec::new();
        scale.validate(&mut warnings);
        assert!(warnings.is_empty());
        // The scale is otherwise unaffected.
        assert_eq!(
            scale.map_to_color(&Value::Str("#FFFFFF".into())),
            Some((255, 255, 255))
        );
    }
}
