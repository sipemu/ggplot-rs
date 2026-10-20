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

/// Path geometry — connects points in row order (no x-sort).
pub struct GeomPath {
    pub color: (u8, u8, u8),
    pub width: f64,
    pub alpha: f64,
    /// An optional arrowhead drawn at the end(s) of each drawn polyline
    /// (one head set per group).
    pub arrow: Option<Arrow>,
    /// Line cap style (R grid's `lineend`).
    pub lineend: LineEnd,
}

impl Default for GeomPath {
    fn default() -> Self {
        GeomPath {
            color: (0, 0, 0),
            width: 1.5,
            alpha: 1.0,
            arrow: None,
            lineend: LineEnd::Butt,
        }
    }
}

impl GeomPath {
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

impl Geom for GeomPath {
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

                let lt = linetype_col
                    .and_then(|lc| scales.map_linetype(&lc[first_idx]))
                    .unwrap_or(Linetype::Solid);

                let points: Vec<(f64, f64)> = indices
                    .iter()
                    .map(|&i| {
                        let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
                        let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
                        coord.transform((nx, ny), &plot_area)
                    })
                    .collect();
                let widths: Vec<f64> = indices
                    .iter()
                    .map(|&i| {
                        linewidth_col
                            .and_then(|lc| scales.map_linewidth(&lc[i]))
                            .unwrap_or(self.width)
                    })
                    .collect();

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

            let points: Vec<(f64, f64)> = (0..data.nrows())
                .map(|i| {
                    let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
                    let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
                    coord.transform((nx, ny), &plot_area)
                })
                .collect();
            let widths: Vec<f64> = (0..data.nrows())
                .map(|i| {
                    linewidth_col
                        .and_then(|lc| scales.map_linewidth(&lc[i]))
                        .unwrap_or(self.width)
                })
                .collect();

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
        "path"
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.color = color;
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

    /// Byte-identical default output: an exact polyline string recorded
    /// before adding `arrow`/`lineend` to `GeomPath`.
    #[test]
    fn default_path_output_is_unchanged() {
        let svg = plot()
            .geom_path_with(GeomPath::default())
            .render_svg_native_with_size(100, 60)
            .unwrap();
        let points = svg
            .split("<polyline")
            .filter_map(|p| p.split("points=\"").nth(1))
            .filter_map(|p| p.split('"').next())
            .find(|p| p.split(' ').count() == 6)
            .expect("the 6-point data polyline");
        assert_eq!(
            points,
            "64.32,53.23 73.41,36.72 82.50,22.45 91.60,12.35 100.69,7.77 109.78,9.35"
        );
        assert!(!svg.contains("stroke-linecap"));
        assert!(!svg.contains("data-part"));
    }

    #[test]
    fn path_with_arrow_tags_one_head_for_the_single_polyline() {
        let svg = plot()
            .geom_path_with(GeomPath::default().arrow(arrow().closed()))
            .render_svg_native_with_size(100, 60)
            .unwrap();
        assert_eq!(svg.matches("data-part=\"arrow\"").count(), 1);
    }
}
