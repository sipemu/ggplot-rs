//! A self-contained SVG `DrawBackend` — a *second* backend (no plotters),
//! proving the `DrawBackend` abstraction: the same `PlotRenderer` drives it.
//!
//! It emits SVG elements directly, so SVG output needs no glyph rasterization
//! (text is `<text>` with font attributes the viewer renders).

use super::backend::{
    DrawBackend, FontFace, LineEnd, LineStyle, PointShape, PointStyle, RectStyle, TextAnchor,
    TextStyle,
};
use super::{Rect, RenderError};

/// Accumulates SVG markup for a plot rendered via [`DrawBackend`].
///
/// Every number written to an attribute is finite: marks whose geometry is
/// `NaN`/`±inf` are dropped (a polyline keeps its finite vertices), and
/// non-finite style values fall back to safe defaults. All data-derived
/// attribute values and text go through [`escape`].
pub struct SvgBackend {
    plot_area: Rect,
    total_area: Rect,
    body: String,
    tooltip: Option<String>,
    axis_key: Option<String>,
    series_key: Option<String>,
    value_key: Option<String>,
    mark_part: Option<String>,
    mark_attrs: Vec<(String, String)>,
    root_attrs: Vec<(String, String)>,
    warnings: Vec<String>,
    /// Emit the root `data-plot` panel rect (off for composite pages, whose
    /// root is not a single panel).
    plot_attr: bool,
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
            tooltip: None,
            axis_key: None,
            series_key: None,
            value_key: None,
            mark_part: None,
            mark_attrs: Vec::new(),
            root_attrs: Vec::new(),
            warnings: Vec::new(),
            plot_attr: true,
        }
    }

    /// Add extra attributes to the root `<svg>` element (e.g. the
    /// `data-domain` family from [`root_data_attrs`]). Names must be plain
    /// attribute names; values are escaped on output.
    pub fn set_root_attrs(&mut self, attrs: Vec<(String, String)>) {
        self.root_attrs = attrs;
    }

    /// Omit the root `data-plot` attribute (composite pages).
    pub(crate) fn without_plot_attr(&mut self) {
        self.plot_attr = false;
    }

    /// Append pre-rendered, already-escaped SVG markup (e.g. a nested plot
    /// fragment) to the body. Crate-internal: callers guarantee well-formedness.
    pub(crate) fn push_raw(&mut self, markup: &str) {
        self.body.push_str(markup);
    }

    /// The accumulated body markup (without the root element).
    pub(crate) fn body(&self) -> &str {
        &self.body
    }

    /// Take the warnings reported while drawing (see [`DrawBackend::warn`]).
    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }

    /// Emit `<tag attrs/>`, or `<tag attrs data-…><title>tip</title></tag>`
    /// when a tooltip / mark metadata is set (native SVG hover + host grouping).
    fn push_mark(&mut self, tag: &str, attrs: &str) {
        // Built-ins first, then user `data-*` attrs — re-validated here as
        // defence in depth even though `geom::set_row_extras` already
        // derives them only from validated `data_attr` mappings, because
        // `set_mark_attrs` is a public `DrawBackend` method any caller could
        // invoke directly. A later entry with the same name replaces an
        // earlier one rather than duplicating it — including `mark_part`'s
        // own `data-part`, added last, so it always wins over anything a
        // caller supplied under that name (an element never carries the
        // same attribute twice, which is an XML parse error).
        let mut pairs: Vec<(String, String)> = Vec::new();
        for (name, val) in [
            ("data-x", &self.axis_key),
            ("data-series", &self.series_key),
            ("data-value", &self.value_key),
        ] {
            if let Some(v) = val {
                pairs.push((name.to_string(), v.clone()));
            }
        }
        for (name, val) in &self.mark_attrs {
            if !is_valid_mark_attr_name(name) {
                continue;
            }
            match pairs.iter_mut().find(|(n, _)| n == name) {
                Some(existing) => existing.1 = val.clone(),
                None => pairs.push((name.clone(), val.clone())),
            }
        }
        // A mark part (e.g. an arrowhead) is decorative: no `<title>`, but it
        // still carries `data-part` plus `aria-hidden` so assistive tech
        // skips it (the parent mark already carries the semantic tooltip).
        // Routed through the same dedup as the other attrs.
        if let Some(part) = &self.mark_part {
            match pairs.iter_mut().find(|(n, _)| n == "data-part") {
                Some(existing) => existing.1 = part.clone(),
                None => pairs.push(("data-part".to_string(), part.clone())),
            }
        }
        let mut data = String::new();
        for (name, val) in &pairs {
            data.push_str(&format!(" {name}=\"{}\"", escape(val)));
        }
        if self.mark_part.is_some() {
            data.push_str(" aria-hidden=\"true\"");
        }
        let mark = if self.mark_part.is_some() {
            format!("<{tag} {attrs}{data}/>")
        } else {
            match &self.tooltip {
                Some(t) => format!("<{tag} {attrs}{data}><title>{}</title></{tag}>", escape(t)),
                None => format!("<{tag} {attrs}{data}/>"),
            }
        };
        self.body.push_str(&mark);
    }

    fn root_open(&self, prefix: &str) -> String {
        let p = &self.plot_area;
        let (w, h) = (self.total_area.width as i64, self.total_area.height as i64);
        let mut extra = String::new();
        for (k, v) in &self.root_attrs {
            extra.push_str(&format!(" {k}=\"{}\"", escape(v)));
        }
        let plot = if self.plot_attr {
            format!(
                " data-plot=\"{} {} {} {}\"",
                num(p.x),
                num(p.y),
                num(p.width),
                num(p.height),
            )
        } else {
            String::new()
        };
        format!("<svg {prefix}width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\"{plot}{extra}>")
    }

    /// Wrap the accumulated elements in a complete `<svg>` document. The
    /// `data-plot` attribute records the panel (data) area in viewBox units so a
    /// host can map screen coordinates back to data (e.g. for zoom/crosshair).
    pub fn finish(self) -> String {
        let open = self.root_open("xmlns=\"http://www.w3.org/2000/svg\" ");
        format!("{open}{}</svg>", self.body)
    }

    /// Wrap the accumulated elements in a *nested* `<svg>` positioned at
    /// `(x, y)` in a parent SVG's user space — no `xmlns` (it inherits the
    /// parent's namespace), so the result can be pasted into a larger SVG as-is.
    /// `data-plot` stays in this fragment's own viewBox units.
    pub fn finish_fragment(self, x: f64, y: f64) -> String {
        let open = self.root_open(&format!("x=\"{}\" y=\"{}\" ", num(x), num(y)));
        format!("{open}{}</svg>", self.body)
    }
}

