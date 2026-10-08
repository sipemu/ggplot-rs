//! Cook's-distance contours for a residuals-vs-leverage plot (R's
//! `plot.lm(which = 5)`): with `x` = leverage `h` and `y` = standardized
//! residual `r`, Cook's distance is `D = r² h / (p (1 − h))`, so the points
//! with `D = level` lie on `r = ±√(level · p · (1 − h) / h)`.
//!
//! The curves are drawn dashed across the panel's leverage range (or
//! `h_range`), clipped to the panel and labelled with their level. They train
//! no scale — like `plot.lm`, they never widen the axes.

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineStyle, Linetype, TextAnchor, TextStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::support::clip_polyline_unit;
use super::{Geom, GeomParams};

/// The standardized residual on the Cook's-distance contour `level` at
/// leverage `h` for a model with `p` parameters: `√(level · p · (1 − h) / h)`
/// (NaN outside `0 < h ≤ 1` or for a non-positive level / `p`).
pub fn cooks_contour_y(level: f64, p: f64, h: f64) -> f64 {
    if !(h > 0.0 && h <= 1.0 && level > 0.0 && p > 0.0) {
        return f64::NAN;
    }
    (level * p * (1.0 - h) / h).sqrt()
}

/// Cook's distance `D = r² h / (p (1 − h))` of a point with standardized
/// residual `r` and leverage `h` (the inverse of [`cooks_contour_y`]).
pub fn cooks_distance(r: f64, h: f64, p: f64) -> f64 {
    r * r * h / (p * (1.0 - h))
}

/// Cook's-distance contour curves (`±√(level·p·(1−h)/h)`), one pair per level.
pub struct GeomCooksContour {
    /// Number of model parameters (incl. the intercept).
    pub p: f64,
    /// Contour levels (R's `cook.levels`, default `[0.5, 1.0]`).
    pub levels: Vec<f64>,
    /// Leverage range to draw over; `None` = the panel's x range (in `(0, 1]`).
    pub h_range: Option<(f64, f64)>,
    pub color: (u8, u8, u8),
    pub width: f64,
    pub linetype: Linetype,
    pub alpha: f64,
    /// Label each curve with its level at its right end.
    pub label: bool,
    pub label_size: f64,
}

impl GeomCooksContour {
    pub fn new(p: usize, levels: &[f64]) -> Self {
        GeomCooksContour {
            p: p as f64,
            levels: levels.to_vec(),
            h_range: None,
            color: (205, 0, 0),
            width: 0.8,
            linetype: Linetype::Dashed,
            alpha: 1.0,
            label: true,
            label_size: 9.0,
        }
    }

    /// Restrict the curves to leverages in `[lo, hi]`.
    pub fn with_h_range(mut self, lo: f64, hi: f64) -> Self {
        self.h_range = Some((lo, hi));
        self
    }
}

impl Default for GeomCooksContour {
    fn default() -> Self {
        GeomCooksContour::new(2, &[0.5, 1.0])
    }
}

impl Geom for GeomCooksContour {
    fn draw(
        &self,
        _data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let (Some(xs), Some(ys)) = (scales.get(&Aesthetic::X), scales.get(&Aesthetic::Y)) else {
            return Ok(());
        };
        if xs.is_discrete() || ys.is_discrete() {
            return Ok(());
        }
        let Some((x0, x1)) = xs.expanded_domain() else {
            return Ok(());
        };
        let (lo, hi) = self.h_range.unwrap_or((x0, x1));
        let lo = lo.max(x0).max(1e-9);
        let hi = hi.min(x1).min(1.0);
        if !(lo.is_finite() && hi.is_finite() && hi > lo) {
            return Ok(());
        }
        let plot_area = backend.plot_area();
        // Log-spaced leverages: the curves are steep near h = 0.
        const N: usize = 240;
        let hs: Vec<f64> = (0..=N)
            .map(|k| lo * (hi / lo).powf(k as f64 / N as f64))
            .collect();
        let style = LineStyle {
            color: self.color,
            alpha: self.alpha,
            width: self.width,
            linetype: self.linetype,
        };
        for &level in self.levels.iter().filter(|l| l.is_finite() && **l > 0.0) {
            let label = super::tip_value(&Value::Float(level));
            super::set_mark(
                backend,
                Some(format!("Cook's distance = {label}")),
                None,
                None,
                super::raw_value(&Value::Float(level)),
            );
            for sign in [1.0, -1.0] {
                let pts: Vec<(f64, f64)> = hs
                    .iter()
                    .map(|&h| {
                        let r = sign * cooks_contour_y(level, self.p, h);
                        (xs.map(&Value::Float(h)), ys.map(&Value::Float(r)))
                    })
                    .collect();
                let runs = clip_polyline_unit(&pts);
                for run in &runs {
                    let px: Vec<(f64, f64)> = run
                        .iter()
                        .map(|&q| coord.transform(q, &plot_area))
                        .collect();
                    backend.draw_line(&px, &style)?;
                }
                // Label at the right end of the rightmost visible run.
                if self.label {
                    if let Some(&end) = runs
                        .iter()
                        .filter_map(|r| r.last())
                        .max_by(|a, b| a.0.total_cmp(&b.0))
                    {
                        let (px, py) = coord.transform(end, &plot_area);
                        let dy = if sign > 0.0 { -6.0 } else { 6.0 };
                        backend.draw_text(
                            &label,
                            (px - 2.0, py + dy),
                            &TextStyle {
                                color: self.color,
                                size: self.label_size,
                                anchor: TextAnchor::End,
                                ..TextStyle::default()
                            },
                        )?;
                    }
                }
            }
        }
        super::clear_mark(backend);
        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![]
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
        "cooks_contour"
    }
    fn inherit_aes(&self) -> bool {
        false
    }
    fn set_series_color(&mut self, _color: (u8, u8, u8)) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contour_inverts_cooks_distance() {
        for &(level, p, h) in &[(0.5, 3.0, 0.1), (1.0, 2.0, 0.4), (4.0, 5.0, 0.9)] {
            let r = cooks_contour_y(level, p, h);
            assert!((cooks_distance(r, h, p) - level).abs() < 1e-12);
        }
        assert!(cooks_contour_y(0.5, 3.0, 0.0).is_nan());
        assert!(cooks_contour_y(0.5, 3.0, 1.5).is_nan());
        assert_eq!(cooks_contour_y(0.5, 3.0, 1.0), 0.0);
    }
}
