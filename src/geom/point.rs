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

/// Scatter plot points.
pub struct GeomPoint {
    pub size: f64,
    pub color: (u8, u8, u8),
    pub alpha: f64,
}

impl Default for GeomPoint {
    fn default() -> Self {
        GeomPoint {
            size: 3.0,
            color: (0, 0, 0),
            alpha: 1.0,
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
}
