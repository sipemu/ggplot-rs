use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::step::StepDirection;
use super::{Geom, GeomParams};

/// How a band's edges connect consecutive `(x, ymin, ymax)` rows.
pub(crate) enum BandShape<'a> {
    /// Straight segments (`geom_ribbon`).
    Linear,
    /// A step function (`geom_stepribbon`), like `geom_step`'s direction.
    Step(&'a StepDirection),
}

/// Insert step corners into a run of normalized `(x, y)` points.
pub(crate) fn stepped(raw: &[(f64, f64)], direction: &StepDirection) -> Vec<(f64, f64)> {
    let mut out = Vec::with_capacity(raw.len() * 2);
    for (j, &(x, y)) in raw.iter().enumerate() {
        if j > 0 {
            let (px, py) = raw[j - 1];
            match direction {
                StepDirection::Hv => out.push((x, py)),
                StepDirection::Vh => out.push((px, y)),
                StepDirection::Mid => {
                    let m = 0.5 * (px + x);
                    out.push((m, py));
                    out.push((m, y));
                }
            }
        }
        out.push((x, y));
    }
    out
}

/// Draw one filled band per group (group / colour / fill level) between
/// `ymin` and `ymax`, rows ordered by x. Fill comes from a mapped `fill`,
/// else `fill`. Shared by `geom_ribbon`, `geom_stepribbon`, `geom_qq_band`.
pub(crate) fn draw_bands(
    data: &DataFrame,
    coord: &dyn Coord,
    scales: &ScaleSet,
    backend: &mut dyn DrawBackend,
    fill: (u8, u8, u8),
    alpha: f64,
    shape: BandShape<'_>,
) -> Result<(), RenderError> {
    let x_col = data
        .column("x")
        .ok_or(RenderError::MissingAesthetic("x".into()))?;
    let ymin_col = data
        .column("ymin")
        .ok_or(RenderError::MissingAesthetic("ymin".into()))?;
    let ymax_col = data
        .column("ymax")
        .ok_or(RenderError::MissingAesthetic("ymax".into()))?;
    let plot_area = backend.plot_area();
    let y_scale = scales.get(&Aesthetic::Y);
    let xm = super::support::XMapper::new(data, scales);

    for rows in super::support::row_groups(data) {
        let mut pts: Vec<(f64, f64, f64)> = rows
            .iter()
            .map(|&i| {
                (
                    xm.map(&x_col[i], i),
                    y_scale.map(|s| s.map(&ymin_col[i])).unwrap_or(0.0),
                    y_scale.map(|s| s.map(&ymax_col[i])).unwrap_or(0.0),
                )
            })
            .filter(|(x, a, b)| x.is_finite() && a.is_finite() && b.is_finite())
            .collect();
        if pts.len() < 2 {
            continue;
        }
        pts.sort_by(|a, b| a.0.total_cmp(&b.0));
        let upper: Vec<(f64, f64)> = pts.iter().map(|p| (p.0, p.2)).collect();
        let lower: Vec<(f64, f64)> = pts.iter().map(|p| (p.0, p.1)).collect();
        let (upper, mut lower) = match shape {
            BandShape::Linear => (upper, lower),
            BandShape::Step(dir) => (stepped(&upper, dir), stepped(&lower, dir)),
        };
        lower.reverse();
        let polygon: Vec<(f64, f64)> = upper
            .into_iter()
            .chain(lower)
            .map(|p| coord.transform(p, &plot_area))
            .collect();

        let first = rows[0];
        let band_fill = data
            .column("fill")
            .and_then(|c| scales.map_color(&Aesthetic::Fill, &c[first]))
            .unwrap_or(fill);
        let series = super::series_key(data, first);
        super::set_mark(backend, series.clone(), None, series, None);
        backend.draw_polygon(
            &polygon,
            &RectStyle {
                fill: Some(band_fill),
                stroke: None,
                stroke_width: 0.0,
                alpha,
                clip: true,
            },
        )?;
    }
    super::clear_mark(backend);
    Ok(())
}

/// Ribbon geometry — filled band between ymin and ymax (one per group).
pub struct GeomRibbon {
    pub fill: (u8, u8, u8),
    pub alpha: f64,
}

impl Default for GeomRibbon {
    fn default() -> Self {
        GeomRibbon {
            fill: (97, 156, 255),
            alpha: 0.3,
        }
    }
}

impl Geom for GeomRibbon {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        draw_bands(
            data,
            coord,
            scales,
            backend,
            self.fill,
            self.alpha,
            BandShape::Linear,
        )
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Ymin, Aesthetic::Ymax]
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
        "ribbon"
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }
}

/// Step ribbon (pammtools' `geom_stepribbon`): a band whose edges are step
/// functions — the confidence band of a Kaplan–Meier curve or an ECDF.
/// `direction` matches `geom_step` (`Hv` default: the value holds until the
/// next x). `±Inf` x (e.g. `stat_ecdf`'s padding) extends to the panel edge.
pub struct GeomStepribbon {
    pub fill: (u8, u8, u8),
    pub alpha: f64,
    pub direction: StepDirection,
}

impl Default for GeomStepribbon {
    fn default() -> Self {
        GeomStepribbon {
            fill: (97, 156, 255),
            alpha: 0.3,
            direction: StepDirection::Hv,
        }
    }
}

impl GeomStepribbon {
    /// A step ribbon with the given direction.
    pub fn new(direction: StepDirection) -> Self {
        GeomStepribbon {
            direction,
            ..Default::default()
        }
    }
}

impl Geom for GeomStepribbon {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        draw_bands(
            data,
            coord,
            scales,
            backend,
            self.fill,
            self.alpha,
            BandShape::Step(&self.direction),
        )
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Ymin, Aesthetic::Ymax]
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
        "stepribbon"
    }

    fn allows_infinite(&self) -> bool {
        true
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_corners() {
        let raw = [(0.0, 1.0), (1.0, 2.0), (2.0, 3.0)];
        assert_eq!(
            stepped(&raw, &StepDirection::Hv),
            vec![(0.0, 1.0), (1.0, 1.0), (1.0, 2.0), (2.0, 2.0), (2.0, 3.0)]
        );
        assert_eq!(
            stepped(&raw, &StepDirection::Vh),
            vec![(0.0, 1.0), (0.0, 2.0), (1.0, 2.0), (1.0, 3.0), (2.0, 3.0)]
        );
        assert_eq!(
            stepped(&raw[..2], &StepDirection::Mid),
            vec![(0.0, 1.0), (0.5, 1.0), (0.5, 2.0), (1.0, 2.0)]
        );
    }
}
