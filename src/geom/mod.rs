pub mod area;
pub mod bar;
pub mod bin2d;
pub mod blank;
pub mod boxplot;
pub mod bracket;
pub mod col;
pub mod contour;
pub mod count;
pub mod crossbar;
pub mod curve;
pub mod density;
pub mod density2d;
pub mod dotplot;
pub mod errorbar;
pub mod freqpoly;
pub mod hex;
pub mod histogram;
pub mod jitter;
pub mod line;
pub mod linerange;
pub mod path;
pub mod point;
pub mod pointrange;
pub mod polygon;
pub mod qq;
pub mod raster;
pub mod rect;
pub mod refline;
pub mod ribbon;
pub mod rug;
pub mod segment;
#[cfg(feature = "sf")]
pub mod sf;
pub mod smooth;
pub mod spoke;
pub mod step;
pub mod text;
pub mod tile;
pub mod violin;

use std::collections::HashMap;

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::Position;
use crate::render::backend::DrawBackend;
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::Stat;
use crate::theme::Theme;

/// Fixed (non-mapped) visual parameters for a geom.
#[derive(Clone, Debug, Default)]
pub struct GeomParams {
    pub values: HashMap<String, f64>,
    pub color: Option<(u8, u8, u8)>,
    pub fill: Option<(u8, u8, u8)>,
    pub alpha: Option<f64>,
}

/// Trait for geometric objects that draw data on the plot.
pub trait Geom: Send + Sync {
    /// Draw this geometry.
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError>;

    /// Required aesthetics.
    fn required_aes(&self) -> Vec<Aesthetic>;

    /// Default stat for this geom.
    fn default_stat(&self) -> Box<dyn Stat>;

    /// Default position adjustment.
    fn default_position(&self) -> Box<dyn Position>;

    /// Non-mapped visual defaults.
    fn default_params(&self) -> GeomParams;

    /// Name for debug/display.
    fn name(&self) -> &str;

    /// Apply a brand/primary color to this geom's single-series default (its
    /// `color` or `fill`). The build pipeline calls this only when the layer has
    /// no color/fill aesthetic mapped, so an explicit mapping always wins. The
    /// default is a no-op; series geoms override it.
    fn set_series_color(&mut self, _color: (u8, u8, u8)) {}

    /// Whether this geom draws from a 0 baseline, so the Y scale should include
    /// 0 even when `y` is explicitly mapped (bars/columns/area/histograms — as
    /// in ggplot2). Default false.
    fn include_zero_baseline(&self) -> bool {
        false
    }
}

/// Format a value for a hover tooltip — strings verbatim, numbers rounded short,
/// `Na` empty.
pub(crate) fn tip_value(v: &crate::data::Value) -> String {
    use crate::data::Value;
    match v {
        Value::Str(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Na => String::new(),
        // A datetime axis reads as a calendar date, not raw epoch seconds.
        Value::DateTime(secs) => crate::data::format_epoch_secs(*secs),
        _ => v
            .as_f64()
            .map(|f| format!("{}", (f * 1000.0).round() / 1000.0))
            .unwrap_or_default(),
    }
}
