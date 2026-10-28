use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, PointShape, PointStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Whether a point layer's marks are inked.
///
/// Points over a line often exist mainly as per-point hover and click
/// targets (each mark carries its `<title>` and `data-*` attributes). On a
/// dense series they swamp the line, so they can be drawn without ink: the
/// mark stays in the SVG with `fill-opacity="0"`, which browsers still
/// hit-test (`pointer-events: visiblePainted` counts a painted fill of any
/// opacity), so hover and clicks keep working.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PointVisibility {
    /// Every point is drawn (the default).
    #[default]
    Visible,
    /// No point is inked.
    Hidden,
    /// Visible while the panel's distinct x positions are at least
    /// `min_spacing_px` apart on average, hidden when denser. Decided per
    /// panel, so a facet with a short series keeps its dots.
    Auto { min_spacing_px: f64 },
}

impl PointVisibility {
    /// `Auto` with a 3 px average spacing: a 7-day series at 15-minute
    /// resolution (672 points) is hidden on any plot narrower than ~2000 px,
    /// a 12-point monthly series stays visible down to ~36 px.
    pub const AUTO: PointVisibility = PointVisibility::Auto {
        min_spacing_px: 3.0,
    };
}

/// Scatter plot points.
pub struct GeomPoint {
    pub size: f64,
    pub color: (u8, u8, u8),
    pub alpha: f64,
    /// Whether the marks are inked; see [`PointVisibility`].
    pub visibility: PointVisibility,
    /// When `visibility` hides the points: keep them as transparent hover
    /// and click targets (`true`, the default), or omit them from the
    /// output entirely (`false`, for static output where nothing hovers).
    pub hidden_targets: bool,
}

impl Default for GeomPoint {
    fn default() -> Self {
        GeomPoint {
            size: 3.0,
            color: (0, 0, 0),
            alpha: 1.0,
            visibility: PointVisibility::Visible,
            hidden_targets: true,
        }
    }
}

impl Geom for GeomPoint {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        draw_points(
            data,
            coord,
            scales,
            backend,
            PointDefaults {
                size: self.size,
                color: self.color,
                alpha: self.alpha,
                shape: PointShape::Circle,
                visibility: self.visibility,
                hidden_targets: self.hidden_targets,
            },
        )
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatIdentity)
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "point"
    }

    fn consumes_tooltips(&self) -> bool {
        true
    }

    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let (true, Some(ink)) = (self.color == (0, 0, 0), el.ink) {
            self.color = ink;
        }
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.color = color;
    }
}

/// Unmapped defaults for [`draw_points`].
pub(crate) struct PointDefaults {
    pub size: f64,
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub shape: PointShape,
    pub visibility: PointVisibility,
    pub hidden_targets: bool,
}

/// `true` when the panel's distinct x positions sit closer than
/// `min_spacing_px` on average. The span is measured between the
/// transformed first and last x (so a flipped coord measures along y).
fn too_dense(
    data: &DataFrame,
    x_col: &[crate::data::Value],
    xm: &super::support::XMapper<'_>,
    coord: &dyn Coord,
    plot_area: &crate::render::Rect,
    min_spacing_px: f64,
) -> bool {
    let mut xs: Vec<f64> = (0..data.nrows())
        .filter(|&i| !matches!(x_col[i], crate::data::Value::Na))
        .map(|i| xm.map(&x_col[i], i))
        .filter(|v| v.is_finite())
        .collect();
    xs.sort_by(f64::total_cmp);
    xs.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
    let (Some(&lo), Some(&hi)) = (xs.first(), xs.last()) else {
        return false;
    };
    if xs.len() < 2 {
        return false;
    }
    let (ax, ay) = coord.transform((lo, 0.5), plot_area);
    let (bx, by) = coord.transform((hi, 0.5), plot_area);
    let span = ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt();
    span / ((xs.len() - 1) as f64) < min_spacing_px
}

