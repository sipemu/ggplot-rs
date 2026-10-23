//! A self-contained RGBA raster [`DrawBackend`] (feature `canvas`).
//!
//! Unlike the plotters bitmap backend it compiles to wasm and is tuned for
//! large-N: it rasterises points/lines/polygons/text into an RGBA byte buffer in
//! pure Rust (text via `ab_glyph` + the bundled font), so there's no per-mark
//! DOM node and no wasm↔JS call per draw. Hand `into_rgba()` to a `<canvas>`
//! (`putImageData`) in the browser, or `into_png()` a file natively.

// The low-level rasterisers take (buf, w, h, coords…, colour, alpha) — plenty of
// scalar arguments by nature.
#![allow(clippy::too_many_arguments)]

use ab_glyph::{point, Font, FontRef, PxScale, ScaleFont};

use super::backend::{
    DrawBackend, FontFace, LineStyle, PointStyle, RectStyle, TextAnchor, TextStyle,
};
use super::{Rect, RenderError};

const FONT: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans.ttf");

/// An in-memory RGBA canvas (row-major, 4 bytes/pixel, opaque white ground).
pub struct PixelBackend {
    width: usize,
    height: usize,
    buf: Vec<u8>,
    plot_area: Rect,
    total_area: Rect,
    font: FontRef<'static>,
}

impl PixelBackend {
    pub fn new(width: u32, height: u32, plot_area: Rect) -> Self {
        let (w, h) = (width as usize, height as usize);
        PixelBackend {
            width: w,
            height: h,
            buf: vec![255; w * h * 4], // white, opaque
            plot_area,
            total_area: Rect {
                x: 0.0,
                y: 0.0,
                width: width as f64,
                height: height as f64,
            },
            font: FontRef::try_from_slice(FONT).expect("bundled font parses"),
        }
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width as u32, self.height as u32)
    }

    /// The raw RGBA buffer (`width*height*4` bytes) for `ctx.putImageData`.
    pub fn into_rgba(self) -> Vec<u8> {
        self.buf
    }

    /// Encode the canvas to PNG bytes.
    pub fn into_png(self) -> Result<Vec<u8>, RenderError> {
        let img = image::RgbaImage::from_raw(self.width as u32, self.height as u32, self.buf)
            .ok_or_else(|| RenderError::BackendError("buffer size mismatch".into()))?;
        let mut out = std::io::Cursor::new(Vec::new());
        img.write_to(&mut out, image::ImageOutputFormat::Png)
            .map_err(|e| RenderError::BackendError(format!("{e:?}")))?;
        Ok(out.into_inner())
    }
}

/// Alpha-blend one pixel (source-over onto the opaque ground).
fn blend(buf: &mut [u8], w: usize, h: usize, x: i32, y: i32, (r, g, b): (u8, u8, u8), a: f64) {
    if x < 0 || y < 0 || x as usize >= w || y as usize >= h {
        return;
    }
    let a = a.clamp(0.0, 1.0);
    if a <= 0.0 {
        return;
    }
    let i = (y as usize * w + x as usize) * 4;
    let inv = 1.0 - a;
    buf[i] = (r as f64 * a + buf[i] as f64 * inv).round() as u8;
    buf[i + 1] = (g as f64 * a + buf[i + 1] as f64 * inv).round() as u8;
    buf[i + 2] = (b as f64 * a + buf[i + 2] as f64 * inv).round() as u8;
    buf[i + 3] = 255;
}

