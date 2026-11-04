//! Vector PDF backend (feature `pdf`, #467) on top of [krilla]: lines,
//! polygons (even-odd holes), points, rounded rects, clip groups, raster map
//! tiles and real, selectable text in the bundled DejaVu fonts (embedded as
//! subsets). One canvas px is one PDF point.
//!
//! krilla's page surface borrows its document, so drawing is recorded as a
//! display list of krilla paths / text runs and replayed into a fresh
//! document by [`PdfBackend::finish`].

use ab_glyph::{Font as _, FontRef, ScaleFont as _};
use krilla::color::rgb;
use krilla::geom::{Path, PathBuilder, Point, Size, Transform};
use krilla::image::Image;
use krilla::num::NormalizedF32;
use krilla::page::PageSettings;
use krilla::paint::{Fill, FillRule, LineCap, LineJoin, Stroke, StrokeDash};
use krilla::surface::Surface;
use krilla::text::{Font, TextDirection};
use krilla::Document;

use super::backend::{
    DrawBackend, FontFace, LineEnd, LineStyle, PointShape, PointStyle, RectStyle, TextAnchor,
    TextStyle,
};
use super::{Rect, RenderError};

const SANS: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans.ttf");
const SANS_BOLD: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans-Bold.ttf");
const SANS_OBLIQUE: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans-Oblique.ttf");
const SERIF: &[u8] = include_bytes!("../../assets/fonts/DejaVuSerif.ttf");
const SERIF_BOLD: &[u8] = include_bytes!("../../assets/fonts/DejaVuSerif-Bold.ttf");
const SERIF_ITALIC: &[u8] = include_bytes!("../../assets/fonts/DejaVuSerif-Italic.ttf");
const MONO: &[u8] = include_bytes!("../../assets/fonts/DejaVuSansMono.ttf");

/// The bundled font file for a family / face (sans, serif or mono).
fn font_bytes(family: Option<&str>, face: FontFace) -> &'static [u8] {
    let fam = family.unwrap_or("sans").to_ascii_lowercase();
    if fam.contains("mono") || fam.contains("courier") {
        MONO
    } else if fam.contains("serif") && !fam.contains("sans") {
        match face {
            FontFace::Bold => SERIF_BOLD,
            FontFace::Italic => SERIF_ITALIC,
            _ => SERIF,
        }
    } else {
        match face {
            FontFace::Bold => SANS_BOLD,
            FontFace::Italic => SANS_OBLIQUE,
            _ => SANS,
        }
    }
}

/// One recorded drawing operation.
enum Op {
    Path {
        path: Path,
        fill: Option<Fill>,
        stroke: Option<Stroke>,
    },
    Text {
        start: Point,
        font: Font,
        size: f32,
        text: String,
        fill: Fill,
        halo: Option<Stroke>,
        transform: Option<Transform>,
    },
    Image {
        image: Image,
        rect: Rect,
        clip: Path,
        alpha: f32,
    },
    PushClip(Path),
    Pop,
}