/// Draw one point per row (shared by `geom_point` and censor marks):
/// mapped `color`/`size`/`alpha`/`shape` win over `defaults`; the x position
/// honours a dodge offset; every point carries hover metadata.
pub(crate) fn draw_points(
    data: &DataFrame,
    coord: &dyn Coord,
    scales: &ScaleSet,
    backend: &mut dyn DrawBackend,
    defaults: PointDefaults,
) -> Result<(), RenderError> {
    let x_col = data
        .column("x")
        .ok_or(RenderError::MissingAesthetic("x".into()))?;
    let y_col = data
        .column("y")
        .ok_or(RenderError::MissingAesthetic("y".into()))?;
    let color_col = data.column("color");
    let size_col = data.column("size");
    let alpha_col = data.column("alpha");
    let shape_col = data.column("shape");
    let label_col = data.column("label");

    let plot_area = backend.plot_area();
    let xm = super::support::XMapper::new(data, scales);
    let y_scale = scales.get(&Aesthetic::Y);

    let hidden = match defaults.visibility {
        PointVisibility::Visible => false,
        PointVisibility::Hidden => true,
        PointVisibility::Auto { min_spacing_px } => {
            too_dense(data, x_col, &xm, coord, &plot_area, min_spacing_px)
        }
    };
    if hidden && !defaults.hidden_targets {
        return Ok(());
    }

    for i in 0..data.nrows() {
        // Drop missing values (ggplot2 removes rows with NA x/y). Without this
        // an NA maps to 0.0 and paints a spurious point at the panel baseline.
        if matches!(x_col[i], crate::data::Value::Na) || matches!(y_col[i], crate::data::Value::Na)
        {
            continue;
        }
        let nx = xm.map(&x_col[i], i);
        let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
        let (px, py) = coord.transform((nx, ny), &plot_area);

        // Hover tooltip: an explicit `label` with its y value ("series: 22"),
        // else "(x, y)" prefixed by group.
        let tip = match label_col
            .map(|c| super::tip_value(&c[i]))
            .filter(|s| !s.is_empty())
        {
            Some(l) => Some(format!("{l}: {}", super::tip_value(&y_col[i]))),
            None => {
                let xy = format!(
                    "({}, {})",
                    super::tip_value(&x_col[i]),
                    super::tip_value(&y_col[i])
                );
                match color_col
                    .map(|c| super::tip_value(&c[i]))
                    .filter(|s| !s.is_empty())
                {
                    Some(gr) => Some(format!("{gr}: {xy}")),
                    None => Some(xy),
                }
            }
        };
        super::set_mark(
            backend,
            tip,
            Some(super::tip_value(&x_col[i])),
            super::series_key(data, i),
            super::measured_value(data, i),
        );
        super::set_mark_formatted(
            backend,
            scales,
            Some(&x_col[i]),
            &Aesthetic::Y,
            super::measured_raw(data, i),
        );
        super::set_row_extras(backend, data, i);

        let (r, g, b) = if let Some(cc) = color_col {
            scales
                .map_color(&Aesthetic::Color, &cc[i])
                .unwrap_or(defaults.color)
        } else {
            defaults.color
        };

        let alpha = alpha_col
            .and_then(|c| scales.map_alpha(&c[i]))
            .or_else(|| alpha_col.and_then(|c| c[i].as_f64()))
            .unwrap_or(defaults.alpha);
        let alpha = if hidden { 0.0 } else { alpha };

        let size = size_col
            .and_then(|c| scales.map_size(&c[i]))
            .or_else(|| size_col.and_then(|c| c[i].as_f64()))
            .unwrap_or(defaults.size);

        let shape = shape_col
            .and_then(|c| scales.map_shape(&c[i]))
            .unwrap_or(defaults.shape);

        backend.draw_shape(
            (px, py),
            size,
            &PointStyle {
                color: (r, g, b),
                alpha,
                filled: true,
                shape,
            },
        )?;
    }
    super::clear_mark(backend);
    super::clear_row_extras(backend);

    Ok(())
}

#[cfg(test)]
mod interactive_aes_tests {
    use crate::data::Value;
    use crate::prelude::*;