/// An anti-aliased filled disc — the hot path for scatter points.
fn fill_circle(
    buf: &mut [u8],
    w: usize,
    h: usize,
    cx: f64,
    cy: f64,
    r: f64,
    c: (u8, u8, u8),
    a: f64,
) {
    let r = r.max(0.5);
    let (x0, x1) = ((cx - r - 1.0).floor() as i32, (cx + r + 1.0).ceil() as i32);
    let (y0, y1) = ((cy - r - 1.0).floor() as i32, (cy + r + 1.0).ceil() as i32);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let d = ((x as f64 + 0.5 - cx).powi(2) + (y as f64 + 0.5 - cy).powi(2)).sqrt();
            let cov = (r - d + 0.5).clamp(0.0, 1.0); // 1px AA edge
            if cov > 0.0 {
                blend(buf, w, h, x, y, c, cov * a);
            }
        }
    }
}

/// A thick segment, drawn by stamping small discs along it.
#[allow(clippy::too_many_arguments)]
fn segment(
    buf: &mut [u8],
    w: usize,
    h: usize,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    c: (u8, u8, u8),
    width: f64,
    a: f64,
) {
    let r = (width / 2.0).max(0.5);
    let (dx, dy) = (x1 - x0, y1 - y0);
    let len = (dx * dx + dy * dy).sqrt().max(1.0);
    let steps = len.ceil() as i32;
    for s in 0..=steps {
        let t = s as f64 / steps as f64;
        fill_circle(buf, w, h, x0 + dx * t, y0 + dy * t, r, c, a);
    }
}

/// Even-odd scanline polygon fill, one ring.
fn fill_polygon(buf: &mut [u8], w: usize, h: usize, pts: &[(f64, f64)], c: (u8, u8, u8), a: f64) {
    fill_polygon_rings(buf, w, h, &[pts], c, a);
}

/// Even-odd scanline fill across every ring at once (`geom_sf`'s polygon
/// holes, #178): every ring's edges are collected before sorting each
/// scanline's crossings, so a hole ring's pair of crossings on a given row
/// cancels the exterior's fill between them — the standard even-odd
/// definition generalises directly from one ring to several.
fn fill_polygon_rings(
    buf: &mut [u8],
    w: usize,
    h: usize,
    rings: &[&[(f64, f64)]],
    c: (u8, u8, u8),
    a: f64,
) {
    let (mut y0, mut y1) = (f64::INFINITY, f64::NEG_INFINITY);
    for ring in rings {
        if ring.len() < 3 {
            continue;
        }
        for p in ring.iter() {
            y0 = y0.min(p.1);
            y1 = y1.max(p.1);
        }
    }
    if !(y0.is_finite() && y1.is_finite()) {
        return;
    }
    let (y0, y1) = (y0.floor() as i32, y1.ceil() as i32);
    for y in y0..=y1 {
        let yc = y as f64 + 0.5;
        let mut xs: Vec<f64> = Vec::new();
        for ring in rings {
            let n = ring.len();
            if n < 3 {
                continue;
            }
            for i in 0..n {
                let (ax, ay) = ring[i];
                let (bx, by) = ring[(i + 1) % n];
                if (ay <= yc && by > yc) || (by <= yc && ay > yc) {
                    xs.push(ax + (yc - ay) / (by - ay) * (bx - ax));
                }
            }
        }
        xs.sort_by(|p, q| p.total_cmp(q));
        for span in xs.chunks(2) {
            if let [xa, xb] = span {
                for x in xa.round() as i32..=xb.round() as i32 {
                    blend(buf, w, h, x, y, c, a);
                }
            }
        }
    }
}

impl DrawBackend for PixelBackend {
    fn plot_area(&self) -> Rect {
        self.plot_area.clone()
    }
    fn total_area(&self) -> Rect {
        self.total_area.clone()
    }

    fn draw_circle(
        &mut self,
        (cx, cy): (f64, f64),
        radius: f64,
        style: &PointStyle,
    ) -> Result<(), RenderError> {
        fill_circle(
            &mut self.buf,
            self.width,
            self.height,
            cx,
            cy,
            radius,
            style.color,
            style.alpha,
        );
        Ok(())
    }