/// A [`DrawBackend`] that writes a single-page vector PDF.
pub struct PdfBackend {
    width: u32,
    height: u32,
    plot_area: Rect,
    total_area: Rect,
    ops: Vec<Op>,
    /// Open clip groups (`begin_clip` / `begin_panel`), closed by `end_*`.
    open_clips: usize,
    /// One krilla `Font` per bundled file, so each is embedded once.
    fonts: Vec<(&'static [u8], Font)>,
    corner_claims: Vec<Rect>,
}

fn color(c: (u8, u8, u8)) -> krilla::paint::Paint {
    rgb::Color::new(c.0, c.1, c.2).into()
}

fn opacity(a: f64) -> NormalizedF32 {
    NormalizedF32::new(if a.is_finite() {
        a.clamp(0.0, 1.0)
    } else {
        1.0
    } as f32)
    .unwrap_or(NormalizedF32::ONE)
}

fn fill(c: (u8, u8, u8), alpha: f64, rule: FillRule) -> Fill {
    Fill {
        paint: color(c),
        opacity: opacity(alpha),
        rule,
    }
}

fn stroke(c: (u8, u8, u8), width: f64, alpha: f64) -> Stroke {
    Stroke {
        paint: color(c),
        width: width.max(0.0) as f32,
        opacity: opacity(alpha),
        ..Default::default()
    }
}

fn finite(p: (f64, f64)) -> bool {
    p.0.is_finite() && p.1.is_finite()
}

/// A path through `pts` (non-finite points dropped), closed when asked.
fn poly_path(pts: &[(f64, f64)], close: bool) -> Option<Path> {
    let mut pb = PathBuilder::new();
    let mut n = 0;
    for &(x, y) in pts.iter().filter(|p| finite(**p)) {
        if n == 0 {
            pb.move_to(x as f32, y as f32);
        } else {
            pb.line_to(x as f32, y as f32);
        }
        n += 1;
    }
    if n < 2 {
        return None;
    }
    if close {
        pb.close();
    }
    pb.finish()
}

fn rect_path(r: &Rect) -> Option<Path> {
    poly_path(
        &[
            (r.x, r.y),
            (r.x + r.width, r.y),
            (r.x + r.width, r.y + r.height),
            (r.x, r.y + r.height),
        ],
        true,
    )
}

impl PdfBackend {
    /// A `w` × `h` px page (1 px = 1 pt) whose panel is `plot_area`.
    pub fn new(w: u32, h: u32, plot_area: Rect) -> Self {
        PdfBackend {
            width: w,
            height: h,
            plot_area,
            total_area: Rect {
                x: 0.0,
                y: 0.0,
                width: w as f64,
                height: h as f64,
            },
            ops: Vec::new(),
            open_clips: 0,
            fonts: Vec::new(),
            corner_claims: Vec::new(),
        }
    }

    fn font(&mut self, bytes: &'static [u8]) -> Option<Font> {
        if let Some((_, f)) = self.fonts.iter().find(|(b, _)| std::ptr::eq(*b, bytes)) {
            return Some(f.clone());
        }
        let f = Font::new(bytes.into(), 0)?;
        self.fonts.push((bytes, f.clone()));
        Some(f)
    }

    fn push_path(&mut self, path: Option<Path>, fill: Option<Fill>, stroke: Option<Stroke>) {
        if let Some(path) = path {
            if fill.is_some() || stroke.is_some() {
                self.ops.push(Op::Path { path, fill, stroke });
            }
        }
    }

    /// Write the PDF.
    pub fn finish(mut self) -> Result<Vec<u8>, RenderError> {
        for _ in 0..self.open_clips {
            self.ops.push(Op::Pop);
        }
        let mut document = Document::new();
        let settings = PageSettings::from_wh(self.width.max(1) as f32, self.height.max(1) as f32)
            .ok_or_else(|| RenderError::BackendError("pdf: invalid page size".into()))?;
        let mut page = document.start_page_with(settings);
        let mut surface = page.surface();
        for op in self.ops {
            replay(&mut surface, op);
        }
        surface.finish();
        page.finish();
        document
            .finish()
            .map_err(|e| RenderError::BackendError(format!("pdf: {e:?}")))
    }
}

fn replay(surface: &mut Surface<'_>, op: Op) {
    match op {
        Op::Path { path, fill, stroke } => {
            surface.set_fill(fill);
            surface.set_stroke(stroke);
            surface.draw_path(&path);
        }
        Op::Text {
            start,
            font,
            size,
            text,
            fill,
            halo,
            transform,
        } => {
            if let Some(t) = transform {
                surface.push_transform(&t);
            }
            if let Some(h) = halo {
                surface.set_fill(None);
                surface.set_stroke(Some(h));
                surface.draw_text(start, font.clone(), size, &text, true, TextDirection::Auto);
            }
            surface.set_stroke(None);
            surface.set_fill(Some(fill));
            surface.draw_text(start, font, size, &text, false, TextDirection::Auto);
            if transform.is_some() {
                surface.pop();
            }
        }
        Op::Image {
            image,
            rect,
            clip,
            alpha,
        } => {
            surface.push_clip_path(&clip, &FillRule::NonZero);
            surface.push_opacity(opacity(alpha as f64));
            surface.push_transform(&Transform::from_translate(rect.x as f32, rect.y as f32));
            if let Some(size) = Size::from_wh(rect.width as f32, rect.height as f32) {
                surface.draw_image(image, size);
            }
            surface.pop();
            surface.pop();
            surface.pop();
        }
        Op::PushClip(path) => surface.push_clip_path(&path, &FillRule::NonZero),
        Op::Pop => surface.pop(),
    }
}

impl DrawBackend for PdfBackend {
    fn draw_circle(
        &mut self,
        (cx, cy): (f64, f64),
        radius: f64,
        style: &PointStyle,
    ) -> Result<(), RenderError> {
        if !(cx.is_finite() && cy.is_finite() && radius.is_finite()) {
            return Ok(());
        }
        let r = radius.max(0.0) as f32;
        let (cx, cy) = (cx as f32, cy as f32);
        // Four cubic arcs (kappa = 0.5523).
        let k = 0.552_284_8 * r;
        let mut pb = PathBuilder::new();
        pb.move_to(cx + r, cy);
        pb.cubic_to(cx + r, cy + k, cx + k, cy + r, cx, cy + r);
        pb.cubic_to(cx - k, cy + r, cx - r, cy + k, cx - r, cy);
        pb.cubic_to(cx - r, cy - k, cx - k, cy - r, cx, cy - r);
        pb.cubic_to(cx + k, cy - r, cx + r, cy - k, cx + r, cy);
        pb.close();
        let (f, s) = if style.filled {
            (
                Some(fill(style.color, style.alpha, FillRule::NonZero)),
                None,
            )
        } else {
            (None, Some(stroke(style.color, 1.0, style.alpha)))
        };
        self.push_path(pb.finish(), f, s);
        Ok(())
    }

