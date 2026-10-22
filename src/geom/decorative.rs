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
    fn decorative_delegates_default_stat_position_and_params() {
        let plain = GeomPoint::default();
        let (stat_name, position_name) = (
            plain.default_stat().name().to_string(),
            plain.default_position().name().to_string(),
        );
        let wrapped = Decorative(Box::new(GeomPoint::default()));
        assert_eq!(wrapped.default_stat().name(), stat_name);
        assert_eq!(wrapped.default_position().name(), position_name);
        // `GeomParams` has no `PartialEq`; compare via `Debug`.
        assert_eq!(
            format!("{:?}", wrapped.default_params()),
            format!("{:?}", GeomPoint::default().default_params())
        );
    }

    #[test]
    fn decorative_never_consumes_tooltips() {
        // GeomPoint itself consumes tooltips; the wrapper must not, since
        // the wrapped tooltip would have no visible effect.
        assert!(GeomPoint::default().consumes_tooltips());
        let wrapped = Decorative(Box::new(GeomPoint::default()));
        assert!(!wrapped.consumes_tooltips());
    }

    #[test]
    fn decorative_delegates_set_series_color_and_apply_element_geom() {
        // Neither `Geom` method is observable from outside the trait
        // (no `as_any`/`Debug` on every geom), so exercise both through a
        // full render: a decorative `geom_line` with no explicit colour
        // still picks up the brand colour via `set_series_color`, and
        // still recolours from theme ink via `apply_element_geom` when
        // there's no brand colour — the same as a non-decorative layer.
        use crate::aes::Aes;
        use crate::data::Value;
        use crate::plot::GGPlot;
        let data = vec![
            ("x".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
            ("y".to_string(), vec![Value::Float(1.0), Value::Float(2.0)]),
        ];

        let brand_svg = GGPlot::new(data.clone())
            .aes(Aes::new().x("x").y("y"))
            .geom_line()
            .decorative()
            .primary_color((9, 9, 9))
            .render_svg_native_with_size(200, 200)
            .unwrap();
        assert!(brand_svg.contains("#090909"), "{brand_svg}");

        let ink_svg = GGPlot::new(data)
            .aes(Aes::new().x("x").y("y"))
            .geom_line()
            .decorative()
            .theme(Theme::default().with_ink_paper((10, 20, 30), (255, 255, 255)))
            .render_svg_native_with_size(200, 200)
            .unwrap();
        assert!(ink_svg.contains("#0A141E"), "{ink_svg}");
    }

    #[test]
    fn decorative_delegates_include_zero_baseline_allows_infinite_and_keeps_na_as_break() {
        use crate::geom::bar::GeomBar;
        use crate::geom::line::GeomLine;
        use crate::geom::rect::GeomRect;

        let bar = Decorative(Box::new(GeomBar::default()));
        assert!(bar.include_zero_baseline());
        assert!(!Decorative(Box::new(GeomPoint::default())).include_zero_baseline());

        let rect = Decorative(Box::new(GeomRect::default()));
        assert!(rect.allows_infinite());
        assert!(!Decorative(Box::new(GeomPoint::default())).allows_infinite());

        let line = Decorative(Box::new(GeomLine::default()));
        assert!(line.keeps_na_as_break());
        assert!(!Decorative(Box::new(GeomPoint::default())).keeps_na_as_break());
    }

    #[test]
    fn decorative_delegates_inherit_aes() {
        use crate::geom::refline::GeomHline;
        // Reference lines return false; an ordinary geom returns true.
        assert!(!Decorative(Box::new(GeomHline::new(1.0))).inherit_aes());
        assert!(Decorative(Box::new(GeomPoint::default())).inherit_aes());
    }

    #[test]
    fn decorative_delegates_mapped_only_columns_and_validate_layer() {
        use crate::geom::boxplot::GeomBoxplot;
        let wrapped = Decorative(Box::new(GeomBoxplot::default()));
        assert_eq!(
            wrapped.mapped_only_columns(),
            GeomBoxplot::default().mapped_only_columns()
        );
        // stat="identity" with none of the five-number-summary columns
        // mapped must still error through the wrapper.
        assert!(wrapped.validate_layer(&[], "identity").is_err());
        assert!(wrapped
            .validate_layer(&["y".to_string()], "boxplot")
            .is_ok());
    }

    #[test]
    fn decorative_delegates_setup_data() {
        use crate::data::Value;
        use crate::geom::boxplot::GeomBoxplot;

        fn frame() -> DataFrame {
            let mut d = DataFrame::new();
            d.add_column("outliers".to_string(), vec![Value::Str("1,2".to_string())]);
            d.add_column("ymin".to_string(), vec![Value::Float(0.0)]);
            d.add_column("ymax".to_string(), vec![Value::Float(10.0)]);
            d
        }

        let mut plain_data = frame();
        let mut plain_warnings = Vec::new();
        GeomBoxplot::default().setup_data(&mut plain_data, &mut plain_warnings);

        let mut wrapped_data = frame();
        let mut wrapped_warnings = Vec::new();
        Decorative(Box::new(GeomBoxplot::default()))
            .setup_data(&mut wrapped_data, &mut wrapped_warnings);

        assert_eq!(
            wrapped_data.column_names(),
            plain_data.column_names(),
            "setup_data must reach the wrapped geom, adding the same columns"
        );
    }

    /// A minimal `DrawBackend` that records every call, so `MuteMarks`'s
    /// forwarding (drawing/query calls) and muting (per-mark metadata) can
    /// be asserted directly without going through a full plot render.
    #[derive(Default)]
    struct MockBackend {
        circles: usize,
        lines: usize,
        rects: usize,
        texts: usize,
        polygons: usize,
        mark_geom_calls: usize,
        mark_part_calls: usize,
        warn_calls: usize,
        tooltip_calls: usize,
        mark_axis_calls: usize,
        mark_series_calls: usize,
        mark_value_calls: usize,
        mark_attrs_calls: usize,
        mark_fvalue_calls: usize,
        mark_fx_calls: usize,
    }

    impl DrawBackend for MockBackend {
        fn draw_circle(
            &mut self,
            _center: (f64, f64),
            _radius: f64,
            _style: &PointStyle,
        ) -> Result<(), RenderError> {
            self.circles += 1;
            Ok(())
        }
        fn draw_line(
            &mut self,
            _points: &[(f64, f64)],
            _style: &LineStyle,
        ) -> Result<(), RenderError> {
            self.lines += 1;
            Ok(())
        }
        fn draw_rect(
            &mut self,
            _tl: (f64, f64),
            _br: (f64, f64),
            _style: &RectStyle,
        ) -> Result<(), RenderError> {
            self.rects += 1;
            Ok(())
        }
        fn draw_text(
            &mut self,
            _text: &str,
            _pos: (f64, f64),
            _style: &TextStyle,
        ) -> Result<(), RenderError> {
            self.texts += 1;
            Ok(())
        }
        fn draw_polygon(
            &mut self,
            _points: &[(f64, f64)],
            _style: &RectStyle,
        ) -> Result<(), RenderError> {
            self.polygons += 1;
            Ok(())
        }
        fn plot_area(&self) -> crate::render::Rect {
            crate::render::Rect {
                x: 1.0,
                y: 2.0,
                width: 3.0,
                height: 4.0,
            }
        }
        fn total_area(&self) -> crate::render::Rect {
            crate::render::Rect {
                x: 5.0,
                y: 6.0,
                width: 7.0,
                height: 8.0,
            }
        }
        fn set_tooltip(&mut self, _tooltip: Option<String>) {
            self.tooltip_calls += 1;
        }
        fn set_mark_axis(&mut self, _key: Option<String>) {
            self.mark_axis_calls += 1;
        }
        fn set_mark_series(&mut self, _series: Option<String>) {
            self.mark_series_calls += 1;
        }
        fn set_mark_value(&mut self, _value: Option<String>) {
            self.mark_value_calls += 1;
        }
        fn set_mark_geom(&mut self, _geom: Option<&str>) {
            self.mark_geom_calls += 1;
        }
        fn warn(&mut self, _message: String) {
            self.warn_calls += 1;
        }
        fn set_mark_part(&mut self, _part: Option<&str>) {
            self.mark_part_calls += 1;
        }
        fn set_mark_attrs(&mut self, _attrs: Vec<(String, String)>) {
            self.mark_attrs_calls += 1;
        }
        fn wants_formatted_values(&self) -> bool {
            true
        }
        fn set_mark_fvalue(&mut self, _value: Option<String>) {
            self.mark_fvalue_calls += 1;
        }
        fn set_mark_fx(&mut self, _value: Option<String>) {
            self.mark_fx_calls += 1;
        }
    }

    #[test]
    fn mute_marks_forwards_drawing_and_query_calls() {
        let mut inner = MockBackend::default();
        {
            let mut muted = MuteMarks { inner: &mut inner };
            muted
                .draw_circle((0.0, 0.0), 1.0, &PointStyle::default())
                .unwrap();
            muted
                .draw_line(&[(0.0, 0.0), (1.0, 1.0)], &LineStyle::default())
                .unwrap();
            muted
                .draw_rect((0.0, 0.0), (1.0, 1.0), &RectStyle::default())
                .unwrap();
            muted
                .draw_text("hi", (0.0, 0.0), &TextStyle::default())
                .unwrap();
            muted
                .draw_polygon(&[(0.0, 0.0)], &RectStyle::default())
                .unwrap();
            assert_eq!(muted.plot_area().x, 1.0);
            assert_eq!(muted.total_area().x, 5.0);
            muted.set_mark_geom(Some("point"));
            muted.set_mark_part(Some("arrow"));
            muted.warn("oops".to_string());
            assert!(muted.wants_formatted_values());
        }
        assert_eq!(inner.circles, 1);
        assert_eq!(inner.lines, 1);
        assert_eq!(inner.rects, 1);
        assert_eq!(inner.texts, 1);
        assert_eq!(inner.polygons, 1);
        assert_eq!(inner.mark_geom_calls, 1);
        assert_eq!(inner.mark_part_calls, 1);
        assert_eq!(inner.warn_calls, 1);
    }

    #[test]
    fn mute_marks_mutes_per_mark_metadata() {
        let mut inner = MockBackend::default();
        {
            let mut muted = MuteMarks { inner: &mut inner };
            muted.set_tooltip(Some("tip".to_string()));
            muted.set_mark_axis(Some("1".to_string()));
            muted.set_mark_series(Some("a".to_string()));
            muted.set_mark_value(Some("1".to_string()));
            muted.set_mark_attrs(vec![("x".to_string(), "y".to_string())]);
            muted.set_mark_fvalue(Some("1".to_string()));
            muted.set_mark_fx(Some("1".to_string()));
        }
        // None of these reached the inner backend.
        assert_eq!(inner.tooltip_calls, 0);
        assert_eq!(inner.mark_axis_calls, 0);
        assert_eq!(inner.mark_series_calls, 0);
        assert_eq!(inner.mark_value_calls, 0);
        assert_eq!(inner.mark_attrs_calls, 0);
        assert_eq!(inner.mark_fvalue_calls, 0);
        assert_eq!(inner.mark_fx_calls, 0);
    }

    #[test]
    fn mock_backend_itself_records_the_metadata_calls_mute_marks_suppresses() {
        // Sanity on the mock used above: called directly (not through
        // `MuteMarks`), its own per-mark-metadata methods do record —
        // confirming `mute_marks_mutes_per_mark_metadata`'s zero counts
        // come from `MuteMarks` muting them, not from the mock being a
        // no-op itself.
        let mut backend = MockBackend::default();
        backend.set_tooltip(Some("tip".to_string()));
        backend.set_mark_axis(Some("1".to_string()));
        backend.set_mark_series(Some("a".to_string()));
        backend.set_mark_value(Some("1".to_string()));
        backend.set_mark_attrs(vec![("x".to_string(), "y".to_string())]);
        backend.set_mark_fvalue(Some("1".to_string()));
        backend.set_mark_fx(Some("1".to_string()));
        assert_eq!(backend.tooltip_calls, 1);
        assert_eq!(backend.mark_axis_calls, 1);
        assert_eq!(backend.mark_series_calls, 1);
        assert_eq!(backend.mark_value_calls, 1);
        assert_eq!(backend.mark_attrs_calls, 1);
        assert_eq!(backend.mark_fvalue_calls, 1);
        assert_eq!(backend.mark_fx_calls, 1);
    }

    #[test]
    fn decorative_draw_runs_the_wrapped_geom_through_mute_marks() {
        // End-to-end sanity: `Decorative::draw` actually constructs a
        // `MuteMarks` around the given backend and calls the wrapped
        // geom's own `draw`, rather than skipping it.
        use crate::coord::cartesian::CoordCartesian;
        use crate::data::Value;

        let mut data = DataFrame::new();
        data.add_column("x".to_string(), vec![Value::Float(1.0)]);
        data.add_column("y".to_string(), vec![Value::Float(1.0)]);
        let coord = CoordCartesian::default();
        let scales = ScaleSet::default();
        let theme = Theme::default();
        let mut backend = MockBackend::default();
        let wrapped = Decorative(Box::new(GeomPoint::default()));
        wrapped
            .draw(&data, &coord, &scales, &theme, &mut backend)
            .unwrap();
        assert_eq!(
            backend.circles, 1,
            "the wrapped geom's own mark must still be drawn"
        );
        assert_eq!(
            backend.tooltip_calls, 0,
            "but its tooltip call must be muted"
        );
    }
}
