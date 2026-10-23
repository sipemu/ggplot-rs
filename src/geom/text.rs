use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, RectStyle, TextAnchor, TextStyle};
use crate::render::text::{wrap_text, wrapped_block_width};
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
    /// Word-wrap width in px (ggtext / `str_wrap` behaviour), measured with
    /// the theme's text metrics ([`crate::render::text::text_width`]).
    /// `None` (the default) never wraps — unchanged from pre-#176 behaviour.
    pub wrap_width: Option<f64>,
    /// Line height as a multiple of `size`, applied between wrapped lines
    /// (CSS-style; ggplot2's `lineheight` is the same idea). Only matters
    /// when `wrap_width` is set and the label wraps to more than one line.
    pub line_height: f64,
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

    /// Word-wrap the label to lines no wider than `width_px` (#176).
    pub fn with_wrap(mut self, width_px: f64) -> Self {
        self.wrap_width = Some(width_px);
        self
    }

    /// Line height as a multiple of `size` for wrapped text (default `1.2`).
    pub fn with_line_height(mut self, line_height: f64) -> Self {
        self.line_height = line_height;
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
            wrap_width: None,
            line_height: 1.2,
        }
    }
}

impl Geom for GeomText {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        theme: &Theme,
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
        let xm = super::support::XMapper::new(data, scales);
        let y_scale = scales.get(&Aesthetic::Y);

        let mut drawn_bboxes: Vec<(f64, f64, f64, f64)> = Vec::new();

        for i in 0..data.nrows() {
            let nx = xm.map(&x_col[i], i);
            let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
            let (px, py) = coord.transform((nx, ny), &plot_area);

            let text = label_col[i].to_group_key();
            let lines = match self.wrap_width {
                Some(w) => wrap_text(&text, w, self.size),
                None => vec![text],
            };
            let lh = self.size * self.line_height;
            let n = lines.len() as f64;
            // The backend centres text vertically (dominant-baseline=middle), so
            // apply vjust as a baseline shift over the whole wrapped block: 0 =
            // bottom-anchored (block above the point), 0.5 = centred, 1 =
            // top-anchored (block below). y grows down. An unwrapped single line
            // (the pre-#176 case, `lines.len() == 1`) keeps the exact pre-#176
            // `(vjust - 0.5) * size` shift — `line_height` only matters once a
            // label actually wraps, so an un-wrapped plot's output is unchanged.
            let block_h = if n <= 1.0 { self.size } else { lh * n };
            let py_block = py + (self.vjust - 0.5) * block_h;

            if self.check_overlap {
                let w = wrapped_block_width(&lines, self.size);
                let bbox = (
                    px - w / 2.0,
                    py_block - block_h / 2.0,
                    px + w / 2.0,
                    py_block + block_h / 2.0,
                );
                if bboxes_overlap(&bbox, &drawn_bboxes) {
                    continue;
                }
                drawn_bboxes.push(bbox);
            }

            let anchor = hjust_to_anchor(self.hjust);
            let family = super::resolve_font_family(&self.fontfamily, theme);
            let top = py_block - block_h / 2.0;
            for (li, line) in lines.iter().enumerate() {
                // n == 1 draws at py_block directly (byte-identical to pre-#176
                // single-line output); n > 1 lays lines out by `lh` within the
                // block computed above.
                let line_y = if n <= 1.0 {
                    py_block
                } else {
                    top + lh * (li as f64 + 0.5)
                };
                backend.draw_text(
                    line,
                    (px, line_y),
                    &TextStyle {
                        color: self.color,
                        size: self.size,
                        anchor: anchor.clone(),
                        angle: 0.0,
                        family: family.clone(),
                        face: crate::render::backend::FontFace::Plain,
                    },
                )?;
            }
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
    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let (true, Some(ink)) = (self.color == (0, 0, 0), el.ink) {
            self.color = ink;
        }
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
    /// Word-wrap width in px (ggtext / `str_wrap` behaviour), measured with
    /// the theme's text metrics ([`crate::render::text::text_width`]). The
    /// background box grows to fit however many lines result, plus
    /// `padding`. `None` (the default) never wraps — unchanged from
    /// pre-#176 behaviour.
    pub wrap_width: Option<f64>,
    /// Line height as a multiple of `size`, applied between wrapped lines.
    /// Only matters when `wrap_width` is set and the label wraps to more
    /// than one line.
    pub line_height: f64,
    /// Corner radius in px for the background box (ggplot2
    /// `geom_label(label.r)`, #176). `0.0` (the default) is a plain
    /// rectangle, unchanged from pre-#176 behaviour.
    pub radius: f64,
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

    /// Word-wrap the label to lines no wider than `width_px`, growing the
    /// background box to fit (#176).
    pub fn with_wrap(mut self, width_px: f64) -> Self {
        self.wrap_width = Some(width_px);
        self
    }

    /// Line height as a multiple of `size` for wrapped text (default `1.2`).
    pub fn with_line_height(mut self, line_height: f64) -> Self {
        self.line_height = line_height;
        self
    }

    /// Corner radius in px for the background box (ggplot2 `label.r`, #176).
    pub fn with_radius(mut self, radius_px: f64) -> Self {
        self.radius = radius_px;
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
            wrap_width: None,
            line_height: 1.2,
            radius: 0.0,
        }
    }
}

