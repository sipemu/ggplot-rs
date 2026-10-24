#[cfg(feature = "sf")]
pub mod annotation_scale;
pub mod area;
pub mod arrow;
pub mod bar;
pub mod bin2d;
#[cfg(feature = "sf")]
pub mod bivariate_legend;
pub mod blank;
pub mod boxplot;
pub mod bracket;
pub mod candlestick;
pub mod censor;
pub mod col;
pub mod contour;
pub mod cooks;
pub mod count;
pub mod crossbar;
pub mod curve;
pub(crate) mod decorative;
pub mod density;
pub mod density2d;
pub mod dotplot;
pub mod errorbar;
pub mod errorbarh;
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
pub mod repel;
pub mod ribbon;
pub mod rug;
pub mod segment;
#[cfg(feature = "sf")]
pub mod sf;
#[cfg(feature = "sf")]
pub mod sf_text;
pub mod smooth;
pub mod spoke;
pub mod step;
pub(crate) mod support;
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

    /// Recolour this geom's *built-in default* colour/fill from the theme's
    /// ink/paper/accent palette (B2-7's `element_geom`), called once per
    /// layer before [`set_series_color`](Self::set_series_color) — and only
    /// when the layer was *not* configured explicitly (`geom_*_with(...)`;
    /// the build pipeline gates the call on the same `explicit_style` flag
    /// `set_series_color`'s brand-colour application is gated on). That
    /// gate, not a field-value comparison, is what keeps an explicit colour
    /// untouched: a geom that honours this replaces a field still at its
    /// own hard-coded default (black/`(50, 50, 50)` → `ink`, white →
    /// `paper`) by comparing its *value*, which cannot by itself tell an
    /// explicitly-set colour that happens to equal that default (e.g.
    /// `GeomPoint { color: (0, 0, 0), .. }` under `geom_point_with`) apart
    /// from one that was simply never touched — only skipping the call
    /// entirely for an explicitly-configured layer closes that gap (review
    /// PR#141 finding 4). Default no-op; `None` fields of `ElementGeom`
    /// change nothing, so the default theme (no ink/paper/accent set)
    /// stays byte-identical.
    fn apply_element_geom(&mut self, _el: &crate::theme::elements::ElementGeom) {}

    /// Whether this geom draws from a 0 baseline, so the Y scale should include
    /// 0 even when `y` is explicitly mapped (bars/columns/area/histograms — as
    /// in ggplot2). Default false.
    fn include_zero_baseline(&self) -> bool {
        false
    }

    /// Whether `-Inf`/`Inf` position values are meaningful for this geom
    /// (ggplot2: "extend to the panel edge"). Rows with infinite positions are
    /// otherwise dropped with a warning before stats run. `NaN` is always
    /// dropped — unless [`Geom::keeps_na_as_break`] says otherwise. Default
    /// false; `geom_rect` returns true.
    fn allows_infinite(&self) -> bool {
        false
    }

    /// Whether a row with a non-finite position is a *break* rather than a
    /// removal (ggplot2's `GeomPath`/`GeomLine` NA handling): the row still
    /// counts toward the "removed N rows" warning, but it is **not** deleted
    /// from the built `DataFrame`, so the geom's own per-group point list
    /// still contains it — as a non-finite pixel position after the scale
    /// and coordinate transform, which [`crate::render::backend::DrawBackend::draw_line`]
    /// splits on into separate polylines (ggplot2's NA-break semantics)
    /// instead of silently joining the finite points across the gap.
    /// Default false (the row is dropped, as for every other geom);
    /// line/path/step return true.
    fn keeps_na_as_break(&self) -> bool {
        false
    }

    /// Whether this geom calls [`set_row_extras`] (so it reads
    /// [`Aes::tooltip`](crate::aes::Aes::tooltip) and
    /// [`Aes::data_attr`](crate::aes::Aes::data_attr), and a
    /// [`GGPlot::tooltip`](crate::plot::GGPlot::tooltip) template on it has
    /// a visible effect). Default false; the build pipeline warns when a
    /// `tooltip()` template is set on a layer whose geom returns false
    /// here (review PR#141 finding 6, LOW: a template silently did nothing
    /// on e.g. `geom_area`/`geom_boxplot`/`geom_ribbon`). Point, segment,
    /// curve, path, line, hex, polygon, sf, rect/tile and col/bar/
    /// histogram return true.
    fn consumes_tooltips(&self) -> bool {
        false
    }

    /// Geom-specific data preparation after the stat and position steps but
    /// before scale training (ggplot2's `GeomX$setup_data`) — e.g. a tile adds
    /// its `xmin`/`xmax`/`ymin`/`ymax` extents so continuous scales train on
    /// them. `warnings` is the layer's own build-warnings sink (B3-5: added
    /// so `GeomBoxplot` can warn on an unparsable outlier value here, the
    /// same channel every other build warning uses — never `eprintln!`).
    /// Default: no-op.
    fn setup_data(&self, _data: &mut DataFrame, _warnings: &mut Vec<String>) {}

    /// Whether the layer inherits the plot-level aesthetic mapping (ggplot2's
    /// `inherit.aes`). Reference-line geoms return false, as in ggplot2, so a
    /// plot-level `color`/`x`/`y` mapping does not leak into a `geom_hline`.
    /// Default true.
    fn inherit_aes(&self) -> bool {
        true
    }

    /// Canonical column names this geom only reads when the user actually
    /// mapped the corresponding aesthetic (D3, B3-5) — never from a raw
    /// column that merely happens to share the name (e.g. a SQL alias
    /// literally called `lower`). `build_layer` drops any of these columns
    /// right after `resolve_mappings` unless the aesthetic was mapped.
    /// Default empty; `GeomBoxplot` returns `["lower", "middle", "upper",
    /// "outliers"]`.
    fn mapped_only_columns(&self) -> &'static [&'static str] {
        &[]
    }

    /// Stat-dependent layer validation beyond [`required_aes`](Self::required_aes)
    /// (B3-5): called once per layer with the resolved (pre-stat, post-D3-drop)
    /// column names and the layer's stat name, before the stat runs. Default
    /// no-op. `GeomBoxplot` requires `lower`/`middle`/`upper` to be mapped
    /// under `stat="identity"`, else `y`.
    fn validate_layer(&self, _pre_stat_columns: &[String], _stat_name: &str) -> Result<(), String> {
        Ok(())
    }
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
    measured_raw(data, i).and_then(raw_value)
}

