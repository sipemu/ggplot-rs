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
    /// Horizontal justification: 0.0 = left, 0.5 = center (default), 1.0 = right.
    pub hjust: f64,
    /// Vertical justification: 0.0 = bottom, 0.5 = middle (default), 1.0 = top.
    pub vjust: f64,
    /// Font family name (informational; actual rendering depends on backend).
    pub fontfamily: String,
    /// When true, skip drawing labels that overlap previously drawn labels.
    pub check_overlap: bool,
}

impl GeomText {
    pub fn with_hjust(mut self, hjust: f64) -> Self {
        self.hjust = hjust;
        self
    }

    pub fn with_vjust(mut self, vjust: f64) -> Self {
        self.vjust = vjust;
        self
    }

    pub fn with_fontfamily(mut self, family: &str) -> Self {
        self.fontfamily = family.to_string();
        self
    }

    pub fn with_check_overlap(mut self, check: bool) -> Self {
        self.check_overlap = check;
        self
    }
}

impl Default for GeomText {
    fn default() -> Self {
        GeomText {
            size: 10.0,
            color: (0, 0, 0),
            alpha: 1.0,
            hjust: 0.5,
            vjust: 0.5,
            fontfamily: String::new(),
            check_overlap: false,
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

        let mut drawn_bboxes: Vec<(f64, f64, f64, f64)> = Vec::new();

        for i in 0..data.nrows() {
            let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
            let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
            let (px, py) = coord.transform((nx, ny), &plot_area);

            let text = label_col[i].to_group_key();

            if self.check_overlap {
                let w = text.len() as f64 * self.size * 0.6;
                let h = self.size;
                let bbox = (px - w / 2.0, py - h / 2.0, px + w / 2.0, py + h / 2.0);
                if bboxes_overlap(&bbox, &drawn_bboxes) {
                    continue;
                }
                drawn_bboxes.push(bbox);
            }

            let anchor = hjust_to_anchor(self.hjust);
            backend.draw_text(
                &text,
                (px, py),
                &TextStyle {
                    color: self.color,
                    size: self.size,
                    anchor,
                    angle: 0.0,
                    family: None,
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
    /// Horizontal justification: 0.0 = left, 0.5 = center (default), 1.0 = right.
    pub hjust: f64,
    /// Vertical justification: 0.0 = bottom, 0.5 = middle (default), 1.0 = top.
    pub vjust: f64,
    /// Font family name (informational; actual rendering depends on backend).
    pub fontfamily: String,
    /// When true, skip drawing labels that overlap previously drawn labels.
    pub check_overlap: bool,
}

impl GeomLabel {
    pub fn with_hjust(mut self, hjust: f64) -> Self {
        self.hjust = hjust;
        self
    }

    pub fn with_vjust(mut self, vjust: f64) -> Self {
        self.vjust = vjust;
        self
    }

    pub fn with_fontfamily(mut self, family: &str) -> Self {
        self.fontfamily = family.to_string();
        self
    }

    pub fn with_check_overlap(mut self, check: bool) -> Self {
        self.check_overlap = check;
        self
    }
}

impl Default for GeomLabel {
    fn default() -> Self {
        GeomLabel {
            size: 10.0,
            color: (0, 0, 0),
            fill: (255, 255, 255),
            alpha: 0.8,
            padding: 3.0,
            hjust: 0.5,
            vjust: 0.5,
            fontfamily: String::new(),
            check_overlap: false,
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

        let mut drawn_bboxes: Vec<(f64, f64, f64, f64)> = Vec::new();

        for i in 0..data.nrows() {
            let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
            let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
            let (px, py) = coord.transform((nx, ny), &plot_area);

            let text = label_col[i].to_group_key();
            let approx_width = text.len() as f64 * self.size * 0.6;
            let half_w = approx_width / 2.0 + self.padding;
            let half_h = self.size / 2.0 + self.padding;

            if self.check_overlap {
                let bbox = (px - half_w, py - half_h, px + half_w, py + half_h);
                if bboxes_overlap(&bbox, &drawn_bboxes) {
                    continue;
                }
                drawn_bboxes.push(bbox);
            }

            // Background rect
            backend.draw_rect(
                (px - half_w, py - half_h),
                (px + half_w, py + half_h),
                &RectStyle {
                    fill: Some(self.fill),
                    stroke: Some(self.color),
                    stroke_width: 0.5,
                    alpha: self.alpha,
                    clip: true,
                },
            )?;

            // Text
            let anchor = hjust_to_anchor(self.hjust);
            backend.draw_text(
                &text,
                (px, py),
                &TextStyle {
                    color: self.color,
                    size: self.size,
                    anchor,
                    angle: 0.0,
                    family: None,
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

/// Map hjust (0.0 = left, 0.5 = center, 1.0 = right) to TextAnchor.
fn hjust_to_anchor(hjust: f64) -> TextAnchor {
    if hjust < 0.25 {
        TextAnchor::Start
    } else if hjust > 0.75 {
        TextAnchor::End
    } else {
        TextAnchor::Middle
    }
}

/// Check if a bbox overlaps any existing bbox.
fn bboxes_overlap(candidate: &(f64, f64, f64, f64), existing: &[(f64, f64, f64, f64)]) -> bool {
    for b in existing {
        // Two rects overlap if they overlap on both axes
        if candidate.0 < b.2 && candidate.2 > b.0 && candidate.1 < b.3 && candidate.3 > b.1 {
            return true;
        }
    }
    false
}
