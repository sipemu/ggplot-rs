//! A self-contained SVG `DrawBackend` — a *second* backend (no plotters),
//! proving the `DrawBackend` abstraction: the same `PlotRenderer` drives it.
//!
//! It emits SVG elements directly, so SVG output needs no glyph rasterization
//! (text is `<text>` with font attributes the viewer renders).

use super::backend::{
    DrawBackend, FillPattern, FontFace, LineEnd, LineStyle, Linetype, PointShape, PointStyle,
    RectStyle, TextAnchor, TextStyle,
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
    /// Geom of the layer being drawn (see [`DrawBackend::set_mark_geom`]).
    mark_geom: Option<String>,
    root_attrs: Vec<(String, String)>,
    warnings: Vec<String>,
    /// Emit the root `data-plot` panel rect (off for composite pages, whose
    /// root is not a single panel).
    plot_attr: bool,
    /// B2-4: whether `GGPlot::svg_formatted_values` is on — see
    /// [`DrawBackend::wants_formatted_values`].
    formatted_values: bool,
    /// `data-fvalue` for the next drawn mark(s) — see
    /// [`DrawBackend::set_mark_fvalue`].
    mark_fvalue: Option<String>,
    /// `data-fx` for the next drawn mark(s) — see
    /// [`DrawBackend::set_mark_fx`].
    mark_fx: Option<String>,
    /// Patterns registered so far via [`Self::pattern_url`] (#206),
    /// deduped by equality: `(pattern, "<id_prefix>-pat-<n>")`. Drained
    /// into a `<defs>` block by [`Self::pattern_defs`] on `finish`/
    /// `finish_fragment`.
    patterns: Vec<(FillPattern, String)>,
    /// This backend instance's id prefix for `<defs>` — process-unique
    /// (a monotonic counter), so inlining several rendered SVGs into one
    /// HTML page never lets one's `url(#…)` resolve into another's
    /// `<pattern>` (SVG ids share one DOM-wide namespace once inlined).
    id_prefix: String,
    /// #221: `Aes::class` tokens for the next drawn mark(s) — see
    /// [`DrawBackend::set_mark_class`].
    mark_user_class: Option<String>,
    /// #221: the caller-named group `(attr, value)` the next drawn mark(s)
    /// belong to — see [`DrawBackend::set_mark_group`].
    mark_group: Option<(String, String)>,
    /// #221: the current layer's `GGPlot::layer_svg_attr`s — see
    /// [`DrawBackend::set_layer_attrs`].
    layer_attrs: Vec<(String, String)>,
    /// #221: each group's `(attr, value, accumulated markup)`, in order of
    /// first appearance.
    groups: Vec<(String, String, String)>,
    /// #221: `(byte offset into body, index into groups)` — where each
    /// group's `<g>` is spliced in by [`Self::flush_groups`]. Offsets are
    /// non-decreasing (a group's slot is the body length when its first
    /// mark arrives, and the body only ever grows at the end).
    group_slots: Vec<(usize, usize)>,
    /// #255: accessible name text ([`GGPlot::svg_title`](crate::plot::GGPlot::svg_title)),
    /// written as a `<title id="{id_prefix}-title">` first child of the
    /// root `<svg>` plus `role="img"`/`aria-labelledby`. Default `None`;
    /// byte-identical output when not set.
    svg_title: Option<String>,
    /// #255: accessible description text ([`GGPlot::svg_desc`](crate::plot::GGPlot::svg_desc)),
    /// written as a `<desc id="{id_prefix}-desc">` first child of the root
    /// `<svg>` (after `<title>`, if any) plus `role="img"`/`aria-describedby`.
    /// Default `None`; byte-identical output when not set.
    svg_desc: Option<String>,
    /// #273: panel-corner boxes claimed so far this render pass — see
    /// [`DrawBackend::corner_claims`]/[`DrawBackend::claim_corner_box`].
    corner_claims: Vec<Rect>,
    /// The single-panel data rect ([`DrawBackend::set_viewport`]), written
    /// as the root `data-viewport` (with `data-plot`).
    viewport: Option<Rect>,
    /// Fixed-aspect facet panels ([`DrawBackend::begin_panel`]): `(index,
    /// label, clip rect, data rect)`. Each clip becomes a `<clipPath>` in
    /// `<defs>`; the list is written as the root `data-panels`.
    panels: Vec<(usize, String, Rect, Rect)>,
}