/// [`measured_value`], but the raw `Value` itself (for `Scale::format_value`,
/// B2-4) instead of its display string.
pub(crate) fn measured_raw(data: &DataFrame, i: usize) -> Option<&crate::data::Value> {
    data.column(crate::position::RAW_Y_COL)
        .or_else(|| data.column("y"))
        .and_then(|c| c.get(i))
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

/// Resolve the font family for a text-drawing geom (text, label, repel,
/// bracket, annotations) — B2-6/B2-7's three-tier order: `geom_family`
/// (the geom's own `fontfamily`, e.g. `GeomText::with_fontfamily`) when
/// set, else `theme.geom.family` (B2-7's `element_geom`), else
/// `theme.text.family` (read after `Theme::resolve_inheritance`, so an
/// inherited value is already resolved), else `None` (the SVG backend's
/// own `"sans-serif"` default). `ElementText::family` defaults to
/// `"sans-serif"` itself and `theme.geom.family` defaults to `None`, so
/// with nothing configured anywhere this already returns
/// `Some("sans-serif")` — the same string the backend would have defaulted
/// to, so default output stays byte-identical.
pub(crate) fn resolve_font_family(geom_family: &str, theme: &Theme) -> Option<String> {
    if !geom_family.is_empty() {
        Some(geom_family.to_string())
    } else if let Some(family) = &theme.geom.family {
        Some(family.clone())
    } else if theme.text.family.is_empty() {
        None
    } else {
        Some(theme.text.family.clone())
    }
}

/// Clear all per-mark metadata after a geom has drawn its marks.
pub(crate) fn clear_mark(backend: &mut dyn DrawBackend) {
    set_mark(backend, None, None, None, None);
    backend.set_mark_fvalue(None);
    backend.set_mark_fx(None);
}

/// Set `data-fvalue`/`data-fx` (B2-4) for the mark `set_mark` just
/// configured, formatted through the relevant scale — `value_aes`'s (Y for
/// most geoms, Fill for hex/tile) for `data-fvalue`, X for `data-fx`.
/// Always sets both (even to `None`), so a row without a formattable value
/// doesn't inherit an earlier row's — the same per-row reset discipline as
/// [`set_row_extras`]. A no-op query (`wants_formatted_values`) short-
/// circuits the scale lookups entirely when
/// [`crate::plot::GGPlot::svg_formatted_values`] is off (the default), so
/// this costs nothing and changes nothing when never turned on.
pub(crate) fn set_mark_formatted(
    backend: &mut dyn DrawBackend,
    scales: &crate::scale::ScaleSet,
    x_raw: Option<&crate::data::Value>,
    value_aes: &Aesthetic,
    value_raw: Option<&crate::data::Value>,
) {
    if !backend.wants_formatted_values() {
        backend.set_mark_fvalue(None);
        backend.set_mark_fx(None);
        return;
    }
    let fvalue = value_raw
        .filter(|v| !v.is_na())
        .and_then(|v| scales.get(value_aes).and_then(|s| s.format_value(v)));
    backend.set_mark_fvalue(fvalue);
    let fx = x_raw
        .filter(|v| !v.is_na())
        .and_then(|v| scales.get(&Aesthetic::X).and_then(|s| s.format_value(v)));
    backend.set_mark_fx(fx);
}

/// Apply one row's interactive aesthetics (B1-7: `Aes::tooltip`,
/// `Aes::data_attr`) to the backend.
///
/// A validated `Aes::tooltip` mapping replaces the geom's own `<title>`
/// (call the geom's own `set_mark(...)` *before* this, so a custom tooltip
/// wins) for *every* row — including when this row's value is NA, which
/// explicitly resets the tooltip to `None` (no `<title>` at all, even if
/// the geom has its own default): once `Aes::tooltip` is mapped, it owns
/// the tooltip for the whole layer, the same row-by-row way `color` or
/// `size` would. It never inherits an earlier row's. It's read from
/// [`crate::aes::TOOLTIP_INTERNAL_NAME`], not a plain `"tooltip"` column
/// name, so an unrelated raw source column literally called `"tooltip"`
/// can never drive every mark's hover title (the same class of bug fixed
/// for `data_attr` below). `set_tooltip` is only called at all when that
/// internal column exists, so layers that never map `Aes::tooltip` don't
/// touch whatever the geom already set.
///
/// Every validated `data_attr` column becomes a `data-<name>` attribute.
/// These are stored under [`crate::aes::DATA_ATTR_INTERNAL_PREFIX`] rather
/// than scanned by a `"data-"` name prefix: a raw source column that merely
/// happens to be named like a data attribute (e.g. a SQL alias) was never
/// validated and must never reach `push_mark` (see
/// `aes::validated_data_attr_internal_name`, and `aes::mapping`'s defensive
/// drop of any raw column that could forge an internal name).
/// `set_mark_attrs` is *always* called, even with an empty list, so a row
/// with no attrs doesn't inherit an earlier row's.
pub(crate) fn set_row_extras(backend: &mut dyn DrawBackend, data: &DataFrame, row: usize) {
    if let Some(col) = data.column(crate::aes::TOOLTIP_INTERNAL_NAME) {
        let tip = col.get(row).filter(|v| !v.is_na()).map(tip_value);
        backend.set_tooltip(tip);
    }
    let mut attrs = Vec::new();
    for name in data.column_names() {
        if let Some(target) = name.strip_prefix(crate::aes::DATA_ATTR_INTERNAL_PREFIX) {
            if let Some(v) = data.column(name).and_then(|c| c.get(row)) {
                if !v.is_na() {
                    attrs.push((target.to_string(), tip_value(v)));
                }
            }
        }
    }
    backend.set_mark_attrs(attrs);
}

/// Clear interactive-aesthetic attributes *and* the tooltip after a layer
/// finishes drawing, so neither leaks into a later layer, an annotation, or
/// the legend's own background/key marks (which share the same backend
/// instance for the rest of the render).
pub(crate) fn clear_row_extras(backend: &mut dyn DrawBackend) {
    backend.set_tooltip(None);
    backend.set_mark_attrs(Vec::new());
}

/// Draw one group's polyline (path/line), honouring a per-point `linewidth`.
/// A constant width draws one polyline (byte-identical to the pre-linewidth
/// behaviour); a varying width falls back to drawing segment-by-segment,
/// since a single stroked polyline can't vary its width along its length.
/// An optional arrowhead is drawn once per polyline, using the width at its
/// last point (closest to where a default `ends(Last)` head sits).
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_polyline_with_linewidth(
    backend: &mut dyn DrawBackend,
    points: &[(f64, f64)],
    widths: &[f64],
    color: (u8, u8, u8),
    alpha: f64,
    linetype: crate::render::backend::Linetype,
    lineend: crate::render::backend::LineEnd,
    arrow: Option<&crate::geom::arrow::Arrow>,
    fallback_width: f64,
) -> Result<(), RenderError> {
    if points.len() < 2 {
        return Ok(());
    }
    let w0 = widths.first().copied().unwrap_or(fallback_width);
    let varies = widths.len() == points.len() && widths.iter().any(|&w| (w - w0).abs() > 1e-9);
    if varies {
        for i in 0..points.len() - 1 {
            backend.draw_line(
                &points[i..=i + 1],
                &crate::render::backend::LineStyle {
                    color,
                    alpha,
                    width: widths[i],
                    linetype,
                    lineend,
                },
            )?;
        }
    } else {
        backend.draw_line(
            points,
            &crate::render::backend::LineStyle {
                color,
                alpha,
                width: w0,
                linetype,
                lineend,
            },
        )?;
    }
    if let Some(arrow) = arrow {
        let resolved_width = widths.last().copied().unwrap_or(fallback_width);
        let hs = crate::geom::arrow::heads(points, arrow, resolved_width);
        crate::geom::arrow::draw_heads(backend, &hs, arrow, color, alpha, resolved_width)?;
    }
    Ok(())
}
