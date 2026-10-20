use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::geom::arrow::Arrow;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineEnd, Linetype};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{draw_polyline_with_linewidth, Geom, GeomParams};

/// Line geometry — connects points sorted by x.
pub struct GeomLine {
    pub color: (u8, u8, u8),
    pub width: f64,
    pub alpha: f64,
    /// An optional arrowhead drawn at the end(s) of each drawn polyline
    /// (one head set per group).
    pub arrow: Option<Arrow>,
    /// Line cap style (R grid's `lineend`).
    pub lineend: LineEnd,
}

impl Default for GeomLine {
    fn default() -> Self {
        GeomLine {
            color: (0, 0, 0),
            width: 1.5,
            alpha: 1.0,
            arrow: None,
            lineend: LineEnd::Butt,
        }
    }
}

impl GeomLine {
    /// Fixed stroke colour, used when no `color` aesthetic is mapped.
    pub fn color(mut self, color: (u8, u8, u8)) -> Self {
        self.color = color;
        self
    }

    /// Sets `width` (the stroke width in pixels).
    pub fn linewidth(mut self, width: f64) -> Self {
        self.width = width;
        self
    }

    /// Fixed opacity, used when no `alpha` aesthetic is mapped.
    pub fn alpha(mut self, alpha: f64) -> Self {
        self.alpha = alpha;
        self
    }

    /// Draw an arrowhead at the end(s) — see [`arrow()`](crate::geom::arrow::arrow).
    pub fn arrow(mut self, arrow: Arrow) -> Self {
        self.arrow = Some(arrow);
        self
    }

    /// Line cap style (R grid's `lineend`).
    pub fn lineend(mut self, lineend: LineEnd) -> Self {
        self.lineend = lineend;
        self
    }
}

impl Geom for GeomLine {
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
        let color_col = data.column("color");
        let linetype_col = data.column("linetype");
        let linewidth_col = data.column("linewidth");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        // If there's a color/group aesthetic, draw separate lines per group
        if let Some(cc) = color_col {
            let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
            for (i, v) in cc.iter().enumerate() {
                let key = v.to_group_key();
                if let Some(entry) = groups.iter_mut().find(|(k, _)| k == &key) {
                    entry.1.push(i);
                } else {
                    groups.push((key, vec![i]));
                }
            }

            for (_, indices) in &groups {
                let first_idx = indices[0];
                let line_color = scales
                    .map_color(&Aesthetic::Color, &cc[first_idx])
                    .unwrap_or(self.color);
                // Tag each line with its series (for hover + linked highlighting).
                let series = Some(super::tip_value(&cc[first_idx]));
                super::set_mark(backend, series.clone(), None, series, None);
                super::set_row_extras(backend, data, first_idx);

                let lt = linetype_col
                    .and_then(|lc| scales.map_linetype(&lc[first_idx]))
                    .unwrap_or(Linetype::Solid);

                // Sort indices by x value
                let mut sorted = indices.clone();
                sorted.sort_by(|&a, &b| {
                    let xa = x_scale.map(|s| s.map(&x_col[a])).unwrap_or(0.0);
                    let xb = x_scale.map(|s| s.map(&x_col[b])).unwrap_or(0.0);
                    xa.total_cmp(&xb)
                });

                let mut points: Vec<(f64, f64)> = sorted
                    .iter()
                    .map(|&i| {
                        let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
                        let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
                        coord.transform((nx, ny), &plot_area)
                    })
                    .collect();
                let mut widths: Vec<f64> = sorted
                    .iter()
                    .map(|&i| {
                        linewidth_col
                            .and_then(|lc| scales.map_linewidth(&lc[i]))
                            .unwrap_or(self.width)
                    })
                    .collect();
                close_for_radar(coord, &mut points);
                if points.len() > widths.len() {
                    widths.push(widths[0]);
                }

                draw_polyline_with_linewidth(
                    backend,
                    &points,
                    &widths,
                    line_color,
                    self.alpha,
                    lt,
                    self.lineend,
                    self.arrow.as_ref(),
                    self.width,
                )?;
            }
            super::clear_mark(backend);
            super::clear_row_extras(backend);
        } else {
            let lt = linetype_col
                .and_then(|lc| {
                    if lc.is_empty() {
                        None
                    } else {
                        scales.map_linetype(&lc[0])
                    }
                })
                .unwrap_or(Linetype::Solid);

            // Sort by x value
            let mut sorted_indices: Vec<usize> = (0..data.nrows()).collect();
            sorted_indices.sort_by(|&a, &b| {
                let xa = x_scale.map(|s| s.map(&x_col[a])).unwrap_or(0.0);
                let xb = x_scale.map(|s| s.map(&x_col[b])).unwrap_or(0.0);
                xa.total_cmp(&xb)
            });

            let mut points: Vec<(f64, f64)> = sorted_indices
                .iter()
                .map(|&i| {
                    let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
                    let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
                    coord.transform((nx, ny), &plot_area)
                })
                .collect();
            let mut widths: Vec<f64> = sorted_indices
                .iter()
                .map(|&i| {
                    linewidth_col
                        .and_then(|lc| scales.map_linewidth(&lc[i]))
                        .unwrap_or(self.width)
                })
                .collect();
            close_for_radar(coord, &mut points);
            if points.len() > widths.len() {
                widths.push(widths[0]);
            }

            if data.nrows() > 0 {
                super::set_row_extras(backend, data, 0);
            }
            draw_polyline_with_linewidth(
                backend,
                &points,
                &widths,
                self.color,
                self.alpha,
                lt,
                self.lineend,
                self.arrow.as_ref(),
                self.width,
            )?;
            super::clear_row_extras(backend);
        }