    /// Review finding #1 (CRITICAL): a raw data column that merely happens
    /// to be named like a `data-*` attribute — never mapped through
    /// `Aes::data_attr` — must never reach the SVG as an attribute, since an
    /// adversarial column name (e.g. a SQL alias) could inject arbitrary
    /// attribute syntax.
    #[test]
    fn a_raw_column_named_like_a_data_attr_is_never_emitted_as_one() {
        let evil = "data-a onmouseover=alert(1) b";
        let svg = GGPlot::new(vec![
            ("x".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
            ("y".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
            (
                evil.into(),
                vec![Value::Str("v".into()), Value::Str("v".into())],
            ),
        ])
        .aes(Aes::new().x("x").y("y")) // no data_attr mapping at all
        .geom_point()
        .render_svg_native_with_size(200, 200)
        .unwrap();

        assert!(!svg.contains("onmouseover"));
        assert!(!svg.contains(evil));
        // Also: a column that merely *looks* like a validly-named data-attr
        // (no injection, but still unmapped) must not leak either, so
        // "layers without these aes are byte-identical" actually holds.
        let svg2 = GGPlot::new(vec![
            ("x".into(), vec![Value::Float(0.0)]),
            ("y".into(), vec![Value::Float(0.0)]),
            ("data-row".into(), vec![Value::Str("42".into())]),
        ])
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .render_svg_native_with_size(200, 200)
        .unwrap();
        assert!(!svg2.contains("data-row"));
    }

    #[test]
    fn missing_tooltip_or_attr_on_a_later_row_does_not_inherit_an_earlier_rows() {
        let svg = GGPlot::new(vec![
            ("x".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
            ("y".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
            ("tip".into(), vec![Value::Str("first".into()), Value::Na]),
            ("extra".into(), vec![Value::Str("A".into()), Value::Na]),
        ])
        .aes(
            Aes::new()
                .x("x")
                .y("y")
                .tooltip("tip")
                .data_attr("extra", "extra"),
        )
        .geom_point()
        .render_svg_native_with_size(200, 200)
        .unwrap();

        // Exactly one <title> (row 1's) and one data-extra (row 1's) — row 2
        // carries neither.
        assert_eq!(svg.matches("<title>first</title>").count(), 1);
        assert_eq!(svg.matches("data-extra=\"A\"").count(), 1);
        // Row 2's circle (the second <circle ...>) has no data-extra and no
        // <title> at all.
        let circles: Vec<&str> = svg.split("<circle").skip(1).collect();
        assert_eq!(circles.len(), 2);
        let second = circles[1].split("/>").next().unwrap();
        assert!(!second.contains("data-extra"));
        assert!(!circles[1].contains("<title>"));
    }

    #[test]
    fn tooltip_does_not_leak_into_a_later_layer_or_the_legend() {
        let svg = GGPlot::new(vec![
            ("x".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
            ("y".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
            (
                "g".into(),
                vec![Value::Str("a".into()), Value::Str("b".into())],
            ),
            (
                "tip".into(),
                vec![Value::Str("T1".into()), Value::Str("T2".into())],
            ),
        ])
        .aes(Aes::new().x("x").y("y").color("g").tooltip("tip"))
        .geom_point()
        .annotate_rect(0.2, 0.3, 0.2, 0.3)
        .render_svg_native_with_size(300, 300)
        .unwrap();

        // Only the two mapped point tooltips exist; the legend keys/
        // background and the annotation rect carry none of them.
        assert_eq!(svg.matches("<title>").count(), 2);
        assert!(svg.contains("<title>T1</title>"));
        assert!(svg.contains("<title>T2</title>"));
    }

    /// Review R-1 (blocking): an unmapped raw column literally named
    /// `"tooltip"` must not drive tooltips — the same class of bug fixed
    /// for `data_attr`, now also fixed for `Aes::tooltip`'s own plain name.
    #[test]
    fn a_raw_column_named_tooltip_with_no_mapping_sets_no_tooltips() {
        let with_raw_tooltip_col = GGPlot::new(vec![
            ("x".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
            ("y".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
            (
                "tooltip".into(),
                vec![Value::Str("sneaky 1".into()), Value::Str("sneaky 2".into())],
            ),
        ])
        .aes(Aes::new().x("x").y("y")) // no Aes::tooltip mapping
        .geom_point()
        .render_svg_native_with_size(200, 200)
        .unwrap();
        assert!(!with_raw_tooltip_col.contains("sneaky"));

        // Same dataset, without the raw column, for a baseline title count.
        let without = GGPlot::new(vec![
            ("x".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
            ("y".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
        ])
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .render_svg_native_with_size(200, 200)
        .unwrap();
        // geom_point's own default title is "(x, y)", present either way —
        // the raw "tooltip" column changed nothing about it.
        let title_count = |s: &str| s.matches("<title>").count();
        assert_eq!(title_count(&with_raw_tooltip_col), title_count(&without));
    }

    /// Review R-3 (low): a raw column that tries to *forge* the internal
    /// `data_attr`/`tooltip` storage name directly (NUL byte included —
    /// `GGPlot::new` accepts any `String`) is dropped by `resolve_mappings`,
    /// not carried through and misread as a validated attribute.
    #[test]
    fn a_raw_column_forging_the_internal_prefix_is_dropped() {
        let forged_data_attr = format!("{}data-spoof", crate::aes::DATA_ATTR_INTERNAL_PREFIX);
        let forged_tooltip = crate::aes::TOOLTIP_INTERNAL_NAME.to_string();
        let svg = GGPlot::new(vec![
            ("x".into(), vec![Value::Float(0.0)]),
            ("y".into(), vec![Value::Float(0.0)]),
            (forged_data_attr, vec![Value::Str("evil".into())]),
            (forged_tooltip, vec![Value::Str("evil tip".into())]),
        ])
        .aes(Aes::new().x("x").y("y"))
        .geom_point()
        .render_svg_native_with_size(200, 200)
        .unwrap();
        assert!(!svg.contains("data-spoof"));
        assert!(!svg.contains("evil"));
    }
}

#[cfg(test)]
mod visibility_tests {
    use super::PointVisibility;
    use crate::data::Value;
    use crate::prelude::*;

    fn series(n: usize) -> GGPlot {
        GGPlot::new(vec![
            ("x".into(), (0..n).map(|i| Value::Float(i as f64)).collect()),
            (
                "y".into(),
                (0..n).map(|i| Value::Float((i % 7) as f64)).collect(),
            ),
        ])
        .aes(Aes::new().x("x").y("y"))
        .geom_line()
    }

    fn points(svg: &str) -> Vec<&str> {
        svg.split("<circle").skip(1).collect()
    }

    fn render(n: usize, visibility: PointVisibility, hidden_targets: bool) -> String {
        series(n)
            .geom_point_with(GeomPoint {
                visibility,
                hidden_targets,
                ..Default::default()
            })
            .render_svg_native_with_size(400, 300)
            .unwrap()
    }

    #[test]
    fn auto_hides_a_dense_series_but_keeps_hover_targets() {
        let svg = render(672, PointVisibility::AUTO, true);
        let pts = points(&svg);
        assert_eq!(pts.len(), 672);
        assert!(pts.iter().all(|c| c.contains("fill-opacity=\"0.000\"")));
        // Each target still carries its tooltip.
        assert_eq!(svg.matches("<title>").count(), 672);
    }

    #[test]
    fn auto_keeps_a_sparse_series_visible() {
        let svg = render(12, PointVisibility::AUTO, true);
        let pts = points(&svg);
        assert_eq!(pts.len(), 12);
        assert!(pts.iter().all(|c| !c.contains("fill-opacity")));
    }

    #[test]
    fn auto_counts_distinct_x_not_rows() {
        // Two series sharing 40 x positions: ~8 px apart on a 400 px plot.
        let n = 40;
        let svg = GGPlot::new(vec![
            (
                "x".into(),
                (0..2 * n).map(|i| Value::Float((i % n) as f64)).collect(),
            ),
            (
                "y".into(),
                (0..2 * n).map(|i| Value::Float(i as f64)).collect(),
            ),
            (
                "g".into(),
                (0..2 * n)
                    .map(|i| Value::Str(if i < n { "a" } else { "b" }.into()))
                    .collect(),
            ),
        ])
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_point_with(GeomPoint {
            visibility: PointVisibility::AUTO,
            ..Default::default()
        })
        .render_svg_native_with_size(400, 300)
        .unwrap();
        assert!(points(&svg).iter().all(|c| !c.contains("fill-opacity")));
    }

    #[test]
    fn hidden_and_visible_override_the_rule_both_ways() {
        let hidden = render(12, PointVisibility::Hidden, true);
        assert!(points(&hidden)
            .iter()
            .all(|c| c.contains("fill-opacity=\"0.000\"")));
        let visible = render(672, PointVisibility::Visible, true);
        assert!(points(&visible).iter().all(|c| !c.contains("fill-opacity")));
    }

    #[test]
    fn hidden_points_can_be_omitted_entirely() {
        assert!(points(&render(672, PointVisibility::AUTO, false)).is_empty());
        assert!(points(&render(12, PointVisibility::Hidden, false)).is_empty());
        // A sparse series under Auto is still drawn.
        assert_eq!(points(&render(12, PointVisibility::AUTO, false)).len(), 12);
    }
}
