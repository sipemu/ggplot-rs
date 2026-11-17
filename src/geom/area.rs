use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::stack::PositionStack;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineStyle, Linetype, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::align::StatAlign;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Area geometry — filled polygon from line to x-axis baseline.
pub struct GeomArea {
    pub fill: (u8, u8, u8),
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub line_width: f64,
    /// How consecutive points are joined (straight by default; a smooth
    /// `areaspline` with [`Interpolate::MonotoneCubic`](super::support::Interpolate::MonotoneCubic)).
    pub interpolate: super::support::Interpolate,
    /// Fill each side of a threshold in its own colour (red below, green
    /// above), cut at the exact crossing; one outline. Along y, each part
    /// fills between the curve and the threshold.
    pub split: Option<super::support::SignSplit>,
}

impl Default for GeomArea {
    fn default() -> Self {
        GeomArea {
            fill: (97, 156, 255),
            color: (50, 50, 50),
            alpha: 0.4,
            line_width: 1.0,
            interpolate: super::support::Interpolate::Linear,
            split: None,
        }
    }
}

/// Polygons, each with its fill colour.
pub(crate) type Parts = Vec<(Vec<(f64, f64)>, (u8, u8, u8))>;

/// A band (upper curve over lower, in scale space) cut by `sp`: one polygon
/// per run, in its side's colour. Along y each run fills down (up) to the
/// threshold; along x the band is cut at `x = at`.
pub(crate) fn split_band(
    sp: &super::support::SignSplit,
    upper: &[(f64, f64)],
    lower: &[(f64, f64)],
    x_scale: Option<&dyn crate::scale::Scale>,
    y_scale: Option<&dyn crate::scale::Scale>,
) -> Parts {
    use super::support::{split_runs, SplitAxis};
    match sp.axis {
        SplitAxis::Y => {
            let at = y_scale.map_or(sp.at, |s| s.map(&Value::Float(sp.at)));
            split_runs(upper, SplitAxis::Y, at)
                .into_iter()
                .filter_map(|(above, mut run)| {
                    let (first, last) = (run.first()?.0, run.last()?.0);
                    run.extend([(last, at), (first, at)]);
                    Some((run, sp.colour(above)))
                })
                .collect()
        }
        SplitAxis::X => {
            let at = x_scale.map_or(sp.at, |s| s.map(&Value::Float(sp.at)));
            let lows = split_runs(lower, SplitAxis::X, at);
            split_runs(upper, SplitAxis::X, at)
                .into_iter()
                .zip(lows)
                .map(|((above, mut up), (_, low))| {
                    up.extend(low.into_iter().rev());
                    (up, sp.colour(above))
                })
                .collect()
        }
    }
}

impl Geom for GeomArea {
    fn key_fill(&self) -> Option<((u8, u8, u8), f64)> {
        Some((self.fill, self.alpha))
    }

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

        // A fill/color aesthetic (or an explicit group) splits the data into one
        // filled band per series — as in a grouped or stacked area chart.
        let group_col = data
            .column("fill")
            .or_else(|| data.column("color"))
            .or_else(|| data.column("group"));
        // `ymin` is set by position="stack"/"fill" (the bottom of each segment);
        // without it the band runs from the y=0 baseline.
        let ymin_col = data.column("ymin");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);
        let base_ny = y_scale.map(|s| s.map(&Value::Float(0.0))).unwrap_or(0.0);

        // Group row indices by the series key, preserving first-seen order (which
        // is the stacking order the position adjustment produced).
        let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
        match group_col {
            Some(gc) => {
                for (i, v) in gc.iter().enumerate() {
                    let key = v.to_group_key();
                    match groups.iter_mut().find(|(k, _)| k == &key) {
                        Some((_, idx)) => idx.push(i),
                        None => groups.push((key, vec![i])),
                    }
                }
            }
            None => groups.push((String::new(), (0..data.nrows()).collect())),
        }

        for (_, indices) in &groups {
            // Order this series left-to-right so the band is a clean polygon.
            let mut idx = indices.clone();
            idx.sort_by(|&a, &b| {
                let xa = x_scale.map(|s| s.map(&x_col[a])).unwrap_or(0.0);
                let xb = x_scale.map(|s| s.map(&x_col[b])).unwrap_or(0.0);
                xa.total_cmp(&xb)
            });

            let fill = group_col
                .and_then(|gc| scales.map_color(&Aesthetic::Fill, &gc[idx[0]]))
                .or_else(|| {
                    group_col.and_then(|gc| scales.map_color(&Aesthetic::Color, &gc[idx[0]]))
                })
                .unwrap_or(self.fill);

            let mut upper_raw: Vec<(f64, f64)> = Vec::with_capacity(idx.len());
            let mut lower_raw: Vec<(f64, f64)> = Vec::with_capacity(idx.len());
            for &i in &idx {
                let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
                let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
                let nb = ymin_col
                    .and_then(|c| c[i].as_f64())
                    .and_then(|v| y_scale.map(|s| s.map(&Value::Float(v))))
                    .unwrap_or(base_ny);
                upper_raw.push((nx, ny));
                lower_raw.push((nx, nb));
            }
            let smooth = |pts: &[(f64, f64)]| super::support::interpolate(pts, self.interpolate).0;
            let px = |pts: &[(f64, f64)]| -> Vec<(f64, f64)> {
                pts.iter()
                    .map(|&p| coord.transform(p, &plot_area))
                    .collect()
            };
            let (up_n, low_n) = (smooth(&upper_raw), smooth(&lower_raw));
            let upper = px(&up_n);
            // the filled parts: the whole band, or one per side of the split
            let parts = match &self.split {
                None => {
                    let mut poly = up_n.clone();
                    poly.extend(low_n.iter().rev());
                    vec![(poly, fill)]
                }
                Some(sp) => split_band(sp, &up_n, &low_n, x_scale, y_scale),
            };
            for (poly, colour) in parts {
                if poly.len() >= 3 {
                    backend.draw_polygon(
                        &px(&poly),
                        &RectStyle {
                            fill: Some(colour),
                            stroke: None,
                            stroke_width: 0.0,
                            alpha: self.alpha,
                            clip: true,
                            corner_radius: 0.0,
                        },
                    )?;
                }
            }
            if upper.len() >= 2 {
                // A single-series area keeps its dark outline; multi-series bands
                // outline in their own fill so stacked bands stay legible.
                let line_color = if groups.len() > 1 { fill } else { self.color };
                backend.draw_line(
                    &upper,
                    &LineStyle {
                        color: line_color,
                        alpha: 1.0,
                        width: self.line_width,
                        linetype: Linetype::Solid,
                        ..Default::default()
                    },
                )?;
            }
        }

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        // ggplot2 parity (#172): `geom_area()`'s default stat is
        // `stat_align()`, so series sampled at different x stack without
        // dipping where one has no row.
        Box::new(StatAlign)
    }
    fn default_position(&self) -> Box<dyn Position> {
        // ggplot2 parity (sipemu/ggplot-rs#171 review finding 5): `geom_area()`
        // defaults to `position_stack()`, same as `geom_col()`/`geom_bar()`
        // (#169, just above those geoms' own `default_position`). 0.23/0.24
        // left this as `PositionIdentity`, the same gap #169 fixed for
        // `geom_col()` — landing it in the same minor as #169 so every
        // default-position parity break is in one release.
        Box::new(PositionStack)
    }
    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }
    fn name(&self) -> &str {
        "area"
    }

    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let (true, Some(ink)) = (self.color == (50, 50, 50), el.ink) {
            self.color = ink;
        }
    }

    fn include_zero_baseline(&self) -> bool {
        true
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }
}
