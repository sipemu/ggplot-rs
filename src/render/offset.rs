//! A [`DrawBackend`] adapter that draws a whole plot into a sub-rectangle of
//! another backend (`annotation_custom` insets, #407): the inner plot is laid
//! out at the origin as if its canvas were `w × h`, and every coordinate is
//! translated by `(dx, dy)` on the way to the host backend — so it works for
//! the SVG, plotters (PNG) and raster backends alike.

use crate::render::backend::{
    DrawBackend, FillPattern, LineStyle, PointStyle, RectStyle, TextStyle,
};
use crate::render::{Rect, RenderError};

pub(crate) struct OffsetBackend<'a> {
    pub inner: &'a mut dyn DrawBackend,
    pub dx: f64,
    pub dy: f64,
    /// The inner plot's panel and canvas, in its own (origin) coordinates.
    pub plot_area: Rect,
    pub total_area: Rect,
    corner_claims: Vec<Rect>,
}

impl<'a> OffsetBackend<'a> {
    pub fn new(
        inner: &'a mut dyn DrawBackend,
        dx: f64,
        dy: f64,
        plot_area: Rect,
        total_area: Rect,
    ) -> Self {
        OffsetBackend {
            inner,
            dx,
            dy,
            plot_area,
            total_area,
            corner_claims: Vec::new(),
        }
    }

    fn p(&self, (x, y): (f64, f64)) -> (f64, f64) {
        (x + self.dx, y + self.dy)
    }

    fn pts(&self, points: &[(f64, f64)]) -> Vec<(f64, f64)> {
        points.iter().map(|&q| self.p(q)).collect()
    }

    fn rings(&self, rings: &[Vec<(f64, f64)>]) -> Vec<Vec<(f64, f64)>> {
        rings.iter().map(|r| self.pts(r)).collect()
    }

    fn r(&self, r: &Rect) -> Rect {
        Rect {
            x: r.x + self.dx,
            y: r.y + self.dy,
            ..r.clone()
        }
    }
}

