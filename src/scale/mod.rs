pub mod alpha;
pub mod color;
pub mod continuous;
pub mod datetime;
pub mod discrete;
pub mod format;
pub mod gradient;
pub mod gradient_n;
pub mod linetype;
pub mod manual;
pub mod palettes;
pub mod scale_set;
pub mod sec_axis;
pub mod shape;
pub mod size;
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

    /// Override the trained domain limits (used by coord_cartesian zoom).
    fn set_limits(&mut self, _min: f64, _max: f64) {
        // Default no-op. Continuous scales override this.
    }
}