impl Geom for GeomLabel {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        theme: &Theme,
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
        let xm = super::support::XMapper::new(data, scales);
        let y_scale = scales.get(&Aesthetic::Y);

        let mut drawn_bboxes: Vec<(f64, f64, f64, f64)> = Vec::new();

        for i in 0..data.nrows() {
            let nx = xm.map(&x_col[i], i);
            let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
            let (px, py) = coord.transform((nx, ny), &plot_area);

            let text = label_col[i].to_group_key();
            // Unwrapped (pre-#176, the default) keeps the exact old box width:
            // a crude char-count estimate, not the real `text_width` metric
            // `with_wrap` uses — changing it would shift every existing
            // geom_label()'s box width with no opt-in. Wrapping is new, so it
            // has no old behaviour to preserve: it sizes the box to fit the
            // wrapped lines using the real metric.
            let (lines, half_w) = match self.wrap_width {
                Some(w) => {
                    let lines = wrap_text(&text, w, self.size);
                    let half_w = wrapped_block_width(&lines, self.size) / 2.0 + self.padding;
                    (lines, half_w)
                }
                None => {
                    let half_w = text.len() as f64 * self.size * 0.6 / 2.0 + self.padding;
                    (vec![text], half_w)
                }
            };
            let lh = self.size * self.line_height;
            let n = lines.len() as f64;
            // See GeomText::draw for why n == 1 keeps the exact pre-#176
            // `size`-based box height and `(vjust - 0.5) * size` shift.
            let block_h = if n <= 1.0 { self.size } else { lh * n };
            let py_block = py + (self.vjust - 0.5) * block_h;
            let half_h = block_h / 2.0 + self.padding;

            if self.check_overlap {
                let bbox = (
                    px - half_w,
                    py_block - half_h,
                    px + half_w,
                    py_block + half_h,
                );
                if bboxes_overlap(&bbox, &drawn_bboxes) {
                    continue;
                }
                drawn_bboxes.push(bbox);
            }

            // Background rect, sized to fit the wrapped lines + padding, with
            // its corners rounded by `radius` (#176's `label.r`).
            backend.draw_rect(
                (px - half_w, py_block - half_h),
                (px + half_w, py_block + half_h),
                &RectStyle {
                    fill: Some(self.fill),
                    stroke: Some(self.color),
                    stroke_width: 0.5,
                    alpha: self.alpha,
                    clip: true,
                    corner_radius: self.radius,
                },
            )?;

            // Text, one draw_text call per wrapped line.
            let anchor = hjust_to_anchor(self.hjust);
            let family = super::resolve_font_family(&self.fontfamily, theme);
            let top = py_block - block_h / 2.0;
            for (li, line) in lines.iter().enumerate() {
                let line_y = if n <= 1.0 {
                    py_block
                } else {
                    top + lh * (li as f64 + 0.5)
                };
                backend.draw_text(
                    line,
                    (px, line_y),
                    &TextStyle {
                        color: self.color,
                        size: self.size,
                        anchor: anchor.clone(),
                        angle: 0.0,
                        family: family.clone(),
                        face: crate::render::backend::FontFace::Plain,
                    },
                )?;
            }
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
    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let (true, Some(ink)) = (self.color == (0, 0, 0), el.ink) {
            self.color = ink;
        }
        if let (true, Some(paper)) = (self.fill == (255, 255, 255), el.paper) {
            self.fill = paper;
        }
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