/// Format a coordinate with two decimals; non-finite becomes `0` (callers
/// drop marks with non-finite geometry before this, so this is a last resort).
fn num(v: f64) -> String {
    if v.is_finite() {
        format!("{v:.2}")
    } else {
        "0".to_string()
    }
}

fn finite_or(v: f64, fallback: f64) -> f64 {
    if v.is_finite() {
        v
    } else {
        fallback
    }
}

/// Root `<svg>` data attributes describing the trained position scales, for
/// hosts that map screen coordinates back to data (zoom, crosshair, brushing):
///
/// - `data-xdomain="x0 x1"` / `data-ydomain="y0 y1"` — the data values at the
///   panel's left/right (bottom/top) edges, i.e. the trained limits *after*
///   expansion, for a continuous (numeric or date-time, epoch seconds) axis.
///   Values are in the scale's transformed space (e.g. log10 units for
///   `scale_x_log10`).
/// - `data-domain="x0 x1 y0 y1"` — both of the above, emitted only when both
///   axes are continuous.
/// - `data-xlevels` / `data-ylevels` — a JSON array of the level labels for a
///   discrete axis, in axis order.
/// - `data-flip="true"` under `coord_flip` (x is then drawn vertically).
///
/// The `x`/`y` names refer to the x/y *aesthetics*. Omitted entirely for
/// plots with free facet scales (each panel has its own domain).
pub fn root_data_attrs(built: &crate::build::BuiltPlot) -> Vec<(String, String)> {
    use crate::aes::Aesthetic;
    let mut out = Vec::new();
    if !built.panel_scales.is_empty() {
        return out;
    }
    let axis = |aes: &Aesthetic| -> (Option<(f64, f64)>, Option<String>) {
        match built.scales.get(aes) {
            Some(s) if s.is_discrete() => {
                let labels: Vec<String> = s
                    .breaks()
                    .into_iter()
                    .map(|(_, l)| json_string(&l))
                    .collect();
                (None, Some(format!("[{}]", labels.join(","))))
            }
            Some(s) => (s.expanded_domain(), None),
            None => (None, None),
        }
    };
    let (xd, xl) = axis(&Aesthetic::X);
    let (yd, yl) = axis(&Aesthetic::Y);
    if let (Some((x0, x1)), Some((y0, y1))) = (xd, yd) {
        out.push((
            "data-domain".into(),
            format!("{} {} {} {}", g(x0), g(x1), g(y0), g(y1)),
        ));
    }
    if let Some((a, b)) = xd {
        out.push(("data-xdomain".into(), format!("{} {}", g(a), g(b))));
    }
    if let Some((a, b)) = yd {
        out.push(("data-ydomain".into(), format!("{} {}", g(a), g(b))));
    }
    if let Some(l) = xl {
        out.push(("data-xlevels".into(), l));
    }
    if let Some(l) = yl {
        out.push(("data-ylevels".into(), l));
    }
    if built.coord.is_flipped() {
        out.push(("data-flip".into(), "true".into()));
    }
    out
}