    fn draw_line(&mut self, pts: &[(f64, f64)], style: &LineStyle) -> Result<(), RenderError> {
        for w in pts.windows(2) {
            segment(
                &mut self.buf,
                self.width,
                self.height,
                w[0].0,
                w[0].1,
                w[1].0,
                w[1].1,
                style.color,
                style.width,
                style.alpha,
            );
        }
        Ok(())
    }

    fn draw_rect(
        &mut self,
        (x0, y0): (f64, f64),
        (x1, y1): (f64, f64),
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        // No native rounded-rect primitive here: approximate as a polygon
        // (a plain rectangle when corner_radius is 0, same as before).
        let r = if style.corner_radius > 0.0 {
            super::backend::rounded_rect_points(x0, y0, x1, y1, style.corner_radius)
        } else {
            vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
        };
        if let Some(fill) = style.fill {
            fill_polygon(
                &mut self.buf,
                self.width,
                self.height,
                &r,
                fill,
                style.alpha,
            );
        }
        if let Some(stroke) = style.stroke {
            let n = r.len();
            for i in 0..n {
                let (a, b) = (r[i], r[(i + 1) % n]);
                segment(
                    &mut self.buf,
                    self.width,
                    self.height,
                    a.0,
                    a.1,
                    b.0,
                    b.1,
                    stroke,
                    style.stroke_width,
                    style.alpha,
                );
            }
        }
        Ok(())
    }

    fn draw_polygon(&mut self, pts: &[(f64, f64)], style: &RectStyle) -> Result<(), RenderError> {
        if let Some(fill) = style.fill {
            fill_polygon(
                &mut self.buf,
                self.width,
                self.height,
                pts,
                fill,
                style.alpha,
            );
        }
        if let Some(stroke) = style.stroke {
            for w in pts.windows(2) {
                segment(
                    &mut self.buf,
                    self.width,
                    self.height,
                    w[0].0,
                    w[0].1,
                    w[1].0,
                    w[1].1,
                    stroke,
                    style.stroke_width,
                    style.alpha,
                );
            }
            if let (Some(first), Some(last)) = (pts.first(), pts.last()) {
                segment(
                    &mut self.buf,
                    self.width,
                    self.height,
                    last.0,
                    last.1,
                    first.0,
                    first.1,
                    stroke,
                    style.stroke_width,
                    style.alpha,
                );
            }
        }
        Ok(())
    }

    fn draw_polygon_rings(
        &mut self,
        rings: &[Vec<(f64, f64)>],
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        if let Some(fill) = style.fill {
            let ring_refs: Vec<&[(f64, f64)]> = rings.iter().map(Vec::as_slice).collect();
            fill_polygon_rings(
                &mut self.buf,
                self.width,
                self.height,
                &ring_refs,
                fill,
                style.alpha,
            );
        }
        if let Some(stroke) = style.stroke {
            for ring in rings {
                if ring.len() < 2 {
                    continue;
                }
                for w in ring.windows(2) {
                    segment(
                        &mut self.buf,
                        self.width,
                        self.height,
                        w[0].0,
                        w[0].1,
                        w[1].0,
                        w[1].1,
                        stroke,
                        style.stroke_width,
                        style.alpha,
                    );
                }
                if let (Some(first), Some(last)) = (ring.first(), ring.last()) {
                    segment(
                        &mut self.buf,
                        self.width,
                        self.height,
                        last.0,
                        last.1,
                        first.0,
                        first.1,
                        stroke,
                        style.stroke_width,
                        style.alpha,
                    );
                }
            }
        }
        Ok(())
    }