    fn draw_shape(
        &mut self,
        (cx, cy): (f64, f64),
        radius: f64,
        style: &PointStyle,
    ) -> Result<(), RenderError> {
        if !(cx.is_finite() && cy.is_finite() && radius.is_finite()) {
            return Ok(());
        }
        let r = radius.max(0.0);
        let poly: Vec<(f64, f64)> = match style.shape {
            PointShape::Circle => return self.draw_circle((cx, cy), radius, style),
            PointShape::Square => vec![
                (cx - r, cy - r),
                (cx + r, cy - r),
                (cx + r, cy + r),
                (cx - r, cy + r),
            ],
            PointShape::Triangle => vec![(cx, cy - r), (cx + r, cy + r), (cx - r, cy + r)],
            PointShape::Diamond => vec![(cx, cy - r), (cx + r, cy), (cx, cy + r), (cx - r, cy)],
            PointShape::Plus | PointShape::Cross => {
                let segs = if style.shape == PointShape::Plus {
                    [(cx - r, cy, cx + r, cy), (cx, cy - r, cx, cy + r)]
                } else {
                    [
                        (cx - r, cy - r, cx + r, cy + r),
                        (cx - r, cy + r, cx + r, cy - r),
                    ]
                };
                let mut pb = PathBuilder::new();
                for (x0, y0, x1, y1) in segs {
                    pb.move_to(x0 as f32, y0 as f32);
                    pb.line_to(x1 as f32, y1 as f32);
                }
                let s = stroke(style.color, (r / 2.5).clamp(1.0, 3.0), style.alpha);
                self.push_path(pb.finish(), None, Some(s));
                return Ok(());
            }
        };
        let (f, s) = if style.filled {
            (
                Some(fill(style.color, style.alpha, FillRule::NonZero)),
                None,
            )
        } else {
            (None, Some(stroke(style.color, 1.0, style.alpha)))
        };
        self.push_path(poly_path(&poly, true), f, s);
        Ok(())
    }

    fn draw_line(&mut self, points: &[(f64, f64)], style: &LineStyle) -> Result<(), RenderError> {
        if style.width.is_nan() || style.width <= 0.0 {
            return Ok(());
        }
        let mut s = stroke(style.color, style.width, style.alpha);
        s.line_cap = match style.lineend {
            LineEnd::Butt => LineCap::Butt,
            LineEnd::Round => LineCap::Round,
            LineEnd::Square => LineCap::Square,
        };
        s.line_join = LineJoin::Round;
        let dash = style.linetype.dash_pattern();
        if !dash.is_empty() {
            s.dash = Some(StrokeDash {
                array: dash
                    .iter()
                    .flat_map(|&(d, g)| [d as f32, g as f32])
                    .collect(),
                offset: 0.0,
            });
        }
        self.push_path(poly_path(points, false), None, Some(s));
        Ok(())
    }

