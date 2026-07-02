use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Raster geometry — a dense regular grid of filled cells (R's `geom_raster`).
///
/// Reads `x`, `y` (cell centres) and `fill`; the cell size is inferred from the
/// smallest spacing between distinct x/y values. Cells are drawn without a
/// stroke, giving the flat "image" look of a heatmap/raster.
pub struct GeomRaster {
    pub fill: (u8, u8, u8),
    pub alpha: f64,
}

impl Default for GeomRaster {
    fn default() -> Self {
        GeomRaster {
            fill: (97, 156, 255),
            alpha: 1.0,
        }
    }
}

/// Smallest positive gap between distinct, sorted values (fallback 1.0).
fn infer_step(values: &[f64]) -> f64 {
    let mut uniq: Vec<f64> = values.to_vec();
    uniq.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    uniq.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
    let mut step = f64::INFINITY;
    for w in uniq.windows(2) {
        step = step.min(w[1] - w[0]);
    }
    if step.is_finite() && step > 0.0 {
        step
    } else {
        1.0
    }
}

impl Geom for GeomRaster {
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
        let fill_col = data.column("fill");

        let xs: Vec<f64> = x_col.iter().filter_map(|v| v.as_f64()).collect();
        let ys: Vec<f64> = y_col.iter().filter_map(|v| v.as_f64()).collect();
        let half_w = infer_step(&xs) / 2.0;
        let half_h = infer_step(&ys) / 2.0;

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        for i in 0..data.nrows() {
            let cx = x_col[i].as_f64().unwrap_or(0.0);
            let cy = y_col[i].as_f64().unwrap_or(0.0);

            let nxmin = x_scale
                .map(|s| s.map(&Value::Float(cx - half_w)))
                .unwrap_or(0.0);
            let nxmax = x_scale
                .map(|s| s.map(&Value::Float(cx + half_w)))
                .unwrap_or(0.0);
            let nymin = y_scale
                .map(|s| s.map(&Value::Float(cy - half_h)))
                .unwrap_or(0.0);
            let nymax = y_scale
                .map(|s| s.map(&Value::Float(cy + half_h)))
                .unwrap_or(0.0);

            let (left, top) = coord.transform((nxmin, nymax), &plot_area);
            let (right, bottom) = coord.transform((nxmax, nymin), &plot_area);

            let fill_color = fill_col
                .and_then(|fc| scales.map_color(&Aesthetic::Fill, &fc[i]))
                .unwrap_or(self.fill);

            backend.draw_rect(
                (left, top.min(bottom)),
                (right, top.max(bottom)),
                &RectStyle {
                    fill: Some(fill_color),
                    stroke: None,
                    stroke_width: 0.0,
                    alpha: self.alpha,
                    clip: true,
                },
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
        "raster"
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }
}
