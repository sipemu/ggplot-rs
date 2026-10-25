use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, FontFace, RectStyle, TextAnchor, TextStyle};
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
    /// Font face (#228, ggplot2's `fontface`): bold/italic, same semantics
    /// and enum as [`crate::geom::repel::GeomTextRepel::fontface`]. A row's
    /// mapped `Aes::fontface` (if valid) wins over this fixed setting.
    pub fontface: FontFace,
    /// Text halo colour (#229, ggrepel's `bg.colour`/`bg.r`; `shadowtext`'s
    /// `bg.col`/`bg.r`): an outline in this colour behind the text, so a
    /// label placed directly on a line or a busy background stays legible
    /// without a box. `None` (the default) draws no halo — set together
    /// with [`bg_r`](Self::bg_r), which is `0.0` (no halo) until set.
    pub bg_colour: Option<(u8, u8, u8)>,
    /// Text halo radius in px (#229, ggrepel's `bg.r`). Only matters once
    /// [`bg_colour`](Self::bg_colour) is set.
    pub bg_r: f64,
}

impl GeomText {
    /// Fixed font face (#228); a mapped `Aes::fontface` wins per row.
    pub fn with_fontface(mut self, face: FontFace) -> Self {
        self.fontface = face;
        self
    }

    /// Text halo colour (#229, ggrepel's `bg.colour`) — set
    /// [`bg_r`](Self::bg_r) too, or the halo has zero width and draws
    /// nothing.
    pub fn bg_colour(mut self, colour: (u8, u8, u8)) -> Self {
        self.bg_colour = Some(colour);
        self
    }

    /// Text halo radius in px (#229, ggrepel's `bg.r`).
    pub fn bg_r(mut self, r: f64) -> Self {
        self.bg_r = r;
        self
    }

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
            fontface: FontFace::Plain,
            bg_colour: None,
            bg_r: 0.0,
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
        let fontface_col = data.column("fontface");

        let plot_area = backend.plot_area();
        let xm = super::support::XMapper::new(data, scales);
        let y_scale = scales.get(&Aesthetic::Y);

        let mut drawn_bboxes: Vec<(f64, f64, f64, f64)> = Vec::new();

        for i in 0..data.nrows() {
            // #221: `Aes::class`/`group_wrap`/`data_attr` per row.
            super::set_row_extras(backend, data, i);
            // Text geoms don't consume tooltips (the build warns if one is
            // mapped) — keep the label box `<title>`-free as before.
            backend.set_tooltip(None);
            let nx = xm.map(&x_col[i], i);
            let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
            let (px, py) = coord.transform((nx, ny), &plot_area);

            // #228: a mapped `Aes::fontface` wins over the fixed setting.
            let face = fontface_col
                .map(|c| &c[i])
                .filter(|v| !v.is_na())
                .and_then(|v| super::parse_fontface(&v.to_group_key()))
                .unwrap_or(self.fontface);

            let text = label_col[i].to_group_key();
            // #249: an embedded `"\n"` is a forced line break, the same
            // `geom_label` behaviour since #229 — split on it *before* any
            // width-based wrapping, so `"a\nb"` always draws two lines
            // instead of one `<text>` with a literal raw newline inside it
            // (SVG has no visual effect for that; most renderers just
            // collapse it to whitespace). `str::lines()` handles "\r\n"
            // too; an empty label keeps the single empty line the old code
            // already drew.
            let paragraphs: Vec<String> = if text.is_empty() {
                vec![text]
            } else {
                text.lines().map(str::to_string).collect()
            };
            let lines = match self.wrap_width {
                Some(w) => paragraphs
                    .iter()
                    .flat_map(|p| wrap_text(p, w, self.size))
                    .collect::<Vec<_>>(),
                None => paragraphs,
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
                        face,
                        bg_colour: self.bg_colour,
                        bg_r: self.bg_r,
                    },
                )?;
            }
        }
        super::clear_row_extras(backend);

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
    /// Font face (#228, ggplot2's `fontface`): bold/italic, same semantics
    /// and enum as [`crate::geom::repel::GeomLabelRepel::fontface`]. A
    /// row's mapped `Aes::fontface` (if valid) wins over this fixed
    /// setting.
    pub fontface: FontFace,
}

impl GeomLabel {
    /// Fixed font face (#228); a mapped `Aes::fontface` wins per row.
    pub fn with_fontface(mut self, face: FontFace) -> Self {
        self.fontface = face;
        self
    }
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
            fontface: FontFace::Plain,
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
        let fontface_col = data.column("fontface");

        let plot_area = backend.plot_area();
        let xm = super::support::XMapper::new(data, scales);
        let y_scale = scales.get(&Aesthetic::Y);

        let mut drawn_bboxes: Vec<(f64, f64, f64, f64)> = Vec::new();

        for i in 0..data.nrows() {
            // #221: `Aes::class`/`group_wrap`/`data_attr` per row.
            super::set_row_extras(backend, data, i);
            // Text geoms don't consume tooltips (the build warns if one is
            // mapped) — keep the label box `<title>`-free as before.
            backend.set_tooltip(None);
            let nx = xm.map(&x_col[i], i);
            let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
            let (px, py) = coord.transform((nx, ny), &plot_area);

            // #228: a mapped `Aes::fontface` wins over the fixed setting.
            let face = fontface_col
                .map(|c| &c[i])
                .filter(|v| !v.is_na())
                .and_then(|v| super::parse_fontface(&v.to_group_key()))
                .unwrap_or(self.fontface);

            let text = label_col[i].to_group_key();
            // #229: an embedded `"\n"` is a forced line break (ggplot2's
            // `geom_label` behaviour) — split on it *before* any
            // width-based wrapping, so `"a\nb"` always draws two lines
            // inside one box, never one box with a literal raw newline
            // sitting unbroken inside the `<text>` (SVG has no visual
            // effect for that; most renderers just collapse it to
            // whitespace). `str::lines()` handles "\r\n" too; an empty
            // label keeps the single empty line the old code already drew.
            let text_len = text.len();
            let paragraphs: Vec<String> = if text.is_empty() {
                vec![text]
            } else {
                text.lines().map(str::to_string).collect()
            };
            let (lines, half_w) = match self.wrap_width {
                Some(w) => {
                    let lines = paragraphs
                        .iter()
                        .flat_map(|p| wrap_text(p, w, self.size))
                        .collect::<Vec<_>>();
                    let half_w = wrapped_block_width(&lines, self.size) / 2.0 + self.padding;
                    (lines, half_w)
                }
                // No `wrap_width` and no forced break (the overwhelming
                // common case): keep the exact pre-#176 box width, a
                // crude char-count estimate, not the real `text_width`
                // metric `with_wrap` uses below — changing it would shift
                // every existing single-line `geom_label()`'s box width
                // with no opt-in.
                None if paragraphs.len() <= 1 => {
                    let half_w = text_len as f64 * self.size * 0.6 / 2.0 + self.padding;
                    (paragraphs, half_w)
                }
                // A forced multi-line label with no explicit wrap: there's
                // no old behaviour to preserve (a `"\n"` here used to
                // produce one wrongly-sized box with unbroken text, a
                // bug — see above), so size the box to fit every line
                // with the real metric, like the `Some(w)` branch.
                None => {
                    let half_w = wrapped_block_width(&paragraphs, self.size) / 2.0 + self.padding;
                    (paragraphs, half_w)
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
                        face,
                        bg_colour: None,
                        bg_r: 0.0,
                    },
                )?;
            }
        }
        super::clear_row_extras(backend);

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