/// Source for [`SvgBackend::id_prefix`] — process-wide, so every backend
/// instance gets a distinct prefix regardless of thread.
static NEXT_ID_PREFIX: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

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
            mark_geom: None,
            root_attrs: Vec::new(),
            warnings: Vec::new(),
            plot_attr: true,
            formatted_values: false,
            mark_fvalue: None,
            mark_fx: None,
            patterns: Vec::new(),
            id_prefix: format!(
                "gg{:x}",
                NEXT_ID_PREFIX.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ),
            mark_user_class: None,
            mark_group: None,
            layer_attrs: Vec::new(),
            groups: Vec::new(),
            group_slots: Vec::new(),
            svg_title: None,
            svg_desc: None,
            corner_claims: Vec::new(),
            viewport: None,
            panels: Vec::new(),
        }
    }

    /// The `url(#…)` fill value for `pattern`, registering its `<defs>
    /// <pattern>` the first time this exact pattern is seen on this
    /// backend (later calls with an equal pattern reuse the same id).
    fn pattern_url(&mut self, pattern: &FillPattern) -> String {
        if let Some((_, id)) = self.patterns.iter().find(|(p, _)| p == pattern) {
            return format!("url(#{id})");
        }
        let id = format!("{}-pat-{}", self.id_prefix, self.patterns.len());
        self.patterns.push((*pattern, id.clone()));
        format!("url(#{id})")
    }

    /// Shared by [`DrawBackend::draw_rect`]/[`DrawBackend::draw_rect_pattern`]
    /// — `fill` is already resolved (a hex colour or a pattern's `url(#…)`).
    fn rect_markup(
        &mut self,
        (x0, y0): (f64, f64),
        (x1, y1): (f64, f64),
        fill: &str,
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        if !(x0.is_finite() && y0.is_finite() && x1.is_finite() && y1.is_finite()) {
            return Ok(());
        }
        let (x, y) = (x0.min(x1), y0.min(y1));
        let (w, h) = ((x1 - x0).abs(), (y1 - y0).abs());
        let stroke = style.stroke.map(rgb).unwrap_or_else(|| "none".into());
        // SVG `<rect>` takes rx/ry natively — clamp to half the shorter side
        // so an oversized radius draws a "stadium" instead of a malformed box.
        let r = style.corner_radius.max(0.0).min(w / 2.0).min(h / 2.0);
        let radius = if r > 0.0 {
            format!(" rx=\"{r:.2}\" ry=\"{r:.2}\"")
        } else {
            String::new()
        };
        let attrs = format!(
            "x=\"{x:.2}\" y=\"{y:.2}\" width=\"{w:.2}\" height=\"{h:.2}\"{radius} fill=\"{fill}\"{} \
             stroke=\"{stroke}\" stroke-width=\"{:.2}\"",
            opacity_attr("fill", style.alpha),
            finite_or(style.stroke_width, 0.0)
        );
        self.push_mark("rect", "bar", &attrs);
        Ok(())
    }

    /// Shared by [`DrawBackend::draw_polygon`]/[`DrawBackend::draw_polygon_pattern`].
    fn polygon_markup(
        &mut self,
        pts: &[(f64, f64)],
        fill: &str,
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        let pts = finite_pts(pts);
        if pts.len() < 3 {
            return Ok(());
        }
        let stroke = style.stroke.map(rgb).unwrap_or_else(|| "none".into());
        let attrs = format!(
            "points=\"{}\" fill=\"{fill}\"{} stroke=\"{stroke}\" stroke-width=\"{:.2}\"",
            points(&pts),
            opacity_attr("fill", style.alpha),
            finite_or(style.stroke_width, 0.0)
        );
        self.push_mark("polygon", "band", &attrs);
        Ok(())
    }

    /// Shared by [`DrawBackend::draw_polygon_rings`]/
    /// [`DrawBackend::draw_polygon_rings_pattern`].
    fn polygon_rings_markup(
        &mut self,
        rings: &[Vec<(f64, f64)>],
        fill: &str,
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        let mut d = String::new();
        for ring in rings {
            let pts = finite_pts(ring);
            if pts.len() < 3 {
                continue;
            }
            d.push('M');
            d.push_str(&format!("{},{}", num(pts[0].0), num(pts[0].1)));
            for p in &pts[1..] {
                d.push('L');
                d.push_str(&format!("{},{}", num(p.0), num(p.1)));
            }
            d.push('Z');
        }
        if d.is_empty() {
            return Ok(());
        }
        let stroke = style.stroke.map(rgb).unwrap_or_else(|| "none".into());
        let attrs = format!(
            "d=\"{d}\" fill-rule=\"evenodd\" fill=\"{fill}\"{} stroke=\"{stroke}\" stroke-width=\"{:.2}\"",
            opacity_attr("fill", style.alpha),
            finite_or(style.stroke_width, 0.0)
        );
        self.push_mark("path", "band", &attrs);
        Ok(())
    }

    /// The `<defs>` block for every pattern [`Self::pattern_url`]
    /// registered (empty when none were) — a `<pattern>` tile per the
    /// stripe's own cycle, rotated by `patternTransform`, a `bg`-filled
    /// tile with a narrower `fg` stripe at its edge (#206).
    fn pattern_defs(&self) -> String {
        if self.patterns.is_empty() && self.panels.is_empty() {
            return String::new();
        }
        let mut defs = String::from("<defs>");
        for (i, _, clip, _) in &self.panels {
            defs.push_str(&format!(
                "<clipPath id=\"{}-panel-{i}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/></clipPath>",
                self.id_prefix,
                num(clip.x),
                num(clip.y),
                num(clip.width),
                num(clip.height),
            ));
        }
        for (p, id) in &self.patterns {
            let FillPattern::Stripe {
                fg,
                bg,
                size_px,
                angle_deg,
            } = p;
            let size = if *size_px > 0.0 { *size_px } else { 1.0 };
            defs.push_str(&format!(
                "<pattern id=\"{id}\" width=\"{size}\" height=\"{size}\" patternUnits=\"userSpaceOnUse\" patternTransform=\"rotate({angle_deg})\"><rect width=\"{size}\" height=\"{size}\" fill=\"{}\"/><rect width=\"{:.3}\" height=\"{size}\" fill=\"{}\"/></pattern>",
                rgb(*bg),
                size * 0.3,
                rgb(*fg),
            ));
        }
        defs.push_str("</defs>");
        defs
    }

    /// Turn `data-fvalue`/`data-fx` on for the whole render
    /// ([`GGPlot::svg_formatted_values`](crate::plot::GGPlot::svg_formatted_values)).
    /// Default off (set once, before drawing).
    pub(crate) fn set_formatted_values(&mut self, on: bool) {
        self.formatted_values = on;
    }

    /// Add extra attributes to the root `<svg>` element (e.g. the
    /// `data-domain` family from [`root_data_attrs`]). Names must be plain
    /// attribute names; values are escaped on output.
    pub fn set_root_attrs(&mut self, attrs: Vec<(String, String)>) {
        self.root_attrs = attrs;
    }

    /// Append extra attributes to the root `<svg>` element, after any
    /// already set by [`set_root_attrs`](Self::set_root_attrs) (e.g. the
    /// `data-domain` family from [`root_data_attrs`]). Unlike
    /// `set_root_attrs`, this does not replace what is already there.
    /// Callers (`GGPlot::svg_attr`/`PlotGrid::svg_attr`, via
    /// [`validate_root_attrs`]) are responsible for name validation and
    /// collision checks; this just appends. Values are escaped on output.
    pub fn extend_root_attrs(&mut self, attrs: Vec<(String, String)>) {
        self.root_attrs.extend(attrs);
    }

    /// Omit the root `data-plot` attribute (composite pages).
    pub(crate) fn without_plot_attr(&mut self) {
        self.plot_attr = false;
    }

    /// #255: set the root `<svg>`'s accessible name text
    /// ([`GGPlot::svg_title`](crate::plot::GGPlot::svg_title)). `None`
    /// (the default) omits the `<title>` child and the `role`/
    /// `aria-labelledby` attributes entirely.
    pub(crate) fn set_svg_title(&mut self, title: Option<String>) {
        self.svg_title = title;
    }

    /// #255: set the root `<svg>`'s accessible description text
    /// ([`GGPlot::svg_desc`](crate::plot::GGPlot::svg_desc)). `None` (the
    /// default) omits the `<desc>` child and the `role`/
    /// `aria-describedby` attributes entirely.
    pub(crate) fn set_svg_desc(&mut self, desc: Option<String>) {
        self.svg_desc = desc;
    }

    /// Append pre-rendered, already-escaped SVG markup (e.g. a nested plot
    /// fragment) to the body. Crate-internal: callers guarantee well-formedness.
    pub(crate) fn push_raw(&mut self, markup: &str) {
        self.body.push_str(markup);
    }

    /// The accumulated body markup (without the root element), with every
    /// pending `group_wrap` group (#221) spliced in.
    pub(crate) fn body(&mut self) -> &str {
        self.flush_groups();
        &self.body
    }

    /// Append one drawn mark: into its `group_wrap` group's buffer while a
    /// layer's mark has one (#221), else straight onto the body.
    fn emit(&mut self, markup: &str) {
        let key = match (&self.mark_group, &self.mark_geom) {
            (Some(k), Some(_)) => k,
            _ => {
                self.body.push_str(markup);
                return;
            }
        };
        match self
            .groups
            .iter_mut()
            .find(|(n, v, _)| n == &key.0 && v == &key.1)
        {
            Some((_, _, content)) => content.push_str(markup),
            None => {
                self.group_slots.push((self.body.len(), self.groups.len()));
                self.groups
                    .push((key.0.clone(), key.1.clone(), markup.to_string()));
            }
        }
    }

    /// Splice every pending group (#221) into the body as
    /// `<g data-<name>="<value>">…</g>` at its first mark's position. The
    /// attribute name was validated by `Aes::group_wrap`/`resolve_mappings`
    /// and is re-checked here (defence in depth, as for `mark_attrs`); a
    /// group with an invalid name is spliced in unwrapped.
    fn flush_groups(&mut self) {
        if self.group_slots.is_empty() {
            return;
        }
        let groups = std::mem::take(&mut self.groups);
        let slots = std::mem::take(&mut self.group_slots);
        let extra: usize = groups
            .iter()
            .map(|(n, v, c)| n.len() + v.len() + c.len() + 12)
            .sum();
        let mut out = String::with_capacity(self.body.len() + extra);
        let mut last = 0;
        for (off, gi) in slots {
            out.push_str(&self.body[last..off]);
            last = off;
            let (name, value, content) = &groups[gi];
            if is_valid_mark_attr_name(name) {
                out.push_str(&format!(
                    "<g {name}=\"{}\">{content}</g>",
                    escape_attr(value)
                ));
            } else {
                out.push_str(content);
            }
        }
        out.push_str(&self.body[last..]);
        self.body = out;
    }

    /// Wrap the accumulated body in a `<g>` carrying `attrs` (escaped) — a
    /// `PlotGrid` cell's own `svg_attr`s (#208), which belong on that
    /// cell's content, not the cell's own root `<svg>` (ggplot-rs's own
    /// `root_data_attrs`/`data-panel`) or the grid's root (the grid's own
    /// `svg_attr`s). A no-op for an empty `attrs` — unaffected output,
    /// same as every other panel without a `svg_attr` call.
    pub(crate) fn wrap_body_in_group(&mut self, attrs: &[(String, String)]) {
        self.flush_groups();
        if attrs.is_empty() {
            return;
        }
        let mut open = String::from("<g");
        for (k, v) in attrs {
            open.push_str(&format!(" {k}=\"{}\"", escape_attr(v)));
        }
        open.push('>');
        self.body = format!("{open}{}</g>", self.body);
    }

    /// Take the warnings reported while drawing (see [`DrawBackend::warn`]).
    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }

    /// ` class="gg-mark gg-<kind>" data-geom="<geom>"` while a layer's geom is
    /// drawing; empty for axes, grid lines, legends and backgrounds.
    ///
    /// #221: the layer's `layer_svg_attr("class", …)` tokens, then the
    /// row's `Aes::class` tokens, are appended (each re-validated, defence
    /// in depth; duplicates written once) — still one `class` attribute.
    fn mark_class(&self, kind: &str) -> String {
        match &self.mark_geom {
            Some(g) => {
                let mut class = format!("gg-mark gg-{kind}");
                let layer = self
                    .layer_attrs
                    .iter()
                    .filter(|(n, _)| n == "class")
                    .map(|(_, v)| v.as_str());
                let mut seen: Vec<&str> = Vec::new();
                for t in layer
                    .chain(self.mark_user_class.as_deref())
                    .flat_map(str::split_whitespace)
                {
                    if crate::aes::is_valid_class_token(t) && !seen.contains(&t) {
                        seen.push(t);
                        class.push(' ');
                        class.push_str(t);
                    }
                }
                format!(" class=\"{class}\" data-geom=\"{}\"", escape_attr(g))
            }
            None => String::new(),
        }
    }

    /// #221: ` aria-hidden="…" pointer-events="…"` from the current layer's
    /// `layer_svg_attr`s (only while a layer draws). `aria_hidden_written`
    /// skips the layer's `aria-hidden` when the mark already carries one
    /// (a mark part — arrowhead/decorative — always writes `"true"`).
    fn layer_flag_attrs(&self, aria_hidden_written: bool) -> String {
        let mut out = String::new();
        if self.mark_geom.is_none() {
            return out;
        }
        for (name, value) in &self.layer_attrs {
            let ok = match name.as_str() {
                "aria-hidden" => !aria_hidden_written && matches!(value.as_str(), "true" | "false"),
                "pointer-events" => POINTER_EVENTS_VALUES.contains(&value.as_str()),
                _ => false,
            };
            if ok {
                out.push_str(&format!(" {name}=\"{value}\""));
            }
        }
        out
    }

    /// #221: the `data-*` attributes a layer's text mark carries — the
    /// layer's `layer_svg_attr("data-…")`s, then the row's `Aes::data_attr`
    /// ones (a row attr replaces a layer attr of the same name). Text marks
    /// don't go through [`Self::push_mark`] (no `<title>`, no built-ins).
    fn text_data_attrs(&self) -> String {
        let mut pairs: Vec<(&str, &str)> = Vec::new();
        if self.mark_geom.is_none() {
            return String::new();
        }
        for (name, val) in self.layer_attrs.iter().chain(self.mark_attrs.iter()) {
            if !is_valid_mark_attr_name(name) {
                continue;
            }
            match pairs.iter_mut().find(|(n, _)| n == name) {
                Some(existing) => existing.1 = val,
                None => pairs.push((name, val)),
            }
        }
        let mut out = String::new();
        for (name, val) in pairs {
            out.push_str(&format!(" {name}=\"{}\"", escape_attr(val)));
        }
        out
    }

    /// Emit `<tag attrs/>`, or `<tag attrs data-…><title>tip</title></tag>`
    /// when a tooltip / mark metadata is set (native SVG hover + host grouping).
    ///
    /// `kind` is the data-mark class suffix (see [`Self::mark_class`]) used
    /// when no mark part is set. While a mark part *is* set (e.g. an
    /// arrowhead), the part name is the effective kind instead — an
    /// arrowhead gets `gg-mark gg-arrow`, not its parent's `gg-band`/`gg-line`.
    fn push_mark(&mut self, tag: &str, kind: &str, attrs: &str) {
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
        // B2-4: `data-fx` only when it says something `data-x` doesn't —
        // most marks have a plain numeric x, where the "formatted" value is
        // the same string, and a redundant duplicate attribute is noise.
        if let Some(fx) = &self.mark_fx {
            if self.axis_key.as_deref() != Some(fx.as_str()) {
                pairs.push(("data-fx".to_string(), fx.clone()));
            }
        }
        if let Some(fvalue) = &self.mark_fvalue {
            pairs.push(("data-fvalue".to_string(), fvalue.clone()));
        }
        // #221: the layer's own `data-*` (`layer_svg_attr`) first, so a
        // row's `data_attr` of the same name replaces it.
        let layer_data = self.layer_attrs.iter().filter(|_| self.mark_geom.is_some());
        for (name, val) in layer_data.chain(self.mark_attrs.iter()) {
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
            data.push_str(&format!(" {name}=\"{}\"", escape_attr(val)));
        }
        if self.mark_part.is_some() {
            data.push_str(" aria-hidden=\"true\"");
        }
        data.push_str(&self.layer_flag_attrs(self.mark_part.is_some()));
        // The class goes last so existing attribute sequences stay
        // byte-identical. Exactly one `class` attribute is ever written here
        // (no `data_attr` mapping can be named bare `class` — see
        // `is_valid_mark_attr_name` — so nothing in `pairs` can collide with it).
        let effective_kind: &str = self.mark_part.as_deref().unwrap_or(kind);
        data.push_str(&self.mark_class(effective_kind));
        let mark = if self.mark_part.is_some() {
            format!("<{tag} {attrs}{data}/>")
        } else {
            match &self.tooltip {
                Some(t) => format!("<{tag} {attrs}{data}><title>{}</title></{tag}>", escape(t)),
                None => format!("<{tag} {attrs}{data}/>"),
            }
        };
        self.emit(&mark);
    }

    fn root_open(&self, prefix: &str) -> String {
        let p = &self.plot_area;
        let (w, h) = (self.total_area.width as i64, self.total_area.height as i64);
        let mut extra = String::new();
        for (k, v) in &self.root_attrs {
            extra.push_str(&format!(" {k}=\"{}\"", escape_attr(v)));
        }
        let mut plot = if self.plot_attr {
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
        if self.plot_attr {
            if let Some(v) = &self.viewport {
                plot.push_str(&format!(
                    " data-viewport=\"{} {} {} {}\"",
                    num(v.x),
                    num(v.y),
                    num(v.width),
                    num(v.height),
                ));
            }
            if !self.panels.is_empty() {
                let items: Vec<String> = self
                    .panels
                    .iter()
                    .map(|(i, label, _, d)| {
                        format!(
                            "{{\"i\":{i},\"label\":{},\"x\":{},\"y\":{},\"w\":{},\"h\":{}}}",
                            json_string(label),
                            num(d.x),
                            num(d.y),
                            num(d.width),
                            num(d.height),
                        )
                    })
                    .collect();
                plot.push_str(&format!(
                    " data-panels=\"{}\"",
                    escape_attr(&format!("[{}]", items.join(",")))
                ));
            }
        }
        // #255: `role="img"` plus `aria-labelledby`/`aria-describedby`
        // pointing at the `<title>`/`<desc>` children `access_children`
        // writes as the root's first children — only when a title/desc
        // was actually set, so default output stays byte-identical.
        let mut access = String::new();
        if self.svg_title.is_some() || self.svg_desc.is_some() {
            access.push_str(" role=\"img\"");
        }
        if self.svg_title.is_some() {
            access.push_str(&format!(" aria-labelledby=\"{}-title\"", self.id_prefix));
        }
        if self.svg_desc.is_some() {
            access.push_str(&format!(" aria-describedby=\"{}-desc\"", self.id_prefix));
        }
        format!(
            "<svg {prefix}width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\"{plot}{access}{extra}>"
        )
    }

    /// #255: `<title>`/`<desc>` markup for the root `<svg>`'s first
    /// children (SVG accessibility practice puts them before any other
    /// content), ids matching [`root_open`]'s `aria-labelledby`/
    /// `aria-describedby`. Empty when neither was set.
    fn access_children(&self) -> String {
        let mut out = String::new();
        if let Some(t) = &self.svg_title {
            out.push_str(&format!(
                "<title id=\"{}-title\">{}</title>",
                self.id_prefix,
                escape(t)
            ));
        }
        if let Some(d) = &self.svg_desc {
            out.push_str(&format!(
                "<desc id=\"{}-desc\">{}</desc>",
                self.id_prefix,
                escape(d)
            ));
        }
        out
    }

    /// Wrap the accumulated elements in a complete `<svg>` document. The
    /// `data-plot` attribute records the panel (data) area in viewBox units so a
    /// host can map screen coordinates back to data (e.g. for zoom/crosshair).
    pub fn finish(mut self) -> String {
        self.flush_groups();
        let open = self.root_open("xmlns=\"http://www.w3.org/2000/svg\" ");
        let access = self.access_children();
        let defs = self.pattern_defs();
        format!("{open}{access}{defs}{}</svg>", self.body)
    }

    /// Wrap the accumulated elements in a *nested* `<svg>` positioned at
    /// `(x, y)` in a parent SVG's user space — no `xmlns` (it inherits the
    /// parent's namespace), so the result can be pasted into a larger SVG as-is.
    /// `data-plot` stays in this fragment's own viewBox units.
    pub fn finish_fragment(mut self, x: f64, y: f64) -> String {
        self.flush_groups();
        let open = self.root_open(&format!("x=\"{}\" y=\"{}\" ", num(x), num(y)));
        let access = self.access_children();
        let defs = self.pattern_defs();
        format!("{open}{access}{defs}{}</svg>", self.body)
    }

    /// Emit one `<polyline>` for an already-finite run of points — see
    /// [`DrawBackend::draw_line`]'s splitting at non-finite vertices.
    fn draw_line_run(&mut self, pts: &[(f64, f64)], style: &LineStyle) {
        let dash = dasharray_attr(&style.linetype);
        // `Butt` is SVG's own default: omitting the attribute keeps default
        // output byte-identical.
        let linecap = match style.lineend {
            LineEnd::Butt => "",
            LineEnd::Round => " stroke-linecap=\"round\"",
            LineEnd::Square => " stroke-linecap=\"square\"",
        };
        let attrs = format!(
            "points=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\"{}{}{}",
            points(pts),
            rgb(style.color),
            finite_or(style.width, 1.0),
            opacity_attr("stroke", style.alpha),
            dash,
            linecap
        );
        self.push_mark("polyline", "line", &attrs);
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

/// ` stroke-dasharray="d,g,…"` for a linetype, or `""` for solid. Uses the
/// sanitised [`Linetype::dash_pattern`], so a `Linetype::Custom` with NaN or
/// negative lengths never reaches the attribute (#220).
fn dasharray_attr(linetype: &Linetype) -> String {
    let pattern = linetype.dash_pattern();
    if pattern.is_empty() {
        return String::new();
    }
    let s = pattern
        .iter()
        .flat_map(|(d, g)| [*d, *g])
        .map(|v| format!("{v}"))
        .collect::<Vec<_>>()
        .join(",");
    format!(" stroke-dasharray=\"{s}\"")
}

/// #214: `d` for [`SvgBackend::draw_polygon_rings_precise`] — one `M<x>,<y>`
/// per ring (the first point, absolute) followed by one `l` command whose
/// coordinate pairs are **deltas from the previous point**, SVG's implicit
/// lineto-repeat (`l dx1,dy1 dx2,dy2 …`), each number rounded to
/// `precision` decimals with trailing zeros trimmed. Deltas are typically
/// far smaller than absolute screen coordinates, and the single `l` letter
/// (vs. one `L` per point) is the other half of the size win; see
/// [`push_num`] for the separator-omission this relies on. A ring under 3
/// finite points is dropped, same as the absolute encoder.
fn compact_rings_d(rings: &[Vec<(f64, f64)>], precision: u8) -> String {
    let mut d = String::new();
    for ring in rings {
        let pts = finite_pts(ring);
        if pts.len() < 3 {
            continue;
        }
        d.push('M');
        push_num(&mut d, &fmt_compact(pts[0].0, precision));
        push_num(&mut d, &fmt_compact(pts[0].1, precision));
        if pts.len() > 1 {
            d.push('l');
            let mut prev = pts[0];
            for p in &pts[1..] {
                push_num(&mut d, &fmt_compact(p.0 - prev.0, precision));
                push_num(&mut d, &fmt_compact(p.1 - prev.1, precision));
                prev = *p;
            }
        }
        d.push('z');
    }
    d
}

/// Append `s` (a formatted number) to `d`, inserting a comma first only
/// when one is actually needed to keep the path parseable: right after a
/// digit, with the new number *not* itself starting with `-` (whose sign
/// already disambiguates where the previous number ended, SVG's own
/// "no separator needed before a negative number" rule — the "no
/// redundant separators" #214 asks for). Right after a command letter
/// (`M`/`l`/`z`) no separator is ever needed.
fn push_num(d: &mut String, s: &str) {
    let needs_sep =
        matches!(d.chars().last(), Some(c) if c.is_ascii_digit()) && !s.starts_with('-');
    if needs_sep {
        d.push(',');
    }
    d.push_str(s);
}

/// Format `v` to `precision` decimals, then trim trailing zeros and a
/// trailing `.` (`"5.0"` → `"5"`, `"1.20"` → `"1.2"`) — the SVG path
/// grammar allows a bare integer anywhere a number is valid. Non-finite
/// becomes `"0"`; `-0`-after-rounding (e.g. a tiny negative delta that
/// rounds to zero) becomes `"0"` too, not `"-0"`.
fn fmt_compact(v: f64, precision: u8) -> String {
    if !v.is_finite() {
        return "0".to_string();
    }
    let s = format!("{v:.*}", precision as usize);
    let trimmed = if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.')
    } else {
        s.as_str()
    };
    match trimmed {
        "" | "-" | "-0" => "0".to_string(),
        other => other.to_string(),
    }
}

fn finite_or(v: f64, fallback: f64) -> f64 {
    if v.is_finite() {
        v
    } else {
        fallback
    }
}

/// ` {prefix}-opacity="{alpha:.3}"` (with its own leading space), or the
/// empty string when `alpha` — after the usual non-finite fallback to fully
/// opaque — formats to `"1.000"`. Full opacity is the SVG default, so
/// omitting it keeps default output byte-identical; an explicit `alpha = 1`
/// is indistinguishable from "not set" and also omits the no-op attribute.
fn opacity_attr(prefix: &str, alpha: f64) -> String {
    let a = finite_or(alpha, 1.0);
    if format!("{a:.3}") == "1.000" {
        String::new()
    } else {
        format!(" {prefix}-opacity=\"{a:.3}\"")
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
/// Standard (RFC 4648) base64 with padding.
fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16
            | (*c.get(1).unwrap_or(&0) as u32) << 8
            | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= c.len() {
                out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

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
    if let Some(at) = &built.legend_at {
        out.push(("data-legend-at".into(), at.clone()));
    }
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

/// Allow-listed root-level attribute names for `GGPlot::svg_attr`/
/// `PlotGrid::svg_attr`: the fixed names `role`, `id`, `class`, `lang`, or
/// `data-*`/`aria-*` with the same charset-restricted suffix `data_attr`
/// uses (`^[a-z0-9][a-z0-9-]*$`, excluding the reserved `part`/`geom`
/// suffixes — irrelevant at the root, but no reason to special-case them
/// out). Anything else is rejected.
fn is_allowed_root_attr_name(name: &str) -> bool {
    matches!(name, "role" | "id" | "class" | "lang")
        || name
            .strip_prefix("data-")
            .is_some_and(crate::aes::is_valid_data_attr_name)
        || name
            .strip_prefix("aria-")
            .is_some_and(crate::aes::is_valid_data_attr_name)
}

/// Validate and filter a caller's requested root `<svg>` attributes
/// (`GGPlot::svg_attr`/`PlotGrid::svg_attr`), checked centrally here and
/// used by both callers rather than re-implemented per type. A name outside
/// [`is_allowed_root_attr_name`]'s allow-list, or one already present in
/// `existing` (what ggplot-rs itself is about to write to the same root —
/// e.g. `data-plot`/`data-domain`/`data-grid`, or `data-warnings` when
/// `svg_warnings_attr` is also on), is dropped with a warning instead of
/// silently overwriting or duplicating it. `label` names the caller
/// (`"plot"`/`"grid"`) in the warning text.
pub(crate) fn validate_root_attrs(
    requested: &[(String, String)],
    existing: &[(String, String)],
    label: &str,
    warnings: &mut Vec<String>,
) -> Vec<(String, String)> {
    let built_in: Vec<&str> = existing.iter().map(|(k, _)| k.as_str()).collect();
    let mut used: Vec<&str> = built_in.clone();
    let mut out = Vec::new();
    for (name, value) in requested {
        if !is_allowed_root_attr_name(name) {
            warnings.push(format!(
                "{label}: svg_attr(\"{name}\") is not on the allow-list (data-*, aria-*, role, id, class, lang) — ignored"
            ));
            continue;
        }
        if used.contains(&name.as_str()) {
            // Review PR#141 nit #9: distinguish an actual collision with
            // something ggplot-rs itself writes (`data-plot`, `data-grid`,
            // …) from a *second* `svg_attr` call for the same name — the
            // latter is the caller's own earlier value, not ggplot-rs's.
            if built_in.contains(&name.as_str()) {
                warnings.push(format!(
                    "{label}: svg_attr(\"{name}\") collides with an attribute ggplot-rs already writes to the root — ignored"
                ));
            } else {
                warnings.push(format!(
                    "{label}: svg_attr(\"{name}\") was already set by an earlier svg_attr call — ignored"
                ));
            }
            continue;
        }
        used.push(name.as_str());
        out.push((name.clone(), value.clone()));
    }
    out
}

/// `["warning one", "warning two"]` — a minimal hand-rolled JSON array of
/// strings for `svg_warnings_attr` (no serde dependency, kept out of the
/// `sf`-feature dependency tree — see [`json_string`]).
pub(crate) fn warnings_json(warnings: &[String]) -> String {
    let items: Vec<String> = warnings.iter().map(|w| json_string(w)).collect();
    format!("[{}]", items.join(","))
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

/// `pointer-events` keyword values accepted by
/// [`validate_layer_svg_attr`] (#221) — SVG 1.1's full list.
const POINTER_EVENTS_VALUES: &[&str] = &[
    "auto",
    "none",
    "visiblePainted",
    "visibleFill",
    "visibleStroke",
    "visible",
    "painted",
    "fill",
    "stroke",
    "all",
    "bounding-box",
];

/// Validate one `GGPlot::layer_svg_attr(name, value)` (#221). The
/// allow-list: `class` (space-separated tokens, each
/// `[A-Za-z][A-Za-z0-9_-]*`; invalid tokens are dropped, the rest kept),
/// `aria-hidden` (`"true"`/`"false"`), `pointer-events` (an SVG keyword,
/// e.g. `"none"`), and `data-*` (same name rules as `Aes::data_attr`).
/// Everything else — notably `style`, `on*` handlers, `id` — is rejected.
/// Returns the attribute to keep (if any) and pushes a warning for anything
/// dropped.
pub(crate) fn validate_layer_svg_attr(
    name: &str,
    value: &str,
    warnings: &mut Vec<String>,
) -> Option<(String, String)> {
    let keep = |v: &str| Some((name.to_string(), v.to_string()));
    match name {
        "class" => {
            let (ok, bad) = crate::aes::split_class_tokens(value);
            if !bad.is_empty() {
                warnings.push(format!(
                    "layer_svg_attr(\"class\"): invalid class token(s) {bad:?} (must match [A-Za-z][A-Za-z0-9_-]*) — dropped"
                ));
            }
            if ok.is_empty() {
                None
            } else {
                keep(&ok.join(" "))
            }
        }
        "aria-hidden" if matches!(value, "true" | "false") => keep(value),
        "pointer-events" if POINTER_EVENTS_VALUES.contains(&value) => keep(value),
        "aria-hidden" | "pointer-events" => {
            warnings.push(format!(
                "layer_svg_attr(\"{name}\"): value {value:?} is not allowed — ignored"
            ));
            None
        }
        _ if is_valid_mark_attr_name(name) => keep(value),
        _ => {
            warnings.push(format!(
                "layer_svg_attr(\"{name}\") is not on the allow-list (class, aria-hidden, pointer-events, data-*) — ignored"
            ));
            None
        }
    }
}

fn rgb((r, g, b): (u8, u8, u8)) -> String {
    format!("#{r:02X}{g:02X}{b:02X}")
}

/// Escape text for use in SVG element *content* (`<text>`/`<title>`/
/// `<tspan>` text nodes). Quotes are escaped too: a `"` inside content is
/// harmless to SVG itself, but the document is routinely inlined into
/// HTML, where an unescaped `"` closing out of an attribute could matter
/// if this ever moved context — escaping it is free and one less thing to
/// reason about. `\t`/`\n`/`\r` are kept literal here (see `escape_attr`,
/// its sibling for *attribute* values, for why those three need different
/// treatment there). Characters that are not legal in XML 1.0 (C0
/// controls other than tab/LF/CR) are dropped so the document stays
/// well-formed.
fn escape(s: &str) -> String {
    escape_impl(s, false)
}

/// Escape text for use in a *quoted SVG attribute value* (#226) — a
/// `data-*`/`href`/`class`/`font-family`/etc. attribute, as opposed to
/// `escape`'s element content. Everything `escape` does, plus `\t`/`\n`/
/// `\r` entity-escaped (`&#9;`/`&#10;`/`&#13;`) instead of left literal.
/// XML attribute-value normalisation collapses an unescaped tab/newline/CR
/// inside an attribute value to a plain space (the SVG spec defers to
/// XML 1.0 §3.3.3 here, and browsers follow it), so a caller reading a
/// multi-line `data-label` or similar back loses the original character
/// unless it's entity-escaped — entity-escaping round-trips it exactly
/// through any XML/DOM parser.
fn escape_attr(s: &str) -> String {
    escape_impl(s, true)
}

fn escape_impl(s: &str, in_attr: bool) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            '\t' if in_attr => out.push_str("&#9;"),
            '\n' if in_attr => out.push_str("&#10;"),
            '\r' if in_attr => out.push_str("&#13;"),
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

/// Split `pts` into maximal runs of consecutive finite vertices, dropping
/// any run shorter than 2 points. Matches ggplot2's NA-break handling for
/// lines: a non-finite vertex breaks the line instead of being silently
/// skipped, which would otherwise visually bridge across the gap — but a
/// single remaining observation isn't a line segment either (ggplot2 drops
/// a one-point path piece the same way), so it draws nothing rather than
/// an invisible 1-point `<polyline>` that would still carry a `<title>`
/// and hover attributes.
fn finite_runs(pts: &[(f64, f64)]) -> Vec<Vec<(f64, f64)>> {
    let mut runs = Vec::new();
    let mut cur: Vec<(f64, f64)> = Vec::new();
    for &(x, y) in pts {
        if x.is_finite() && y.is_finite() {
            cur.push((x, y));
        } else if !cur.is_empty() {
            if cur.len() >= 2 {
                runs.push(std::mem::take(&mut cur));
            } else {
                cur.clear();
            }
        }
    }
    if cur.len() >= 2 {
        runs.push(cur);
    }
    runs
}

impl DrawBackend for SvgBackend {
    fn begin_panel(&mut self, index: usize, label: &str, clip: &Rect, data: &Rect) {
        // A mark group (#221) must not span two clipped panels.
        self.flush_groups();
        self.body.push_str(&format!(
            "<g data-facet-panel=\"{index}\" clip-path=\"url(#{}-panel-{index})\">",
            self.id_prefix
        ));
        self.panels
            .push((index, label.to_string(), clip.clone(), data.clone()));
    }

    fn end_panel(&mut self) {
        self.flush_groups();
        self.body.push_str("</g>");
    }

    fn set_viewport(&mut self, rect: &Rect) {
        self.viewport = Some(rect.clone());
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

    fn set_mark_geom(&mut self, geom: Option<&str>) {
        self.mark_geom = geom.map(str::to_string);
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

    fn wants_formatted_values(&self) -> bool {
        self.formatted_values
    }

    fn set_mark_fvalue(&mut self, value: Option<String>) {
        self.mark_fvalue = value;
    }

    fn set_mark_fx(&mut self, value: Option<String>) {
        self.mark_fx = value;
    }

    fn set_mark_class(&mut self, class: Option<String>) {
        self.mark_user_class = class;
    }

    fn set_mark_group(&mut self, group: Option<(String, String)>) {
        self.mark_group = group;
    }

    fn set_layer_attrs(&mut self, attrs: Vec<(String, String)>) {
        self.layer_attrs = attrs;
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
            "cx=\"{cx:.2}\" cy=\"{cy:.2}\" r=\"{:.2}\" fill=\"{}\"{}",
            radius.max(0.0),
            rgb(style.color),
            opacity_attr("fill", style.alpha)
        );
        self.push_mark("circle", "point", &attrs);
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
        let fill_opacity = opacity_attr("fill", style.alpha);
        let stroke_opacity = opacity_attr("stroke", style.alpha);
        let poly = |pts: &[(f64, f64)]| {
            if style.filled {
                format!("points=\"{}\" fill=\"{color}\"{fill_opacity}", points(pts))
            } else {
                format!(
                    "points=\"{}\" fill=\"none\" stroke=\"{color}\"{stroke_opacity}",
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
                self.push_mark("polygon", "point", &a);
            }
            PointShape::Triangle => {
                let a = poly(&[(cx, cy - r), (cx + r, cy + r), (cx - r, cy + r)]);
                self.push_mark("polygon", "point", &a);
            }
            PointShape::Diamond => {
                let a = poly(&[(cx, cy - r), (cx + r, cy), (cx, cy + r), (cx - r, cy)]);
                self.push_mark("polygon", "point", &a);
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
                    "d=\"{d}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"{:.2}\"{stroke_opacity}",
                    (r / 2.5).clamp(1.0, 3.0)
                );
                self.push_mark("path", "point", &a);
            }
        }
        Ok(())
    }

    fn draw_line(&mut self, pts: &[(f64, f64)], style: &LineStyle) -> Result<(), RenderError> {
        // Split at non-finite vertices instead of silently dropping them:
        // dropping-and-joining would visually bridge the gap, but ggplot2
        // treats a non-finite (NA) vertex as a break in the line (see
        // `finite_runs`). Each run becomes its own `<polyline>` mark, with
        // the same style and the same data-x/series/value/part attrs.
        for run in finite_runs(pts) {
            self.draw_line_run(&run, style);
        }
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
        let dash = dasharray_attr(&style.linetype);
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
            "d=\"{d}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{:.2}\"{}{}{}",
            rgb(style.color),
            finite_or(style.width, 1.0),
            opacity_attr("stroke", style.alpha),
            dash,
            linecap
        );
        self.push_mark("path", "line", &attrs);
        Ok(())
    }

    fn draw_rect(
        &mut self,
        top_left: (f64, f64),
        bottom_right: (f64, f64),
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        let fill = style.fill.map(rgb).unwrap_or_else(|| "none".into());
        self.rect_markup(top_left, bottom_right, &fill, style)
    }

    /// #206: same markup [`draw_rect`](Self::draw_rect) builds, filled
    /// with `pattern`'s registered `url(#…)` instead of `style.fill`.
    fn draw_rect_pattern(
        &mut self,
        top_left: (f64, f64),
        bottom_right: (f64, f64),
        pattern: &FillPattern,
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        let fill = self.pattern_url(pattern);
        self.rect_markup(top_left, bottom_right, &fill, style)
    }

    fn draw_polygon(&mut self, pts: &[(f64, f64)], style: &RectStyle) -> Result<(), RenderError> {
        let fill = style.fill.map(rgb).unwrap_or_else(|| "none".into());
        self.polygon_markup(pts, &fill, style)
    }

    /// #325: an `<image>` with the bytes as a `data:` URI, in a nested
    /// `<svg>` viewport that clips it to `clip`.
    fn draw_image(
        &mut self,
        bytes: &[u8],
        rect: &Rect,
        clip: &Rect,
        alpha: f64,
    ) -> Result<(), RenderError> {
        let finite = [rect.x, rect.y, rect.width, rect.height, clip.x, clip.y]
            .iter()
            .chain([&clip.width, &clip.height])
            .all(|v| v.is_finite());
        if !finite || rect.width <= 0.0 || rect.height <= 0.0 {
            return Ok(());
        }
        let mime = match bytes {
            [0xFF, 0xD8, ..] => "image/jpeg",
            _ => "image/png",
        };
        self.emit(&format!(
            "<svg x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" overflow=\"hidden\">",
            clip.x, clip.y, clip.width, clip.height
        ));
        let attrs = format!(
            "x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" preserveAspectRatio=\"none\"{} href=\"data:{mime};base64,{}\"",
            rect.x - clip.x,
            rect.y - clip.y,
            rect.width,
            rect.height,
            if alpha < 1.0 {
                format!(" opacity=\"{:.3}\"", alpha.clamp(0.0, 1.0))
            } else {
                String::new()
            },
            base64(bytes)
        );
        self.push_mark("image", "image", &attrs);
        self.emit("</svg>");
        Ok(())
    }

    /// #206: same markup [`draw_polygon`](Self::draw_polygon) builds,
    /// filled with `pattern`'s registered `url(#…)` instead of `style.fill`.
    fn draw_polygon_pattern(
        &mut self,
        pts: &[(f64, f64)],
        pattern: &FillPattern,
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        let fill = self.pattern_url(pattern);
        self.polygon_markup(pts, &fill, style)
    }

    /// One `<path>`, every ring a subpath (`M…Z`), `fill-rule="evenodd"` so
    /// holes (rings after the first) are cut out of the fill — SVG's native
    /// equivalent of the even-odd rasteriser the pixel/plotters backends run
    /// (#178). A ring with fewer than 3 finite points is dropped; the whole
    /// shape draws nothing when every ring is.
    fn draw_polygon_rings(
        &mut self,
        rings: &[Vec<(f64, f64)>],
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        let fill = style.fill.map(rgb).unwrap_or_else(|| "none".into());
        self.polygon_rings_markup(rings, &fill, style)
    }

    /// #206: same markup [`draw_polygon_rings`](Self::draw_polygon_rings)
    /// builds (holes cut out under the same even-odd rule), filled with
    /// `pattern`'s registered `url(#…)` instead of `style.fill`.
    fn draw_polygon_rings_pattern(
        &mut self,
        rings: &[Vec<(f64, f64)>],
        pattern: &FillPattern,
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        let fill = self.pattern_url(pattern);
        self.polygon_rings_markup(rings, &fill, style)
    }

    /// #214: with `precision` set, writes a compact `d` (absolute `M` per
    /// ring, then one `l` with implicit-repeat relative coordinate pairs,
    /// rounded to `precision` decimals) instead of the one-`L`-per-point
    /// absolute encoding [`Self::polygon_rings_markup`] builds. `None`
    /// falls back to the trait default (today's encoding, unchanged).
    fn draw_polygon_rings_precise(
        &mut self,
        rings: &[Vec<(f64, f64)>],
        pattern: Option<&FillPattern>,
        precision: Option<u8>,
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        let Some(precision) = precision else {
            return match pattern {
                Some(p) => self.draw_polygon_rings_pattern(rings, p, style),
                None => self.draw_polygon_rings(rings, style),
            };
        };
        let d = compact_rings_d(rings, precision);
        if d.is_empty() {
            return Ok(());
        }
        let fill = match pattern {
            Some(p) => self.pattern_url(p),
            None => style.fill.map(rgb).unwrap_or_else(|| "none".into()),
        };
        let stroke = style.stroke.map(rgb).unwrap_or_else(|| "none".into());
        let attrs = format!(
            "d=\"{d}\" fill-rule=\"evenodd\" fill=\"{fill}\"{} stroke=\"{stroke}\" stroke-width=\"{:.2}\"",
            opacity_attr("fill", style.alpha),
            finite_or(style.stroke_width, 0.0)
        );
        self.push_mark("path", "band", &attrs);
        Ok(())
    }

    fn draw_text(
        &mut self,
        text: &str,
        pos: (f64, f64),
        style: &TextStyle,
    ) -> Result<(), RenderError> {
        self.draw_text_with_title(text, pos, style, None)
    }

    /// #159: with `title`, wraps the `<text>` in a `<title>` child
    /// carrying the full (escaped) label — e.g. a legend key truncated
    /// with an ellipsis to fit a vertical multi-column grid.
    fn draw_text_with_title(
        &mut self,
        text: &str,
        (x, y): (f64, f64),
        style: &TextStyle,
        title: Option<&str>,
    ) -> Result<(), RenderError> {
        if !(x.is_finite() && y.is_finite()) {
            return Ok(());
        }
        let anchor = match style.anchor {
            TextAnchor::Start => "start",
            TextAnchor::Middle => "middle",
            TextAnchor::End => "end",
        };
        let family = escape_attr(style.family.as_deref().unwrap_or("sans-serif"));
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
        let halo = text_halo_attrs(style);
        let angle = finite_or(style.angle, 0.0);
        let transform = if angle.abs() > 0.01 {
            format!(" transform=\"rotate({angle:.1} {x:.2} {y:.2})\"")
        } else {
            String::new()
        };
        let class = format!(
            "{}{}{}",
            self.text_data_attrs(),
            self.layer_flag_attrs(false),
            self.mark_class("text")
        );
        let title_tag = match title {
            Some(t) => format!("<title>{}</title>", escape(t)),
            None => String::new(),
        };
        let markup = format!(
            "<text x=\"{x:.2}\" y=\"{y:.2}\" font-size=\"{:.2}\" text-anchor=\"{anchor}\" \
             dominant-baseline=\"middle\" font-family=\"{family}\"{weight}{fstyle}{halo} fill=\"{}\"{transform}{class}>{title_tag}{}</text>",
            finite_or(style.size, 12.0),
            rgb(style.color),
            escape(text)
        );
        self.emit(&markup);
        Ok(())
    }

    /// #196: each linked run becomes `<a href="…" target="_blank"
    /// rel="noopener"><tspan text-decoration="underline">…</tspan></a>`
    /// (`rel="noopener"` so the opened tab can't reach back via
    /// `window.opener`); a plain run is a bare `<tspan>`. All runs sit
    /// inside one `<text>`, so `style.anchor` aligns their combined width
    /// exactly as it would a single run's.
    fn draw_text_runs(
        &mut self,
        runs: &[(&str, Option<&str>)],
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
        let family = escape_attr(style.family.as_deref().unwrap_or("sans-serif"));
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
        let halo = text_halo_attrs(style);
        let angle = finite_or(style.angle, 0.0);
        let transform = if angle.abs() > 0.01 {
            format!(" transform=\"rotate({angle:.1} {x:.2} {y:.2})\"")
        } else {
            String::new()
        };
        let class = format!(
            "{}{}{}",
            self.text_data_attrs(),
            self.layer_flag_attrs(false),
            self.mark_class("text")
        );
        let mut inner = String::new();
        for (text, href) in runs {
            match href {
                Some(u) => inner.push_str(&format!(
                    "<a href=\"{}\" target=\"_blank\" rel=\"noopener\"><tspan text-decoration=\"underline\">{}</tspan></a>",
                    escape_attr(u),
                    escape(text)
                )),
                None => inner.push_str(&format!("<tspan>{}</tspan>", escape(text))),
            }
        }
        let markup = format!(
            "<text x=\"{x:.2}\" y=\"{y:.2}\" font-size=\"{:.2}\" text-anchor=\"{anchor}\" \
             dominant-baseline=\"middle\" font-family=\"{family}\"{weight}{fstyle}{halo} fill=\"{}\"{transform}{class}>{inner}</text>",
            finite_or(style.size, 12.0),
            rgb(style.color),
        );
        self.emit(&markup);
        Ok(())
    }
}

/// #229: the `stroke`/`stroke-width`/`stroke-linejoin`/`paint-order`
/// attributes for a text halo (ggrepel's `bg.colour`/`bg.r`;
/// `shadowtext`'s `bg.col`/`bg.r`), or `""` when `style.bg_colour` is
/// `None` or `style.bg_r` is non-finite or `<= 0.0` — so the default
/// (`bg_colour: None`) keeps existing output byte-identical. The stroke
/// paints *under* the fill (`paint-order="stroke"`), so the halo never
/// overlaps the glyph's own fill colour. `rgb()` already allow-lists its
/// output to `#RRGGBB` (#221's convention: every attribute value stays
/// escaped/allow-listed, never a raw caller string).
fn text_halo_attrs(style: &TextStyle) -> String {
    let Some(bg) = style.bg_colour else {
        return String::new();
    };
    let r = finite_or(style.bg_r, 0.0);
    if r <= 0.0 {
        return String::new();
    }
    format!(
        " stroke=\"{}\" stroke-width=\"{:.2}\" stroke-linejoin=\"round\" paint-order=\"stroke\"",
        rgb(bg),
        2.0 * r
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::Rect;

    /// #221: the backend re-checks everything callers hand it directly
    /// through the public `DrawBackend` setters (defence in depth).
    #[test]
    fn issue221_backend_rechecks_class_group_and_layer_attrs() {
        let mut b = backend();
        b.set_mark_geom(Some("point"));
        b.set_mark_class(Some("ok \"bad 1x ok".into()));
        b.set_layer_attrs(vec![
            ("aria-hidden".into(), "maybe".into()),
            ("pointer-events".into(), "none\" onload=\"x".into()),
            ("style".into(), "fill:red".into()),
            ("class".into(), "lay".into()),
        ]);
        b.set_mark_group(Some(("onclick".into(), "x".into())));
        b.draw_circle((1.0, 1.0), 1.0, &PointStyle::default())
            .unwrap();
        b.set_mark_group(Some(("data-of".into(), "n".into())));
        let runs = [("a", None), ("b", Some("https://e.x"))];
        b.draw_text_runs(&runs, (5.0, 5.0), &TextStyle::default())
            .unwrap();
        b.set_mark_group(None);
        b.set_mark_geom(None);
        let body = b.body().to_string();
        assert!(
            body.contains(r#"class="gg-mark gg-point lay ok""#),
            "{body}"
        );
        assert!(!body.contains("bad") && !body.contains("1x"), "{body}");
        assert!(
            !body.contains("aria-hidden") && !body.contains("onload"),
            "{body}"
        );
        assert!(
            !body.contains("style=") && !body.contains("onclick"),
            "{body}"
        );
        assert!(body.contains(r#"<g data-of="n"><text"#), "{body}");
        assert!(body.contains("gg-text lay ok"), "{body}");
        // A later finish() has nothing left to splice.
        assert!(b.finish().contains(r#"<g data-of="n">"#));
    }

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

    // #206: pattern fills.

    fn stripe() -> FillPattern {
        FillPattern::Stripe {
            fg: (10, 20, 30),
            bg: (200, 210, 220),
            size_px: 4.0,
            angle_deg: 45.0,
        }
    }

    fn pattern_id(svg: &str) -> String {
        let i = svg.find("<pattern id=\"").unwrap() + "<pattern id=\"".len();
        svg[i..i + svg[i..].find('"').unwrap()].to_string()
    }

    #[test]
    fn a_pattern_fill_registers_one_defs_pattern_and_references_it() {
        let mut b = backend();
        b.draw_rect_pattern((0.0, 0.0), (10.0, 10.0), &stripe(), &RectStyle::default())
            .unwrap();
        let svg = b.finish();
        assert_eq!(svg.matches("<pattern ").count(), 1, "{svg}");
        let id = pattern_id(&svg);
        assert!(
            svg.contains(&format!("fill=\"url(#{id})\"")),
            "rect references the registered pattern id {id}: {svg}"
        );
    }

    #[test]
    fn a_rounded_rect_pattern_still_rounds_its_corners() {
        let mut b = backend();
        b.draw_rect_pattern(
            (0.0, 0.0),
            (10.0, 10.0),
            &stripe(),
            &RectStyle {
                corner_radius: 3.0,
                ..Default::default()
            },
        )
        .unwrap();
        let svg = b.finish();
        assert!(svg.contains("rx=\"3.00\""), "{svg}");
        assert!(svg.contains("fill=\"url(#"), "{svg}");
    }

    #[test]
    fn an_equal_pattern_reuses_the_same_defs_entry() {
        let mut b = backend();
        b.draw_rect_pattern((0.0, 0.0), (5.0, 5.0), &stripe(), &RectStyle::default())
            .unwrap();
        b.draw_polygon_pattern(
            &[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)],
            &stripe(),
            &RectStyle::default(),
        )
        .unwrap();
        let svg = b.finish();
        assert_eq!(
            svg.matches("<pattern ").count(),
            1,
            "two equal patterns share one <defs> entry: {svg}"
        );
    }

    #[test]
    fn different_patterns_get_their_own_defs_entries() {
        let mut b = backend();
        b.draw_rect_pattern((0.0, 0.0), (5.0, 5.0), &stripe(), &RectStyle::default())
            .unwrap();
        let other = FillPattern::Stripe {
            fg: (1, 1, 1),
            bg: (2, 2, 2),
            size_px: 8.0,
            angle_deg: 0.0,
        };
        b.draw_rect_pattern((0.0, 0.0), (5.0, 5.0), &other, &RectStyle::default())
            .unwrap();
        let svg = b.finish();
        assert_eq!(svg.matches("<pattern ").count(), 2, "{svg}");
    }

    #[test]
    fn no_pattern_used_means_no_defs_block_at_all() {
        let mut b = backend();
        b.draw_rect((0.0, 0.0), (5.0, 5.0), &RectStyle::default())
            .unwrap();
        let svg = b.finish();
        assert!(!svg.contains("<defs>"), "{svg}");
    }

    #[test]
    fn pattern_fill_cuts_out_holes_like_a_flat_fill_does() {
        let mut b = backend();
        let outer = vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        let hole = vec![(2.0, 2.0), (8.0, 2.0), (8.0, 8.0), (2.0, 8.0)];
        b.draw_polygon_rings_pattern(&[outer, hole], &stripe(), &RectStyle::default())
            .unwrap();
        let svg = b.finish();
        assert!(svg.contains("fill-rule=\"evenodd\""), "{svg}");
        assert_eq!(
            svg.matches("<path").count(),
            1,
            "one path, both rings: {svg}"
        );
    }

    #[test]
    fn two_backends_on_one_page_never_collide_on_a_pattern_id() {
        let mut a = backend();
        a.draw_rect_pattern((0.0, 0.0), (5.0, 5.0), &stripe(), &RectStyle::default())
            .unwrap();
        let mut b = backend();
        b.draw_rect_pattern((0.0, 0.0), (5.0, 5.0), &stripe(), &RectStyle::default())
            .unwrap();
        let (sa, sb) = (a.finish(), b.finish());
        assert_ne!(pattern_id(&sa), pattern_id(&sb), "{sa}\n{sb}");
    }

    /// #214: decode a [`compact_rings_d`]-style `d` back into absolute
    /// points, for the round-trip test below. Relies on the same
    /// separator rule [`push_num`] writes: a number starts right after a
    /// command letter, right after a comma, or right at a `-` sign (which
    /// needs no comma before it); every other digit boundary has an
    /// explicit comma.
    fn decode_compact_d(d: &str) -> Vec<Vec<(f64, f64)>> {
        let b = d.as_bytes();
        let mut i = 0;
        let read_num = |b: &[u8], i: &mut usize| -> f64 {
            let start = *i;
            if *i < b.len() && b[*i] == b'-' {
                *i += 1;
            }
            while *i < b.len() && b[*i].is_ascii_digit() {
                *i += 1;
            }
            if *i < b.len() && b[*i] == b'.' {
                *i += 1;
                while *i < b.len() && b[*i].is_ascii_digit() {
                    *i += 1;
                }
            }
            std::str::from_utf8(&b[start..*i]).unwrap().parse().unwrap()
        };
        let is_num_start = |c: u8| c.is_ascii_digit() || c == b'-' || c == b'.';
        let mut rings = Vec::new();
        let mut current = Vec::new();
        let mut cursor = (0.0, 0.0);
        while i < b.len() {
            match b[i] {
                b'M' => {
                    if !current.is_empty() {
                        rings.push(std::mem::take(&mut current));
                    }
                    i += 1;
                    let x = read_num(b, &mut i);
                    if i < b.len() && b[i] == b',' {
                        i += 1;
                    }
                    let y = read_num(b, &mut i);
                    cursor = (x, y);
                    current.push(cursor);
                }
                b'l' => {
                    i += 1;
                    while i < b.len() && is_num_start(b[i]) {
                        let dx = read_num(b, &mut i);
                        if i < b.len() && b[i] == b',' {
                            i += 1;
                        }
                        let dy = read_num(b, &mut i);
                        if i < b.len() && b[i] == b',' {
                            i += 1;
                        }
                        cursor = (cursor.0 + dx, cursor.1 + dy);
                        current.push(cursor);
                    }
                }
                b'z' => {
                    i += 1;
                    if !current.is_empty() {
                        rings.push(std::mem::take(&mut current));
                    }
                }
                _ => i += 1,
            }
        }
        if !current.is_empty() {
            rings.push(current);
        }
        rings
    }

    /// A noisy, roughly-circular ring with `n` points, the shape of a
    /// simplified real boundary (not axis-aligned, no repeating deltas).
    fn noisy_ring(n: usize, cx: f64, cy: f64, r: f64) -> Vec<(f64, f64)> {
        (0..n)
            .map(|i| {
                let t = i as f64 / n as f64 * std::f64::consts::TAU;
                let wobble = 1.0 + 0.03 * ((i * 7) as f64).sin();
                (cx + r * wobble * t.cos(), cy + r * wobble * t.sin())
            })
            .collect()
    }

    #[test]
    fn compact_encoding_is_well_under_the_absolute_encodings_size() {
        // Isolated from simplification entirely: the same 2,000-point
        // ring, encoded both ways directly (no GeomSf/simplify_px
        // involved) — the byte-size win is the encoding alone.
        let ring = noisy_ring(2000, 400.0, 300.0, 280.0);
        let mut b = backend();
        b.draw_polygon_rings(std::slice::from_ref(&ring), &RectStyle::default())
            .unwrap();
        let absolute_svg = b.finish();
        let absolute_d = absolute_svg
            .split("d=\"")
            .nth(1)
            .and_then(|s| s.split('"').next())
            .expect("absolute path d");

        let compact_d = compact_rings_d(std::slice::from_ref(&ring), 1);

        assert!(
            compact_d.len() * 3 < absolute_d.len() * 2,
            "compact {} bytes should be well under 2/3 of absolute {} bytes",
            compact_d.len(),
            absolute_d.len()
        );
    }

    #[test]
    fn compact_encoding_round_trips_within_precision() {
        let ring = noisy_ring(40, 100.0, 80.0, 60.0);
        let precision: u8 = 1;
        let d = compact_rings_d(std::slice::from_ref(&ring), precision);
        let decoded = decode_compact_d(&d);
        assert_eq!(decoded.len(), 1, "{d}");
        assert_eq!(decoded[0].len(), ring.len(), "{d}");
        // Each step's rounding error is at most half a unit of precision;
        // reconstructed points accumulate it additively from M, so the
        // tolerance at point k scales with k (still a tight, provable
        // bound, not an arbitrary constant).
        let step_tol = 0.5 * 10f64.powi(-(precision as i32));
        for (k, (orig, got)) in ring.iter().zip(decoded[0].iter()).enumerate() {
            let tol = step_tol * (k as f64 + 1.0) + 1e-9;
            assert!(
                (orig.0 - got.0).abs() <= tol && (orig.1 - got.1).abs() <= tol,
                "point {k}: {orig:?} vs {got:?} (tol {tol}): {d}"
            );
        }
    }

    #[test]
    fn compact_encoding_round_trips_a_hole_and_a_multipolygons_parts() {
        // Two parts (first a plain square, second a square with a hole) —
        // exactly the ring-set shape `GeomSf::draw_rings` now passes
        // through for a MultiPolygon row (#213) or a Polygon with holes.
        let part1 = vec![(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)];
        let part2_outer = vec![(50.0, 0.0), (90.0, 0.0), (90.0, 40.0), (50.0, 40.0)];
        let part2_hole = vec![(60.0, 10.0), (80.0, 10.0), (80.0, 30.0), (60.0, 30.0)];
        let rings = vec![part1.clone(), part2_outer.clone(), part2_hole.clone()];
        let d = compact_rings_d(&rings, 1);
        let decoded = decode_compact_d(&d);
        assert_eq!(decoded.len(), 3, "{d}");
        for (orig, got) in [part1, part2_outer, part2_hole].iter().zip(decoded.iter()) {
            assert_eq!(orig.len(), got.len(), "{d}");
            for (o, g) in orig.iter().zip(got.iter()) {
                assert!((o.0 - g.0).abs() <= 0.1 && (o.1 - g.1).abs() <= 0.1, "{d}");
            }
        }
        assert!(d.contains("M0,0l20,0,0,20-20,0z"), "{d}");
    }
}
