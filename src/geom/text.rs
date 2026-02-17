use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, RectStyle, TextAnchor, TextStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Text geometry — draws text labels at data positions.
pub struct GeomText {
    pub size: f64,
    pub color: (u8, u8, u8),
    pub alpha: f64,
}

impl Default for GeomText {
    fn default() -> Self {
        GeomText {
            size: 10.0,
            color: (0, 0, 0),
            alpha: 1.0,
        }
    }
}

impl Geom for GeomText {
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
        let label_col = data
            .column("label")
            .ok_or(RenderError::MissingAesthetic("label".into()))?;

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        for i in 0..data.nrows() {
            let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
            let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
            let (px, py) = coord.transform((nx, ny), &plot_area);

            let text = label_col[i].to_group_key();

            backend.draw_text(
                &text,
                (px, py),
                &TextStyle {
                    color: self.color,
                    size: self.size,
                    anchor: TextAnchor::Middle,
                    angle: 0.0,
                },
            )?;
        }

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y, Aesthetic::Label]
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
        "text"
    }
}

/// Label geometry — like text but with a background rectangle.
pub struct GeomLabel {
    pub size: f64,
    pub color: (u8, u8, u8),
    pub fill: (u8, u8, u8),
    pub alpha: f64,
    pub padding: f64,
}

impl Default for GeomLabel {
    fn default() -> Self {
        GeomLabel {
            size: 10.0,
            color: (0, 0, 0),
            fill: (255, 255, 255),
            alpha: 0.8,
            padding: 3.0,
        }
    }
}

impl Geom for GeomLabel {
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
        let label_col = data
            .column("label")
            .ok_or(RenderError::MissingAesthetic("label".into()))?;

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        for i in 0..data.nrows() {
            let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
            let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
            let (px, py) = coord.transform((nx, ny), &plot_area);

            let text = label_col[i].to_group_key();
            let approx_width = text.len() as f64 * self.size * 0.6;
            let half_w = approx_width / 2.0 + self.padding;
            let half_h = self.size / 2.0 + self.padding;

            // Background rect
            backend.draw_rect(
                (px - half_w, py - half_h),
                (px + half_w, py + half_h),
                &RectStyle {
                    fill: Some(self.fill),
                    stroke: Some(self.color),
                    stroke_width: 0.5,
                    alpha: self.alpha,
                },
            )?;

            // Text
            backend.draw_text(
                &text,
                (px, py),
                &TextStyle {
                    color: self.color,
                    size: self.size,
                    anchor: TextAnchor::Middle,
                    angle: 0.0,
                },
            )?;
        }

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X, Aesthetic::Y, Aesthetic::Label]
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
        "label"
    }
}