    fn draw_quadratic(
        &mut self,
        p0: (f64, f64),
        ctrl: (f64, f64),
        p1: (f64, f64),
        style: &LineStyle,
    ) -> Result<(), RenderError> {
        if !(finite(p0) && finite(ctrl) && finite(p1)) || style.width.is_nan() || style.width <= 0.0
        {
            return Ok(());
        }
        let mut pb = PathBuilder::new();
        pb.move_to(p0.0 as f32, p0.1 as f32);
        pb.quad_to(ctrl.0 as f32, ctrl.1 as f32, p1.0 as f32, p1.1 as f32);
        let mut s = stroke(style.color, style.width, style.alpha);
        let dash = style.linetype.dash_pattern();
        if !dash.is_empty() {
            s.dash = Some(StrokeDash {
                array: dash
                    .iter()
                    .flat_map(|&(d, g)| [d as f32, g as f32])
                    .collect(),
                offset: 0.0,
            });
        }
        self.push_path(pb.finish(), None, Some(s));
        Ok(())
    }

    fn draw_rect(
        &mut self,
        top_left: (f64, f64),
        bottom_right: (f64, f64),
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        if !(finite(top_left) && finite(bottom_right)) {
            return Ok(());
        }
        let (x0, x1) = (
            top_left.0.min(bottom_right.0),
            top_left.0.max(bottom_right.0),
        );
        let (y0, y1) = (
            top_left.1.min(bottom_right.1),
            top_left.1.max(bottom_right.1),
        );
        let r = style
            .corner_radius
            .max(0.0)
            .min((x1 - x0) / 2.0)
            .min((y1 - y0) / 2.0);
        let path = if r > 0.0 {
            let (x0, y0, x1, y1, r) = (x0 as f32, y0 as f32, x1 as f32, y1 as f32, r as f32);
            let mut pb = PathBuilder::new();
            pb.move_to(x0 + r, y0);
            pb.line_to(x1 - r, y0);
            pb.quad_to(x1, y0, x1, y0 + r);
            pb.line_to(x1, y1 - r);
            pb.quad_to(x1, y1, x1 - r, y1);
            pb.line_to(x0 + r, y1);
            pb.quad_to(x0, y1, x0, y1 - r);
            pb.line_to(x0, y0 + r);
            pb.quad_to(x0, y0, x0 + r, y0);
            pb.close();
            pb.finish()
        } else {
            rect_path(&Rect {
                x: x0,
                y: y0,
                width: x1 - x0,
                height: y1 - y0,
            })
        };
        let f = style.fill.map(|c| fill(c, style.alpha, FillRule::NonZero));
        let s = style
            .stroke
            .filter(|_| style.stroke_width > 0.0)
            .map(|c| stroke(c, style.stroke_width, style.alpha));
        self.push_path(path, f, s);
        Ok(())
    }

    fn draw_polygon(
        &mut self,
        points: &[(f64, f64)],
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        let f = style.fill.map(|c| fill(c, style.alpha, FillRule::NonZero));
        let s = style
            .stroke
            .filter(|_| style.stroke_width > 0.0)
            .map(|c| stroke(c, style.stroke_width, style.alpha));
        self.push_path(poly_path(points, true), f, s);
        Ok(())
    }

    fn draw_polygon_rings(
        &mut self,
        rings: &[Vec<(f64, f64)>],
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        let mut pb = PathBuilder::new();
        let mut any = false;
        for ring in rings {
            let pts: Vec<&(f64, f64)> = ring.iter().filter(|p| finite(**p)).collect();
            if pts.len() < 3 {
                continue;
            }
            pb.move_to(pts[0].0 as f32, pts[0].1 as f32);
            for p in &pts[1..] {
                pb.line_to(p.0 as f32, p.1 as f32);
            }
            pb.close();
            any = true;
        }
        if !any {
            return Ok(());
        }
        let f = style.fill.map(|c| fill(c, style.alpha, FillRule::EvenOdd));
        let s = style
            .stroke
            .filter(|_| style.stroke_width > 0.0)
            .map(|c| stroke(c, style.stroke_width, style.alpha));
        self.push_path(pb.finish(), f, s);
        Ok(())
    }

