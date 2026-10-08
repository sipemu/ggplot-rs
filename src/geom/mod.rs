pub mod area;
pub mod bar;
pub mod bin2d;
pub mod blank;
pub mod boxplot;
pub mod bracket;
pub mod candlestick;
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

    /// Whether `-Inf`/`Inf` position values are meaningful for this geom
    /// (ggplot2: "extend to the panel edge"). Rows with infinite positions are
    /// otherwise dropped with a warning before stats run. `NaN` is always
    /// dropped. Default false; `geom_rect` returns true.
    fn allows_infinite(&self) -> bool {
        false
    }

    /// Geom-specific data preparation after the stat and position steps but
    /// before scale training (ggplot2's `GeomX$setup_data`) — e.g. a tile adds
    /// its `xmin`/`xmax`/`ymin`/`ymax` extents so continuous scales train on
    /// them. Default: no-op.
    fn setup_data(&self, _data: &mut DataFrame) {}
}

/// Format a value for a hover tooltip — strings verbatim, numbers rounded short,
/// `Na` empty.
pub(crate) fn tip_value(v: &crate::data::Value) -> String {
    crate::format::format_value(v)
}

/// Half-width (normalized panel units) for bars on a continuous / date x axis:
/// `width` × the smallest gap between distinct mapped x positions, like
/// ggplot2's `resolution(x)`. A fixed fraction would make 60 daily bars overlap
/// and 3 bars look like slivers. Falls back to `fallback` with < 2 distinct xs.
pub(crate) fn continuous_bar_half_width(
    mapped_xs: impl Iterator<Item = f64>,
    width: f64,
    fallback: f64,
) -> f64 {
    let mut xs: Vec<f64> = mapped_xs.filter(|x| x.is_finite()).collect();
    xs.sort_by(|a, b| a.total_cmp(b));
    xs.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
    let gap = xs
        .windows(2)
        .map(|w| w[1] - w[0])
        .fold(f64::INFINITY, f64::min);
    if gap.is_finite() && gap > 0.0 {
        gap * width / 2.0
    } else {
        fallback
    }
}

/// Raw (unformatted) value for a `data-value` attribute: numbers in shortest
/// round-trip form, date-times as epoch seconds, strings verbatim. `None` for
/// missing / non-finite values.
pub(crate) fn raw_value(v: &crate::data::Value) -> Option<String> {
    use crate::data::Value;
    match v {
        Value::Float(f) if f.is_finite() => Some(format!("{f}")),
        Value::Float(_) | Value::Na => None,
        Value::Integer(i) => Some(i.to_string()),
        Value::DateTime(s) => Some(s.to_string()),
        Value::Str(s) => Some(s.clone()),
        Value::Bool(b) => Some(b.to_string()),
    }
}

/// The series key of row `i` — its colour, else fill, else group level — for
/// a `data-series` attribute.
pub(crate) fn series_key(data: &DataFrame, i: usize) -> Option<String> {
    ["color", "fill", "group"].iter().find_map(|c| {
        data.column(c)
            .and_then(|col| col.get(i))
            .filter(|v| !v.is_na())
            .map(tip_value)
            .filter(|s| !s.is_empty())
    })
}

/// The raw measured y of row `i`. Stacked/filled positions overwrite `y` with
/// the cumulative top, so prefer the pre-position value they preserve.
pub(crate) fn measured_value(data: &DataFrame, i: usize) -> Option<String> {
    data.column(crate::position::RAW_Y_COL)
        .or_else(|| data.column("y"))
        .and_then(|c| c.get(i))
        .and_then(raw_value)
}

/// Set all per-mark metadata (tooltip, `data-x`, `data-series`, `data-value`)
/// for the next drawn mark(s).
pub(crate) fn set_mark(
    backend: &mut dyn DrawBackend,
    tooltip: Option<String>,
    x: Option<String>,
    series: Option<String>,
    value: Option<String>,
) {
    backend.set_tooltip(tooltip);
    backend.set_mark_axis(x);
    backend.set_mark_series(series);
    backend.set_mark_value(value);
}

/// Clear all per-mark metadata after a geom has drawn its marks.
pub(crate) fn clear_mark(backend: &mut dyn DrawBackend) {
    set_mark(backend, None, None, None, None);
}