impl DrawBackend for OffsetBackend<'_> {
    fn draw_circle(
        &mut self,
        center: (f64, f64),
        radius: f64,
        style: &PointStyle,
    ) -> Result<(), RenderError> {
        let c = self.p(center);
        self.inner.draw_circle(c, radius, style)
    }
    fn draw_shape(
        &mut self,
        center: (f64, f64),
        radius: f64,
        style: &PointStyle,
    ) -> Result<(), RenderError> {
        let c = self.p(center);
        self.inner.draw_shape(c, radius, style)
    }
    fn draw_line(&mut self, points: &[(f64, f64)], style: &LineStyle) -> Result<(), RenderError> {
        let p = self.pts(points);
        self.inner.draw_line(&p, style)
    }
    fn draw_quadratic(
        &mut self,
        p0: (f64, f64),
        ctrl: (f64, f64),
        p1: (f64, f64),
        style: &LineStyle,
    ) -> Result<(), RenderError> {
        let (a, b, c) = (self.p(p0), self.p(ctrl), self.p(p1));
        self.inner.draw_quadratic(a, b, c, style)
    }
    fn draw_rect(
        &mut self,
        tl: (f64, f64),
        br: (f64, f64),
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        let (a, b) = (self.p(tl), self.p(br));
        self.inner.draw_rect(a, b, style)
    }
    fn draw_rect_pattern(
        &mut self,
        tl: (f64, f64),
        br: (f64, f64),
        pattern: &FillPattern,
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        let (a, b) = (self.p(tl), self.p(br));
        self.inner.draw_rect_pattern(a, b, pattern, style)
    }
    fn draw_text(
        &mut self,
        text: &str,
        pos: (f64, f64),
        style: &TextStyle,
    ) -> Result<(), RenderError> {
        let p = self.p(pos);
        self.inner.draw_text(text, p, style)
    }
    fn draw_text_with_title(
        &mut self,
        text: &str,
        pos: (f64, f64),
        style: &TextStyle,
        title: Option<&str>,
    ) -> Result<(), RenderError> {
        let p = self.p(pos);
        self.inner.draw_text_with_title(text, p, style, title)
    }
    fn draw_text_runs(
        &mut self,
        runs: &[(&str, Option<&str>)],
        pos: (f64, f64),
        style: &TextStyle,
    ) -> Result<(), RenderError> {
        let p = self.p(pos);
        self.inner.draw_text_runs(runs, p, style)
    }
    fn draw_polygon(
        &mut self,
        points: &[(f64, f64)],
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        let p = self.pts(points);
        self.inner.draw_polygon(&p, style)
    }
    fn draw_polygon_pattern(
        &mut self,
        points: &[(f64, f64)],
        pattern: &FillPattern,
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        let p = self.pts(points);
        self.inner.draw_polygon_pattern(&p, pattern, style)
    }
    fn draw_polygon_rings(
        &mut self,
        rings: &[Vec<(f64, f64)>],
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        let r = self.rings(rings);
        self.inner.draw_polygon_rings(&r, style)
    }
    fn draw_polygon_rings_pattern(
        &mut self,
        rings: &[Vec<(f64, f64)>],
        pattern: &FillPattern,
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        let r = self.rings(rings);
        self.inner.draw_polygon_rings_pattern(&r, pattern, style)
    }
    fn draw_polygon_rings_precise(
        &mut self,
        rings: &[Vec<(f64, f64)>],
        pattern: Option<&FillPattern>,
        precision: Option<u8>,
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        let r = self.rings(rings);
        self.inner
            .draw_polygon_rings_precise(&r, pattern, precision, style)
    }
    fn draw_image(
        &mut self,
        bytes: &[u8],
        rect: &Rect,
        clip: &Rect,
        alpha: f64,
    ) -> Result<(), RenderError> {
        let (r, c) = (self.r(rect), self.r(clip));
        self.inner.draw_image(bytes, &r, &c, alpha)
    }

    fn plot_area(&self) -> Rect {
        self.plot_area.clone()
    }
    fn total_area(&self) -> Rect {
        self.total_area.clone()
    }

    // Legend-corner bookkeeping stays local to the inset.
    fn corner_claims(&self) -> Vec<Rect> {
        self.corner_claims.clone()
    }
    fn claim_corner_box(&mut self, rect: Rect) {
        self.corner_claims.push(rect);
    }
    fn clear_corner_claims(&mut self) {
        self.corner_claims.clear();
    }

    fn begin_panel(&mut self, index: usize, label: &str, clip: &Rect, data: &Rect) {
        let (c, d) = (self.r(clip), self.r(data));
        self.inner.begin_panel(index, label, &c, &d)
    }
    fn end_panel(&mut self) {
        self.inner.end_panel()
    }
    fn set_clipping(&mut self, on: bool) {
        self.inner.set_clipping(on)
    }
    fn set_viewport(&mut self, rect: &Rect) {
        let r = self.r(rect);
        self.inner.set_viewport(&r)
    }
    fn set_tooltip(&mut self, t: Option<String>) {
        self.inner.set_tooltip(t)
    }
    fn set_mark_axis(&mut self, v: Option<String>) {
        self.inner.set_mark_axis(v)
    }
    fn set_mark_series(&mut self, v: Option<String>) {
        self.inner.set_mark_series(v)
    }
    fn set_mark_value(&mut self, v: Option<String>) {
        self.inner.set_mark_value(v)
    }
    fn set_mark_geom(&mut self, g: Option<&str>) {
        self.inner.set_mark_geom(g)
    }
    fn warn(&mut self, m: String) {
        self.inner.warn(m)
    }
    fn set_mark_part(&mut self, p: Option<&str>) {
        self.inner.set_mark_part(p)
    }
    fn set_mark_attrs(&mut self, a: Vec<(String, String)>) {
        self.inner.set_mark_attrs(a)
    }
    fn wants_formatted_values(&self) -> bool {
        self.inner.wants_formatted_values()
    }
    fn set_mark_fvalue(&mut self, v: Option<String>) {
        self.inner.set_mark_fvalue(v)
    }
    fn set_mark_fx(&mut self, v: Option<String>) {
        self.inner.set_mark_fx(v)
    }
    fn set_mark_class(&mut self, v: Option<String>) {
        self.inner.set_mark_class(v)
    }
    fn set_mark_group(&mut self, v: Option<(String, String)>) {
        self.inner.set_mark_group(v)
    }
    fn set_layer_attrs(&mut self, a: Vec<(String, String)>) {
        self.inner.set_layer_attrs(a)
    }
}
