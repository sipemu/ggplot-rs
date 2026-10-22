//! Decorative-layer support (#161): a way to mark a layer as purely
//! decorative (e.g. a boxplot's own mean dot, drawn by a separate
//! `geom_point` layer) so its marks carry **no** per-mark metadata —
//! `<title>`, `data-x`/`data-series`/`data-value`/`data-fvalue`, and
//! `data_attr`-driven attributes — while keeping their own visual style.
//! Without this, any `Geom::draw` that calls [`super::set_mark`]/
//! [`super::set_row_extras`] gets full mark metadata unconditionally, so a
//! decorative mark looks like an independent, selectable data point to any
//! SVG consumer that keys off "has a non-empty `<title>`" (hover lists, a
//! per-x crosshair, click-to-select).
//!
//! [`GGPlot::decorative`](crate::plot::GGPlot::decorative) swaps the last
//! layer's `geom` for a private [`Decorative`] wrapper that delegates every
//! [`Geom`] method to the wrapped geom except `draw`, which runs the
//! wrapped geom against a [`MuteMarks`] backend wrapper turning the
//! per-mark-metadata setters into no-ops. This adds no field to `Layer`
//! (`Layer`'s fields are all `pub` with no `#[non_exhaustive]`, so a field
//! addition would be a semver break for a patch release) and no field to
//! any concrete geom.
//!
//! Nothing in the crate downcasts a `Box<dyn Geom>` (the `Geom` trait has
//! no `as_any`/`downcast`), so wrapping it this way is safe: no code path
//! needs to recover the original concrete geom type.

use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineStyle, PointStyle, RectStyle, TextStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Aesthetic, Geom, GeomParams};

/// A [`DrawBackend`] wrapper that forwards drawing and query calls
/// unchanged but turns every per-mark-metadata setter — tooltip, `data-x`/
/// `data-series`/`data-value`/`data-fvalue`, and `data_attr`-driven
/// attributes — into a no-op, so a decorative layer's marks keep their
/// visual style (`draw_circle`/`draw_line`/...) but get none of that
/// metadata.
struct MuteMarks<'a> {
    inner: &'a mut dyn DrawBackend,
}

impl DrawBackend for MuteMarks<'_> {
    fn draw_circle(
        &mut self,
        center: (f64, f64),
        radius: f64,
        style: &PointStyle,
    ) -> Result<(), RenderError> {
        self.inner.draw_circle(center, radius, style)
    }

    fn draw_line(&mut self, points: &[(f64, f64)], style: &LineStyle) -> Result<(), RenderError> {
        self.inner.draw_line(points, style)
    }

    fn draw_rect(
        &mut self,
        top_left: (f64, f64),
        bottom_right: (f64, f64),
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        self.inner.draw_rect(top_left, bottom_right, style)
    }

    fn draw_text(
        &mut self,
        text: &str,
        pos: (f64, f64),
        style: &TextStyle,
    ) -> Result<(), RenderError> {
        self.inner.draw_text(text, pos, style)
    }

    fn draw_polygon(
        &mut self,
        points: &[(f64, f64)],
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        self.inner.draw_polygon(points, style)
    }

    fn plot_area(&self) -> crate::render::Rect {
        self.inner.plot_area()
    }

    fn total_area(&self) -> crate::render::Rect {
        self.inner.total_area()
    }

    // Per-mark metadata: muted. `Aes::tooltip`/`Aes::data_attr` must have
    // no visible effect on a decorative mark, however the wrapped geom's
    // own `draw` calls `set_mark`/`set_row_extras`.
    fn set_tooltip(&mut self, _tooltip: Option<String>) {}
    fn set_mark_axis(&mut self, _key: Option<String>) {}
    fn set_mark_series(&mut self, _series: Option<String>) {}
    fn set_mark_value(&mut self, _value: Option<String>) {}
    fn set_mark_attrs(&mut self, _attrs: Vec<(String, String)>) {}
    fn set_mark_fvalue(&mut self, _value: Option<String>) {}
    fn set_mark_fx(&mut self, _value: Option<String>) {}

    // Not metadata about *this* mark being a data point — forwarded as-is.
    fn set_mark_geom(&mut self, geom: Option<&str>) {
        self.inner.set_mark_geom(geom);
    }

    fn set_mark_part(&mut self, part: Option<&str>) {
        self.inner.set_mark_part(part);
    }

    fn warn(&mut self, message: String) {
        self.inner.warn(message);
    }

    fn wants_formatted_values(&self) -> bool {
        self.inner.wants_formatted_values()
    }
}

/// Wraps a [`Geom`] so its drawn marks carry no per-mark metadata (#161).
/// See the module docs for why this is a wrapper rather than a field.
pub(crate) struct Decorative(pub(crate) Box<dyn Geom>);

impl Geom for Decorative {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let mut muted = MuteMarks { inner: backend };
        self.0.draw(data, coord, scales, theme, &mut muted)
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        self.0.required_aes()
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        self.0.default_stat()
    }

    fn default_position(&self) -> Box<dyn Position> {
        self.0.default_position()
    }

    fn default_params(&self) -> GeomParams {
        self.0.default_params()
    }

    fn name(&self) -> &str {
        self.0.name()
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.0.set_series_color(color);
    }

    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        self.0.apply_element_geom(el);
    }

    fn include_zero_baseline(&self) -> bool {
        self.0.include_zero_baseline()
    }

    fn allows_infinite(&self) -> bool {
        self.0.allows_infinite()
    }

    fn keeps_na_as_break(&self) -> bool {
        self.0.keeps_na_as_break()
    }

    // Always false: even if the wrapped geom reads `Aes::tooltip`, the
    // `MuteMarks` wrapper above suppresses the resulting `<title>`, so a
    // `.tooltip(...)` template on a decorative layer has no visible effect
    // — the build pipeline's existing has-no-effect warning should fire,
    // the same as for any other geom that doesn't consume tooltips.
    fn consumes_tooltips(&self) -> bool {
        false
    }

    fn setup_data(&self, data: &mut DataFrame, warnings: &mut Vec<String>) {
        self.0.setup_data(data, warnings);
    }

    fn inherit_aes(&self) -> bool {
        self.0.inherit_aes()
    }

    fn mapped_only_columns(&self) -> &'static [&'static str] {
        self.0.mapped_only_columns()
    }

    fn validate_layer(&self, pre_stat_columns: &[String], stat_name: &str) -> Result<(), String> {
        self.0.validate_layer(pre_stat_columns, stat_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::point::GeomPoint;

    #[test]
    fn decorative_delegates_name_and_required_aes() {
        let inner = GeomPoint::default();
        let name = inner.name().to_string();
        let required = inner.required_aes();
        let wrapped = Decorative(Box::new(inner));
        assert_eq!(wrapped.name(), name);
        assert_eq!(wrapped.required_aes(), required);
    }

    #[test]
    fn decorative_never_consumes_tooltips() {
        // GeomPoint itself consumes tooltips; the wrapper must not, since
        // the wrapped tooltip would have no visible effect.
        assert!(GeomPoint::default().consumes_tooltips());
        let wrapped = Decorative(Box::new(GeomPoint::default()));
        assert!(!wrapped.consumes_tooltips());
    }
}
