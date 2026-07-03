//! A self-contained SVG `DrawBackend` — a *second* backend (no plotters),
//! proving the `DrawBackend` abstraction: the same `PlotRenderer` drives it.
//!
//! It emits SVG elements directly, so SVG output needs no glyph rasterization
//! (text is `<text>` with font attributes the viewer renders).

use super::backend::{
    DrawBackend, FontFace, LineStyle, PointStyle, RectStyle, TextAnchor, TextStyle,
};
use super::{Rect, RenderError};

/// Accumulates SVG markup for a plot rendered via [`DrawBackend`].
pub struct SvgBackend {
    plot_area: Rect,
    total_area: Rect,
    body: String,
}

impl SvgBackend {
    pub fn new(width: u32, height: u32, plot_area: Rect) -> Self {
        SvgBackend {
            plot_area,
            total_area: Rect {
                x: 0.0,
                y: 0.0,
                width: width as f64,
                height: height as f64,
            },
            body: String::new(),
        }
    }

    /// Wrap the accumulated elements in a complete `<svg>` document.
    pub fn finish(self) -> String {
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" \
             viewBox=\"0 0 {0} {1}\">{}</svg>",
            self.total_area.width as i64, self.total_area.height as i64, self.body
        )
    }
}

fn rgb((r, g, b): (u8, u8, u8)) -> String {
    format!("#{r:02X}{g:02X}{b:02X}")
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn points(pts: &[(f64, f64)]) -> String {
    pts.iter()
        .map(|(x, y)| format!("{x:.2},{y:.2}"))
        .collect::<Vec<_>>()
        .join(" ")
}

impl DrawBackend for SvgBackend {
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
        self.body.push_str(&format!(
            "<circle cx=\"{cx:.2}\" cy=\"{cy:.2}\" r=\"{radius:.2}\" fill=\"{}\" fill-opacity=\"{:.3}\"/>",
            rgb(style.color), style.alpha
        ));
        Ok(())
    }

    fn draw_line(&mut self, pts: &[(f64, f64)], style: &LineStyle) -> Result<(), RenderError> {
        let dash = match style
            .linetype
            .pattern()
            .iter()
            .flat_map(|(d, g)| [*d, *g])
            .map(|v| format!("{v}"))
            .collect::<Vec<_>>()
            .join(",")
        {
            s if s.is_empty() => String::new(),
            s => format!(" stroke-dasharray=\"{s}\""),
        };
        self.body.push_str(&format!(
            "<polyline points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\" stroke-opacity=\"{:.3}\"{}/>",
            points(pts), rgb(style.color), style.width, style.alpha, dash
        ));
        Ok(())
    }

    fn draw_rect(
        &mut self,
        (x0, y0): (f64, f64),
        (x1, y1): (f64, f64),
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        let (x, y) = (x0.min(x1), y0.min(y1));
        let (w, h) = ((x1 - x0).abs(), (y1 - y0).abs());
        let fill = style.fill.map(rgb).unwrap_or_else(|| "none".into());
        let stroke = style.stroke.map(rgb).unwrap_or_else(|| "none".into());
        self.body.push_str(&format!(
            "<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{w:.2}\" height=\"{h:.2}\" fill=\"{fill}\" \
             fill-opacity=\"{:.3}\" stroke=\"{stroke}\" stroke-width=\"{:.2}\"/>",
            style.alpha, style.stroke_width
        ));
        Ok(())
    }

    fn draw_polygon(&mut self, pts: &[(f64, f64)], style: &RectStyle) -> Result<(), RenderError> {
        let fill = style.fill.map(rgb).unwrap_or_else(|| "none".into());
        let stroke = style.stroke.map(rgb).unwrap_or_else(|| "none".into());
        self.body.push_str(&format!(
            "<polygon points=\"{}\" fill=\"{fill}\" fill-opacity=\"{:.3}\" stroke=\"{stroke}\" stroke-width=\"{:.2}\"/>",
            points(pts), style.alpha, style.stroke_width
        ));
        Ok(())
    }

    fn draw_text(
        &mut self,
        text: &str,
        (x, y): (f64, f64),
        style: &TextStyle,
    ) -> Result<(), RenderError> {
        let anchor = match style.anchor {
            TextAnchor::Start => "start",
            TextAnchor::Middle => "middle",
            TextAnchor::End => "end",
        };
        let family = style.family.as_deref().unwrap_or("sans-serif");
        let weight = if style.face == FontFace::Bold {
            " font-weight=\"bold\""
        } else {
            ""
        };
        let fstyle = if style.face == FontFace::Italic {
            " font-style=\"italic\""
        } else {
            ""
        };
        let transform = if style.angle.abs() > 0.01 {
            format!(" transform=\"rotate({:.1} {x:.2} {y:.2})\"", style.angle)
        } else {
            String::new()
        };
        self.body.push_str(&format!(
            "<text x=\"{x:.2}\" y=\"{y:.2}\" font-size=\"{:.2}\" text-anchor=\"{anchor}\" \
             dominant-baseline=\"middle\" font-family=\"{family}\"{weight}{fstyle} fill=\"{}\"{transform}>{}</text>",
            style.size, rgb(style.color), escape(text)
        ));
        Ok(())
    }
}
