use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineStyle, Linetype, PointShape, PointStyle, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::boxplot::StatBoxplot;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Boxplot geometry.
pub struct GeomBoxplot {
    pub fill: (u8, u8, u8),
    pub color: (u8, u8, u8),
    pub width: f64,
    pub alpha: f64,
}

impl Default for GeomBoxplot {
    fn default() -> Self {
        GeomBoxplot {
            fill: (255, 255, 255),
            color: (50, 50, 50),
            width: 0.75,
            alpha: 1.0,
        }
    }
}

impl Geom for GeomBoxplot {
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
        let lower_col = data.column("lower");
        let middle_col = data.column("middle");
        let upper_col = data.column("upper");
        let ymin_col = data.column("ymin");
        let ymax_col = data.column("ymax");
        let outliers_col = data.column("outliers");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        let x_is_discrete = x_scale.map(|s| s.is_discrete()).unwrap_or(false);

        for i in 0..data.nrows() {
            let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.5);

            let half_width = if x_is_discrete {
                let n = x_scale.map(|s| s.breaks().len()).unwrap_or(1);
                self.width / (n.max(1) as f64 * 2.5)
            } else {
                0.03
            };

            // Get boxplot values
            let lower = lower_col.and_then(|c| c[i].as_f64()).unwrap_or(0.0);
            let middle = middle_col.and_then(|c| c[i].as_f64()).unwrap_or(0.0);
            let upper = upper_col.and_then(|c| c[i].as_f64()).unwrap_or(0.0);
            let ymin = ymin_col.and_then(|c| c[i].as_f64()).unwrap_or(lower);
            let ymax = ymax_col.and_then(|c| c[i].as_f64()).unwrap_or(upper);

            let map_y = |v: f64| y_scale.map(|s| s.map(&Value::Float(v))).unwrap_or(0.0);

            // Box (IQR)
            let (box_left, box_top) = coord.transform((nx - half_width, map_y(upper)), &plot_area);
            let (box_right, box_bottom) =
                coord.transform((nx + half_width, map_y(lower)), &plot_area);
            backend.draw_rect(
                (box_left, box_top),
                (box_right, box_bottom),
                &RectStyle {
                    fill: Some(self.fill),
                    stroke: Some(self.color),
                    stroke_width: 1.0,
                    alpha: self.alpha,
                    clip: true,
                },
            )?;

            // Median line
            let (med_left, med_y) = coord.transform((nx - half_width, map_y(middle)), &plot_area);
            let (med_right, _) = coord.transform((nx + half_width, map_y(middle)), &plot_area);
            backend.draw_line(
                &[(med_left, med_y), (med_right, med_y)],
                &LineStyle {
                    color: self.color,
                    width: 2.0,
                    alpha: 1.0,
                    linetype: Linetype::Solid,
                },
            )?;

            // Lower whisker
            let (_, whisker_bottom) = coord.transform((nx, map_y(ymin)), &plot_area);
            let (center_x, _) = coord.transform((nx, map_y(lower)), &plot_area);
            backend.draw_line(
                &[(center_x, box_bottom), (center_x, whisker_bottom)],
                &LineStyle {
                    color: self.color,
                    width: 1.0,
                    alpha: 1.0,
                    linetype: Linetype::Solid,
                },
            )?;
            // Lower whisker cap
            let (wl, _) = coord.transform((nx - half_width * 0.5, map_y(ymin)), &plot_area);
            let (wr, _) = coord.transform((nx + half_width * 0.5, map_y(ymin)), &plot_area);
            backend.draw_line(
                &[(wl, whisker_bottom), (wr, whisker_bottom)],
                &LineStyle {
                    color: self.color,
                    width: 1.0,
                    alpha: 1.0,
                    linetype: Linetype::Solid,
                },
            )?;

            // Upper whisker
            let (_, whisker_top) = coord.transform((nx, map_y(ymax)), &plot_area);
            backend.draw_line(
                &[(center_x, box_top), (center_x, whisker_top)],
                &LineStyle {
                    color: self.color,
                    width: 1.0,
                    alpha: 1.0,
                    linetype: Linetype::Solid,
                },
            )?;
            // Upper whisker cap
            let (wl, _) = coord.transform((nx - half_width * 0.5, map_y(ymax)), &plot_area);
            let (wr, _) = coord.transform((nx + half_width * 0.5, map_y(ymax)), &plot_area);
            backend.draw_line(
                &[(wl, whisker_top), (wr, whisker_top)],
                &LineStyle {
                    color: self.color,
                    width: 1.0,
                    alpha: 1.0,
                    linetype: Linetype::Solid,
                },
            )?;

            // Outliers
            if let Some(oc) = outliers_col {
                if let Value::Str(s) = &oc[i] {
                    for part in s.split(',') {
                        if let Ok(val) = part.trim().parse::<f64>() {
                            let ny = map_y(val);
                            let (ox, oy) = coord.transform((nx, ny), &plot_area);
                            backend.draw_circle(
                                (ox, oy),
                                2.0,
                                &PointStyle {
                                    color: self.color,
                                    alpha: 1.0,
                                    filled: false,
                                    shape: PointShape::Circle,
                                },
                            )?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatBoxplot)
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "boxplot"
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }
}