/// Shortest round-trip formatting of a finite number (domains are filtered to
/// finite values before this is called).
fn g(v: f64) -> String {
    format!("{v}")
}

/// Minimal JSON string literal (quotes, backslashes and control characters
/// escaped); the result is attribute-escaped again on output.
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Defence-in-depth gate for `push_mark`'s `mark_attrs`: `^data-[a-z0-9][a-z0-9-]*$`.
/// `geom::set_row_extras` only ever supplies already-validated names, but
/// `set_mark_attrs` is a public `DrawBackend` method any caller (a custom
/// `Geom` impl, or misuse from outside the crate) could invoke directly with
/// an arbitrary string, so this is checked again right before it reaches
/// the wire — never trust a single validation layer for attribute-name
/// injection.
fn is_valid_mark_attr_name(name: &str) -> bool {
    match name.strip_prefix("data-") {
        Some(suffix) => crate::aes::is_valid_data_attr_name(suffix),
        None => false,
    }
}

fn rgb((r, g, b): (u8, u8, u8)) -> String {
    format!("#{r:02X}{g:02X}{b:02X}")
}

/// Escape text for use in SVG element content *and* quoted attribute values.
/// Quotes must be escaped too: data values land in `data-x="…"`, and the SVG is
/// routinely inlined into HTML, so an unescaped `"` would let data inject
/// attributes (e.g. event handlers). Characters that are not legal in XML 1.0
/// (C0 controls other than tab/LF/CR) are dropped so the document stays
/// well-formed.
fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            '\t' | '\n' | '\r' => out.push(c),
            c if (c as u32) < 0x20 || c == '\u{FFFE}' || c == '\u{FFFF}' => {}
            c => out.push(c),
        }
    }
    out
}