    fn draw_text(
        &mut self,
        text: &str,
        (x, y): (f64, f64),
        style: &TextStyle,
    ) -> Result<(), RenderError> {
        let scale = PxScale::from(style.size as f32);
        let font = &self.font;
        let scaled = font.as_scaled(scale);
        let width: f32 = text
            .chars()
            .map(|c| scaled.h_advance(font.glyph_id(c)))
            .sum();
        let start_x = match style.anchor {
            TextAnchor::Start => x,
            TextAnchor::Middle => x - width as f64 / 2.0,
            TextAnchor::End => x - width as f64,
        };
        // Approximate the SVG "middle" baseline; bold faked by a bump in coverage.
        let baseline = (y + style.size * 0.35) as f32;
        let bold = style.face == FontFace::Bold;
        let (bw, bh, color) = (self.width, self.height, style.color);
        let mut pen = start_x as f32;
        for ch in text.chars() {
            let gid = font.glyph_id(ch);
            let glyph = gid.with_scale_and_position(scale, point(pen, baseline));
            if let Some(outlined) = font.outline_glyph(glyph) {
                let bounds = outlined.px_bounds();
                outlined.draw(|gx, gy, cov| {
                    let a = if bold {
                        (cov as f64 * 1.4).min(1.0)
                    } else {
                        cov as f64
                    };
                    blend(
                        &mut self.buf,
                        bw,
                        bh,
                        bounds.min.x as i32 + gx as i32,
                        bounds.min.y as i32 + gy as i32,
                        color,
                        a,
                    );
                });
            }
            pen += scaled.h_advance(gid);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(buf: &[u8], w: usize, x: i32, y: i32) -> (u8, u8, u8) {
        let i = (y as usize * w + x as usize) * 4;
        (buf[i], buf[i + 1], buf[i + 2])
    }

    /// A ring after the other is a hole cut out of the fill under the
    /// even-odd rule (`geom_sf`'s `Polygon`, #178) — a 20×20 square with a
    /// 6×6 hole in the middle leaves the hole's centre at the white ground,
    /// while a point between the hole and the exterior edge is filled.
    #[test]
    fn fill_polygon_rings_cuts_out_a_hole() {
        let (w, h) = (20usize, 20usize);
        let mut buf = vec![255u8; w * h * 4];
        let exterior: Vec<(f64, f64)> = vec![(1.0, 1.0), (19.0, 1.0), (19.0, 19.0), (1.0, 19.0)];
        let hole: Vec<(f64, f64)> = vec![(7.0, 7.0), (13.0, 7.0), (13.0, 13.0), (7.0, 13.0)];
        fill_polygon_rings(&mut buf, w, h, &[&exterior, &hole], (200, 50, 50), 1.0);
        assert_eq!(pixel(&buf, w, 10, 10), (255, 255, 255), "hole stays blank");
        assert_eq!(
            pixel(&buf, w, 3, 3),
            (200, 50, 50),
            "ring between is filled"
        );
        // Without the hole ring, the same point would be filled — proving
        // it's the second ring that cuts it, not some other effect.
        let mut filled = vec![255u8; w * h * 4];
        fill_polygon_rings(&mut filled, w, h, &[&exterior], (200, 50, 50), 1.0);
        assert_eq!(pixel(&filled, w, 10, 10), (200, 50, 50));
    }

    /// Two disjoint rings (a multipolygon's two parts, drawn in one call)
    /// each fill independently.
    #[test]
    fn fill_polygon_rings_handles_disjoint_rings() {
        let (w, h) = (20usize, 20usize);
        let mut buf = vec![255u8; w * h * 4];
        let a: Vec<(f64, f64)> = vec![(1.0, 1.0), (5.0, 1.0), (5.0, 5.0), (1.0, 5.0)];
        let b: Vec<(f64, f64)> = vec![(14.0, 14.0), (18.0, 14.0), (18.0, 18.0), (14.0, 18.0)];
        fill_polygon_rings(&mut buf, w, h, &[&a, &b], (10, 20, 30), 1.0);
        assert_eq!(pixel(&buf, w, 3, 3), (10, 20, 30));
        assert_eq!(pixel(&buf, w, 16, 16), (10, 20, 30));
        assert_eq!(
            pixel(&buf, w, 10, 10),
            (255, 255, 255),
            "gap between stays blank"
        );
    }
}
