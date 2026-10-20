use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::geom::arrow::{draw_heads, heads, Arrow};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineEnd, LineStyle, Linetype};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Segment geometry — line segments from (x, y) to (xend, yend).
pub struct GeomSegment {
    pub color: (u8, u8, u8),
    pub width: f64,
    pub alpha: f64,
    /// An optional arrowhead drawn at the segment's end(s) — see
    /// [`crate::geom::arrow::arrow`].
    pub arrow: Option<Arrow>,
    /// Line cap style (R grid's `lineend`).
    pub lineend: LineEnd,
}

impl Default for GeomSegment {
    fn default() -> Self {
        GeomSegment {
            color: (0, 0, 0),
            width: 1.0,
            alpha: 1.0,
            arrow: None,
            lineend: LineEnd::Butt,
        }
    }
}

impl GeomSegment {
    pub fn color(mut self, color: (u8, u8, u8)) -> Self {
        self.color = color;
        self
    }

    /// Sets `width` (the stroke width in pixels).
    pub fn linewidth(mut self, width: f64) -> Self {
        self.width = width;
        self
    }

    pub fn alpha(mut self, alpha: f64) -> Self {
        self.alpha = alpha;
        self
    }

    pub fn arrow(mut self, arrow: Arrow) -> Self {
        self.arrow = Some(arrow);
        self
    }

    pub fn lineend(mut self, lineend: LineEnd) -> Self {
        self.lineend = lineend;
        self
    }
}

impl Geom for GeomSegment {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let x_col = data
            .column("x")
            .ok_or(RenderError::MissingAesthetic("x".into()))?;
        let y_col = data
            .column("y")
            .ok_or(RenderError::MissingAesthetic("y".into()))?;
        let xend_col = data
            .column("xend")
            .ok_or(RenderError::MissingAesthetic("xend".into()))?;
        let yend_col = data
            .column("yend")
            .ok_or(RenderError::MissingAesthetic("yend".into()))?;
        let color_col = data.column("color");
        let linetype_col = data.column("linetype");
        let alpha_col = data.column("alpha");
        let linewidth_col = data.column("linewidth");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        for i in 0..data.nrows() {
            let nx1 = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
            let ny1 = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
            let nx2 = x_scale.map(|s| s.map(&xend_col[i])).unwrap_or(0.0);
            let ny2 = y_scale.map(|s| s.map(&yend_col[i])).unwrap_or(0.0);

            let (px1, py1) = coord.transform((nx1, ny1), &plot_area);
            let (px2, py2) = coord.transform((nx2, ny2), &plot_area);

            let line_color = color_col
                .and_then(|cc| scales.map_color(&Aesthetic::Color, &cc[i]))
                .unwrap_or(self.color);
            let lt = linetype_col
                .and_then(|lc| scales.map_linetype(&lc[i]))
                .unwrap_or(Linetype::Solid);
            let alpha = alpha_col
                .and_then(|ac| scales.map_alpha(&ac[i]).or_else(|| ac[i].as_f64()))
                .filter(|a| a.is_finite())
                .unwrap_or(self.alpha);
            let width = linewidth_col
                .and_then(|lc| scales.map_linewidth(&lc[i]))
                .unwrap_or(self.width);

            let path = [(px1, py1), (px2, py2)];
            backend.draw_line(
                &path,
                &LineStyle {
                    color: line_color,
                    alpha,
                    width,
                    linetype: lt,
                    lineend: self.lineend,
                },
            )?;

            if let Some(arrow) = &self.arrow {
                let hs = heads(&path, arrow, width);
                draw_heads(backend, &hs, arrow, line_color, alpha, width)?;
            }
        }

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y, Aesthetic::Xend, Aesthetic::Yend]
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
        "segment"
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.color = color;
    }
}

/// `geom_calendar`'s month outlines: segments styled at draw time by the
/// theme's [`calendar_month_border`](Theme::calendar_month_border), so a theme
/// set after the calendar still applies (a blank element hides them).
pub(crate) struct GeomCalendarBorder;

impl Geom for GeomCalendarBorder {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let el = &theme.calendar_month_border;
        if !el.visible {
            return Ok(());
        }
        GeomSegment {
            color: el.color,
            width: el.width,
            alpha: 1.0,
            ..Default::default()
        }
        .draw(data, coord, scales, theme, backend)
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        GeomSegment::default().required_aes()
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
        "segment"
    }
}

#[cfg(test)]
mod tests {
    use crate::data::Value;
    use crate::geom::arrow::{arrow, ArrowType};
    use crate::prelude::*;
    use crate::render::backend::LineEnd;

    fn plot() -> GGPlot {
        GGPlot::new(vec![
            ("x".into(), vec![Value::Float(0.0), Value::Float(0.2)]),
            ("y".into(), vec![Value::Float(0.0), Value::Float(1.0)]),
            ("xend".into(), vec![Value::Float(1.0), Value::Float(0.8)]),
            ("yend".into(), vec![Value::Float(0.5), Value::Float(0.3)]),
        ])
        .aes(Aes::new().x("x").y("y").xend("xend").yend("yend"))
    }

    #[test]
    fn arrow_closed_gives_one_tagged_polygon_per_row_with_fill_eq_stroke() {
        let svg = plot()
            .geom_segment_with(GeomSegment::default().arrow(arrow().closed()))
            .render_svg_native_with_size(200, 200)
            .unwrap();
        let count = svg.matches("data-part=\"arrow\"").count();
        assert_eq!(count, 2, "one arrowhead per row: {svg}");
        assert!(svg.contains("aria-hidden=\"true\""));
        // Each arrow mark is a polygon with fill == stroke.
        for part in svg.split("<polygon").skip(1) {
            if part.contains("data-part=\"arrow\"") {
                let fill = part
                    .split("fill=\"")
                    .nth(1)
                    .unwrap()
                    .split('"')
                    .next()
                    .unwrap();
                let stroke = part
                    .split("stroke=\"")
                    .nth(1)
                    .unwrap()
                    .split('"')
                    .next()
                    .unwrap();
                assert_eq!(fill, stroke);
            }
        }
    }

    #[test]
    fn arrow_open_uses_a_three_point_polyline_not_a_polygon() {
        let a = arrow(); // ArrowType::Open is the default.
        assert_eq!(a.kind, ArrowType::Open);
        let svg = plot()
            .geom_segment_with(GeomSegment::default().arrow(a))
            .render_svg_native_with_size(200, 200)
            .unwrap();
        assert!(svg.contains("data-part=\"arrow\""));
        assert!(!svg.contains("<polygon"));
    }

    #[test]
    fn lineend_round_gives_stroke_linecap_round() {
        let svg = plot()
            .geom_segment_with(GeomSegment::default().lineend(LineEnd::Round))
            .render_svg_native_with_size(200, 200)
            .unwrap();
        assert!(svg.contains("stroke-linecap=\"round\""));
    }

    #[test]
    fn default_segment_output_has_no_linecap_or_arrow_marks() {
        let svg = plot()
            .geom_segment_with(GeomSegment::default())
            .render_svg_native_with_size(200, 200)
            .unwrap();
        assert!(!svg.contains("stroke-linecap"));
        assert!(!svg.contains("data-part"));
    }
}