fn points(pts: &[(f64, f64)]) -> String {
    pts.iter()
        .map(|(x, y)| format!("{x:.2},{y:.2}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn finite_pts(pts: &[(f64, f64)]) -> Vec<(f64, f64)> {
    pts.iter()
        .copied()
        .filter(|(x, y)| x.is_finite() && y.is_finite())
        .collect()
}

impl DrawBackend for SvgBackend {
    fn plot_area(&self) -> Rect {
        self.plot_area.clone()
    }
    fn total_area(&self) -> Rect {
        self.total_area.clone()
    }

    fn set_tooltip(&mut self, tooltip: Option<String>) {
        self.tooltip = tooltip;
    }

    fn set_mark_axis(&mut self, key: Option<String>) {
        self.axis_key = key;
    }

    fn set_mark_series(&mut self, series: Option<String>) {
        self.series_key = series;
    }

    fn set_mark_value(&mut self, value: Option<String>) {
        self.value_key = value;
    }

    fn warn(&mut self, message: String) {
        self.warnings.push(message);
    }

    fn set_mark_part(&mut self, part: Option<&str>) {
        self.mark_part = part.map(|s| s.to_string());
    }

    fn set_mark_attrs(&mut self, attrs: Vec<(String, String)>) {
        self.mark_attrs = attrs;
    }

    fn draw_circle(
        &mut self,
        (cx, cy): (f64, f64),
        radius: f64,
        style: &PointStyle,
    ) -> Result<(), RenderError> {
        if !(cx.is_finite() && cy.is_finite() && radius.is_finite()) {
            return Ok(());
        }
        let attrs = format!(
            "cx=\"{cx:.2}\" cy=\"{cy:.2}\" r=\"{:.2}\" fill=\"{}\" fill-opacity=\"{:.3}\"",
            radius.max(0.0),
            rgb(style.color),
            finite_or(style.alpha, 1.0)
        );
        self.push_mark("circle", &attrs);
        Ok(())
    }

    /// Native shapes: filled polygons for square/triangle/diamond, a single
    /// stroked `<path>` for the `+` / `×` glyphs (so each point stays one
    /// hoverable mark carrying its `data-*` attributes).
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
        let color = rgb(style.color);
        let alpha = finite_or(style.alpha, 1.0);
        let poly = |pts: &[(f64, f64)]| {
            if style.filled {
                format!(
                    "points=\"{}\" fill=\"{color}\" fill-opacity=\"{alpha:.3}\"",
                    points(pts)
                )
            } else {
                format!(
                    "points=\"{}\" fill=\"none\" stroke=\"{color}\" stroke-opacity=\"{alpha:.3}\"",
                    points(pts)
                )
            }
        };
        match style.shape {
            PointShape::Circle => return self.draw_circle((cx, cy), radius, style),
            PointShape::Square => {
                let a = poly(&[
                    (cx - r, cy - r),
                    (cx + r, cy - r),
                    (cx + r, cy + r),
                    (cx - r, cy + r),
                ]);
                self.push_mark("polygon", &a);
            }
            PointShape::Triangle => {
                let a = poly(&[(cx, cy - r), (cx + r, cy + r), (cx - r, cy + r)]);
                self.push_mark("polygon", &a);
            }
            PointShape::Diamond => {
                let a = poly(&[(cx, cy - r), (cx + r, cy), (cx, cy + r), (cx - r, cy)]);
                self.push_mark("polygon", &a);
            }
            PointShape::Plus | PointShape::Cross => {
                let d = if style.shape == PointShape::Plus {
                    format!(
                        "M{:.2} {cy:.2}H{:.2}M{cx:.2} {:.2}V{:.2}",
                        cx - r,
                        cx + r,
                        cy - r,
                        cy + r
                    )
                } else {
                    format!(
                        "M{:.2} {:.2}L{:.2} {:.2}M{:.2} {:.2}L{:.2} {:.2}",
                        cx - r,
                        cy - r,
                        cx + r,
                        cy + r,
                        cx - r,
                        cy + r,
                        cx + r,
                        cy - r
                    )
                };
                let a = format!(
                    "d=\"{d}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"{:.2}\" \
                     stroke-opacity=\"{alpha:.3}\"",
                    (r / 2.5).clamp(1.0, 3.0)
                );
                self.push_mark("path", &a);
            }
        }
        Ok(())
    }

    fn draw_line(&mut self, pts: &[(f64, f64)], style: &LineStyle) -> Result<(), RenderError> {
        let pts = finite_pts(pts);
        if pts.len() < 2 {
            return Ok(());
        }
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
        // `Butt` is SVG's own default: omitting the attribute keeps default
        // output byte-identical.
        let linecap = match style.lineend {
            LineEnd::Butt => "",
            LineEnd::Round => " stroke-linecap=\"round\"",
            LineEnd::Square => " stroke-linecap=\"square\"",
        };
        let attrs = format!(
            "points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\" stroke-opacity=\"{:.3}\"{}{}",
            points(&pts),
            rgb(style.color),
            finite_or(style.width, 1.0),
            finite_or(style.alpha, 1.0),
            dash,
            linecap
        );
        self.push_mark("polyline", &attrs);
        Ok(())
    }

    /// Emit a native `<path d="M.. Q..">` quadratic Bézier (one real curve
    /// instead of a flattened polyline). Non-finite input draws nothing.
    fn draw_quadratic(
        &mut self,
        p0: (f64, f64),
        ctrl: (f64, f64),
        p1: (f64, f64),
        style: &LineStyle,
    ) -> Result<(), RenderError> {
        if !(p0.0.is_finite()
            && p0.1.is_finite()
            && ctrl.0.is_finite()
            && ctrl.1.is_finite()
            && p1.0.is_finite()
            && p1.1.is_finite())
        {
            return Ok(());
        }
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
        let linecap = match style.lineend {
            LineEnd::Butt => "",
            LineEnd::Round => " stroke-linecap=\"round\"",
            LineEnd::Square => " stroke-linecap=\"square\"",
        };
        let d = format!(
            "M{:.2} {:.2} Q{:.2} {:.2} {:.2} {:.2}",
            p0.0, p0.1, ctrl.0, ctrl.1, p1.0, p1.1
        );
        let attrs = format!(
            "d=\"{d}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\" stroke-opacity=\"{:.3}\"{}{}",
            rgb(style.color),
            finite_or(style.width, 1.0),
            finite_or(style.alpha, 1.0),
            dash,
            linecap
        );
        self.push_mark("path", &attrs);
        Ok(())
    }

    fn draw_rect(
        &mut self,
        (x0, y0): (f64, f64),
        (x1, y1): (f64, f64),
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        if !(x0.is_finite() && y0.is_finite() && x1.is_finite() && y1.is_finite()) {
            return Ok(());
        }
        let (x, y) = (x0.min(x1), y0.min(y1));
        let (w, h) = ((x1 - x0).abs(), (y1 - y0).abs());
        let fill = style.fill.map(rgb).unwrap_or_else(|| "none".into());
        let stroke = style.stroke.map(rgb).unwrap_or_else(|| "none".into());
        let attrs = format!(
            "x=\"{x:.2}\" y=\"{y:.2}\" width=\"{w:.2}\" height=\"{h:.2}\" fill=\"{fill}\" \
             fill-opacity=\"{:.3}\" stroke=\"{stroke}\" stroke-width=\"{:.2}\"",
            finite_or(style.alpha, 1.0),
            finite_or(style.stroke_width, 0.0)
        );
        self.push_mark("rect", &attrs);
        Ok(())
    }

    fn draw_polygon(&mut self, pts: &[(f64, f64)], style: &RectStyle) -> Result<(), RenderError> {
        let pts = finite_pts(pts);
        if pts.len() < 3 {
            return Ok(());
        }
        let fill = style.fill.map(rgb).unwrap_or_else(|| "none".into());
        let stroke = style.stroke.map(rgb).unwrap_or_else(|| "none".into());
        let attrs = format!(
            "points=\"{}\" fill=\"{fill}\" fill-opacity=\"{:.3}\" stroke=\"{stroke}\" stroke-width=\"{:.2}\"",
            points(&pts),
            finite_or(style.alpha, 1.0),
            finite_or(style.stroke_width, 0.0)
        );
        self.push_mark("polygon", &attrs);
        Ok(())
    }

    fn draw_text(
        &mut self,
        text: &str,
        (x, y): (f64, f64),
        style: &TextStyle,
    ) -> Result<(), RenderError> {
        if !(x.is_finite() && y.is_finite()) {
            return Ok(());
        }
        let anchor = match style.anchor {
            TextAnchor::Start => "start",
            TextAnchor::Middle => "middle",
            TextAnchor::End => "end",
        };
        let family = escape(style.family.as_deref().unwrap_or("sans-serif"));
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
        let angle = finite_or(style.angle, 0.0);
        let transform = if angle.abs() > 0.01 {
            format!(" transform=\"rotate({angle:.1} {x:.2} {y:.2})\"")
        } else {
            String::new()
        };
        self.body.push_str(&format!(
            "<text x=\"{x:.2}\" y=\"{y:.2}\" font-size=\"{:.2}\" text-anchor=\"{anchor}\" \
             dominant-baseline=\"middle\" font-family=\"{family}\"{weight}{fstyle} fill=\"{}\"{transform}>{}</text>",
            finite_or(style.size, 12.0),
            rgb(style.color),
            escape(text)
        ));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::Rect;

    fn backend() -> SvgBackend {
        SvgBackend::new(
            100,
            100,
            Rect {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 100.0,
            },
        )
    }

    #[test]
    fn draw_quadratic_emits_a_path_with_m_and_q() {
        let mut b = backend();
        b.draw_quadratic((0.0, 0.0), (10.0, 20.0), (20.0, 0.0), &LineStyle::default())
            .unwrap();
        let svg = b.finish();
        assert!(svg.contains("<path"));
        assert!(svg.contains("d=\"M0.00 0.00 Q10.00 20.00 20.00 0.00\""));
        assert!(!svg.contains("NaN"));
        assert!(!svg.contains("inf"));
    }

    #[test]
    fn draw_quadratic_drops_non_finite_input() {
        let mut b = backend();
        b.draw_quadratic(
            (f64::NAN, 0.0),
            (10.0, 20.0),
            (20.0, 0.0),
            &LineStyle::default(),
        )
        .unwrap();
        b.draw_quadratic(
            (0.0, 0.0),
            (f64::INFINITY, 20.0),
            (20.0, 0.0),
            &LineStyle::default(),
        )
        .unwrap();
        let svg = b.finish();
        assert!(!svg.contains("<path"));
    }

    #[test]
    fn draw_line_omits_linecap_by_default_but_writes_it_when_set() {
        let mut b = backend();
        b.draw_line(&[(0.0, 0.0), (10.0, 10.0)], &LineStyle::default())
            .unwrap();
        let svg = b.finish();
        assert!(!svg.contains("stroke-linecap"));

        let mut b2 = backend();
        let style = LineStyle {
            lineend: LineEnd::Round,
            ..Default::default()
        };
        b2.draw_line(&[(0.0, 0.0), (10.0, 10.0)], &style).unwrap();
        let svg2 = b2.finish();
        assert!(svg2.contains("stroke-linecap=\"round\""));
    }

    #[test]
    fn square_linecap_is_written_on_both_draw_line_and_draw_quadratic() {
        let style = LineStyle {
            lineend: LineEnd::Square,
            ..Default::default()
        };
        let mut b = backend();
        b.draw_line(&[(0.0, 0.0), (10.0, 10.0)], &style).unwrap();
        assert!(b.finish().contains("stroke-linecap=\"square\""));

        let mut b2 = backend();
        b2.draw_quadratic((0.0, 0.0), (5.0, 10.0), (10.0, 0.0), &style)
            .unwrap();
        assert!(b2.finish().contains("stroke-linecap=\"square\""));
    }

    /// `push_mark`'s defence-in-depth gate: even if an invalid name reaches
    /// `set_mark_attrs` directly (bypassing `Aes::data_attr`'s own
    /// validation — `set_mark_attrs` is a public `DrawBackend` method any
    /// caller could invoke), it's dropped before it reaches the wire.
    #[test]
    fn set_mark_attrs_defence_in_depth_drops_invalid_names() {
        let mut b = backend();
        b.set_mark_attrs(vec![
            ("not-data-prefixed".to_string(), "x".to_string()),
            ("data-".to_string(), "x".to_string()), // empty suffix
            ("data-Upper".to_string(), "x".to_string()), // uppercase
            ("data-ok".to_string(), "valid".to_string()), // the one legitimate entry
        ]);
        b.draw_circle((1.0, 1.0), 1.0, &PointStyle::default())
            .unwrap();
        let svg = b.finish();
        assert!(!svg.contains("not-data-prefixed"));
        assert!(!svg.contains("data-Upper"));
        assert!(!svg.contains("data-=\""));
        assert!(svg.contains("data-ok=\"valid\""));
    }
}
