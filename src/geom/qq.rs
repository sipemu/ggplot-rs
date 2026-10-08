use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineStyle, Linetype, PointShape, PointStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::qq::{StatQQ, StatQQBand, StatQQLine};
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// QQ plot geometry — draws points at theoretical vs sample quantiles.
pub struct GeomQQ {
    pub size: f64,
    pub color: (u8, u8, u8),
    pub alpha: f64,
}

impl Default for GeomQQ {
    fn default() -> Self {
        GeomQQ {
            size: 3.0,
            color: (0, 0, 0),
            alpha: 1.0,
        }
    }
}

impl Geom for GeomQQ {
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

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        for i in 0..data.nrows() {
            let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
            let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
            let (px, py) = coord.transform((nx, ny), &plot_area);

            let pt_color = color_col
                .and_then(|cc| scales.map_color(&Aesthetic::Color, &cc[i]))
                .unwrap_or(self.color);

            // Hover: "(theoretical, sample)", data-x = theoretical quantile,
            // data-value = sample value.
            let xy = format!(
                "({}, {})",
                super::tip_value(&x_col[i]),
                super::tip_value(&y_col[i])
            );
            let series = super::series_key(data, i);
            let tip = Some(match &series {
                Some(s) => format!("{s}: {xy}"),
                None => xy,
            });
            super::set_mark(
                backend,
                tip,
                Some(super::tip_value(&x_col[i])),
                series,
                super::raw_value(&y_col[i]),
            );

            backend.draw_shape(
                (px, py),
                self.size,
                &PointStyle {
                    color: pt_color,
                    alpha: self.alpha,
                    filled: true,
                    shape: PointShape::Circle,
                },
            )?;
        }
        super::clear_mark(backend);

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Y]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatQQ)
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "qq"
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.color = color;
    }
}

/// QQ line geometry — reference line through Q1/Q3 on the QQ plot.
pub struct GeomQQLine {
    pub color: (u8, u8, u8),
    pub width: f64,
    pub alpha: f64,
}

impl Default for GeomQQLine {
    fn default() -> Self {
        GeomQQLine {
            color: (255, 0, 0),
            width: 1.0,
            alpha: 1.0,
        }
    }
}

impl Geom for GeomQQLine {
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

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        // One line per group (colour / group level).
        for rows in super::support::row_groups(data) {
            let points: Vec<(f64, f64)> = rows
                .iter()
                .map(|&i| {
                    let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
                    let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
                    coord.transform((nx, ny), &plot_area)
                })
                .collect();
            let first = rows[0];
            let color = data
                .column("color")
                .and_then(|c| scales.map_color(&Aesthetic::Color, &c[first]))
                .unwrap_or(self.color);
            let series = super::series_key(data, first);
            super::set_mark(backend, series.clone(), None, series, None);
            if points.len() >= 2 {
                backend.draw_line(
                    &points,
                    &LineStyle {
                        color,
                        alpha: self.alpha,
                        width: self.width,
                        linetype: Linetype::Dashed,
                    },
                )?;
            }
        }
        super::clear_mark(backend);

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Y]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatQQLine)
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "qq_line"
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.color = color;
    }
}

/// QQ confidence band (qqplotr's `geom_qq_band`): a filled envelope between
/// `ymin` and `ymax` along the theoretical quantiles, one polygon per group.
/// Default stat [`StatQQBand`] (pointwise normal band at 95%). `±Inf`
/// bounds (a KS band hitting probability 0 or 1) extend to the panel edge.
pub struct GeomQQBand {
    pub fill: (u8, u8, u8),
    pub alpha: f64,
}

impl Default for GeomQQBand {
    fn default() -> Self {
        // qqplotr: rgb(0.6, 0.6, 0.6, 0.5)
        GeomQQBand {
            fill: (153, 153, 153),
            alpha: 0.5,
        }
    }
}

impl Geom for GeomQQBand {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        super::ribbon::draw_bands(
            data,
            coord,
            scales,
            backend,
            self.fill,
            self.alpha,
            super::ribbon::BandShape::Linear,
        )
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Y]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatQQBand::default())
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "qq_band"
    }

    fn allows_infinite(&self) -> bool {
        true
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }
}