        Ok(())
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
        "line"
    }

    fn keeps_na_as_break(&self) -> bool {
        true
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.color = color;
    }
}

/// Under `coord_radar` a series is a closed loop around the spokes.
fn close_for_radar(coord: &dyn Coord, points: &mut Vec<(f64, f64)>) {
    if coord.is_radar() && points.len() >= 3 {
        let first = points[0];
        points.push(first);
    }
}

#[cfg(test)]
mod tests {
    use crate::data::Value;
    use crate::geom::arrow::arrow;
    use crate::prelude::*;

    fn plot() -> GGPlot {
        let n = 6;
        let x: Vec<Value> = (0..n).map(|i| Value::Float(i as f64)).collect();
        let y: Vec<Value> = (0..n)
            .map(|i| Value::Float((i as f64 * 0.37).sin()))
            .collect();
        GGPlot::new(vec![("x".into(), x), ("y".into(), y)]).aes(Aes::new().x("x").y("y"))
    }

    #[test]
    fn default_line_output_has_no_linecap_or_arrow_marks() {
        let svg = plot()
            .geom_line_with(GeomLine::default())
            .render_svg_native_with_size(100, 60)
            .unwrap();
        assert!(!svg.contains("stroke-linecap"));
        assert!(!svg.contains("data-part"));
    }

    #[test]
    fn line_with_arrow_tags_one_head_for_the_single_polyline() {
        let svg = plot()
            .geom_line_with(GeomLine::default().arrow(arrow().closed()))
            .render_svg_native_with_size(100, 60)
            .unwrap();
        assert_eq!(svg.matches("data-part=\"arrow\"").count(), 1);
    }

    #[test]
    fn lineend_round_gives_stroke_linecap_round() {
        let svg = plot()
            .geom_line_with(GeomLine::default().lineend(crate::render::backend::LineEnd::Round))
            .render_svg_native_with_size(100, 60)
            .unwrap();
        assert!(svg.contains("stroke-linecap=\"round\""));
    }

    /// `.color()`/`.alpha()`/`.linewidth()` builders actually change the
    /// rendered stroke (the ungrouped — no `color` aesthetic mapped — line).
    #[test]
    fn color_alpha_and_linewidth_builders_change_the_rendered_stroke() {
        let svg = plot()
            .geom_line_with(
                GeomLine::default()
                    .color((10, 20, 30))
                    .alpha(0.4)
                    .linewidth(5.0),
            )
            .render_svg_native_with_size(100, 60)
            .unwrap();
        assert!(svg.contains("stroke=\"#0A141E\""));
        assert!(svg.contains("stroke-opacity=\"0.400\""));
        assert!(svg.contains("stroke-width=\"5.00\""));
    }

    /// The grouped branch (a `color` aesthetic mapped): one polyline per
    /// group, each tagged with its series for hover grouping.
    #[test]
    fn grouped_line_draws_one_polyline_per_colour_group_with_series_tags() {
        let svg = GGPlot::new(vec![
            (
                "x".into(),
                vec![
                    Value::Float(0.0),
                    Value::Float(1.0),
                    Value::Float(0.0),
                    Value::Float(1.0),
                ],
            ),
            (
                "y".into(),
                vec![
                    Value::Float(0.0),
                    Value::Float(1.0),
                    Value::Float(1.0),
                    Value::Float(0.0),
                ],
            ),
            (
                "g".into(),
                vec![
                    Value::Str("a".into()),
                    Value::Str("a".into()),
                    Value::Str("b".into()),
                    Value::Str("b".into()),
                ],
            ),
        ])
        .aes(Aes::new().x("x").y("y").color("g"))
        .geom_line_with(GeomLine::default().lineend(crate::render::backend::LineEnd::Round))
        .render_svg_native_with_size(100, 100)
        .unwrap();

        assert_eq!(svg.matches("stroke-linecap=\"round\"").count(), 2);
        assert!(svg.contains("data-series=\"a\""));
        assert!(svg.contains("data-series=\"b\""));
    }

    /// `draw_polyline_with_linewidth` (B1-4): when the mapped `linewidth`
    /// varies within a group, a single stroked polyline can't represent
    /// that — it falls back to drawing segment-by-segment, one
    /// `<polyline>` per consecutive point pair, each at its own width.
    #[test]
    fn varying_linewidth_within_a_group_draws_segment_by_segment() {
        let svg = GGPlot::new(vec![
            (
                "x".into(),
                vec![Value::Float(0.0), Value::Float(1.0), Value::Float(2.0)],
            ),
            (
                "y".into(),
                vec![Value::Float(0.0), Value::Float(1.0), Value::Float(0.0)],
            ),
            (
                "w".into(),
                vec![Value::Float(1.0), Value::Float(5.0), Value::Float(9.0)],
            ),
        ])
        .aes(Aes::new().x("x").y("y").linewidth("w"))
        .geom_line()
        .scale_linewidth(ScaleLinewidthContinuous::new().with_range(2.0, 9.0))
        .render_svg_native_with_size(100, 100)
        .unwrap();

        // 3 points -> 2 segments, each its own 2-point polyline at the
        // width mapped for its *starting* point (1.0 -> width 1, 5.0 ->
        // width 5) — not one 3-point polyline at a single width.
        assert_eq!(svg.matches("stroke-width=\"2.00\"").count(), 1);
        assert_eq!(svg.matches("stroke-width=\"5.50\"").count(), 1);
        // The last point's mapped width (9) is never used as a stroke
        // width here — there's no arrow to resolve it for.
        assert!(!svg.contains("stroke-width=\"9.00\""));
    }
}