    fn draw_text(
        &mut self,
        text: &str,
        (x, y): (f64, f64),
        style: &TextStyle,
    ) -> Result<(), RenderError> {
        if text.is_empty()
            || !(x.is_finite() && y.is_finite())
            || style.size.is_nan()
            || style.size <= 0.0
        {
            return Ok(());
        }
        let bytes = font_bytes(style.family.as_deref(), style.face);
        let Some(font) = self.font(bytes) else {
            return Ok(());
        };
        let size = style.size;
        // Real advance widths (with kerning) for the anchor.
        let width = FontRef::try_from_slice(bytes).map_or(0.0, |f| {
            let sf = f.as_scaled(size as f32);
            let mut w = 0.0_f32;
            let mut prev = None;
            for ch in text.chars() {
                let id = sf.glyph_id(ch);
                if let Some(p) = prev {
                    w += sf.kern(p, id);
                }
                w += sf.h_advance(id);
                prev = Some(id);
            }
            w as f64
        });
        let dx = match style.anchor {
            TextAnchor::Start => 0.0,
            TextAnchor::Middle => -width / 2.0,
            TextAnchor::End => -width,
        };
        // `y` is the text's vertical middle (SVG `dominant-baseline:
        // middle`); the baseline sits about a third of an em below it.
        let start = Point::from_xy((x + dx) as f32, (y + size * 0.35) as f32);
        let angle = if style.angle.is_finite() {
            style.angle
        } else {
            0.0
        };
        let transform = (angle.abs() > 0.01)
            .then(|| Transform::from_rotate_at(angle as f32, x as f32, y as f32));
        let halo = style
            .bg_colour
            .filter(|_| style.bg_r > 0.0)
            .map(|c| Stroke {
                line_join: LineJoin::Round,
                ..stroke(c, 2.0 * style.bg_r, 1.0)
            });
        self.ops.push(Op::Text {
            start,
            font,
            size: size as f32,
            text: text.to_string(),
            fill: fill(style.color, 1.0, FillRule::NonZero),
            halo,
            transform,
        });
        Ok(())
    }

    fn draw_image(
        &mut self,
        bytes: &[u8],
        rect: &Rect,
        clip: &Rect,
        alpha: f64,
    ) -> Result<(), RenderError> {
        let data: krilla::Data = bytes.to_vec().into();
        let image = Image::from_png(data.clone(), true).or_else(|_| Image::from_jpeg(data, true));
        if let (Ok(image), Some(clip)) = (image, rect_path(clip)) {
            self.ops.push(Op::Image {
                image,
                rect: rect.clone(),
                clip,
                alpha: alpha as f32,
            });
        }
        Ok(())
    }

    fn begin_clip(&mut self, rect: &Rect) {
        if let Some(p) = rect_path(rect) {
            self.ops.push(Op::PushClip(p));
            self.open_clips += 1;
        }
    }

    fn end_clip(&mut self) {
        if self.open_clips > 0 {
            self.ops.push(Op::Pop);
            self.open_clips -= 1;
        }
    }

    fn begin_panel(&mut self, _index: usize, _label: &str, clip: &Rect, _data: &Rect) {
        self.begin_clip(clip);
    }

    fn end_panel(&mut self) {
        self.end_clip();
    }

    fn plot_area(&self) -> Rect {
        self.plot_area.clone()
    }

    fn total_area(&self) -> Rect {
        self.total_area.clone()
    }

    fn corner_claims(&self) -> Vec<Rect> {
        self.corner_claims.clone()
    }

    fn claim_corner_box(&mut self, rect: Rect) {
        self.corner_claims.push(rect);
    }

    fn clear_corner_claims(&mut self) {
        self.corner_claims.clear();
    }
}
