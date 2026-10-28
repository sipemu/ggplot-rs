#[cfg(feature = "serde")]
pub mod config;
pub mod elements;
pub mod presets;

pub use elements::{ElementGeom, ElementLine, ElementRect, ElementText};

/// Position of the legend.
#[derive(Clone, Debug)]
pub enum LegendPosition {
    Right,
    Left,
    Top,
    Bottom,
    None,
    /// Inside the panel at panel-relative coordinates (0..1, 0..1), like R's
    /// `legend.position = c(x, y)`. `(0, 0)` is bottom-left, `(1, 1)` top-right.
    Inside(f64, f64),
}

/// Whether title/subtitle/caption align to the panel or the whole plot width
/// (R's `plot.title.position` / `plot.caption.position`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum TitlePosition {
    #[default]
    Panel,
    Plot,
}

/// Corner for the `labs(tag)` label (R's `plot.tag.position`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum TagPosition {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Layout direction of the legend keys (R's `legend.direction`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LegendDirection {
    Vertical,
    Horizontal,
}

/// How an outer x-axis tick label that would cross the SVG's edge is
/// handled (B3-1; ggplot2 itself leaves this to the viewport/device and has
/// no direct equivalent — this is a ggplot-rs extension, D4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AxisLabelFit {
    /// The 0.21 behaviour: the label is drawn exactly where ggplot2's own
    /// math puts it, even if that crosses the SVG edge.
    Clip,
    /// The panel keeps its size; an outer label that would cross the edge
    /// is shifted inward just enough to stay inside, at draw time. Use this
    /// for panels that must stay pixel-aligned with other elements (e.g. a
    /// consumer's `::TRACK`-aligned strip).
    Shift,
    /// The layout reserves just the overflow as extra left/right panel
    /// margin, so a plot whose outer labels already fit is byte-identical.
    /// Anything still outside after that (e.g. a faceted plot, where the
    /// reservation is skipped) is shifted inward at draw time, same as
    /// [`Shift`](Self::Shift).
    #[default]
    Reserve,
}

/// Plot margins (in pixels).
#[derive(Clone, Debug)]
pub struct Margin {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl Default for Margin {
    fn default() -> Self {
        Margin {
            top: 10.0,
            right: 10.0,
            bottom: 10.0,
            left: 10.0,
        }
    }
}

/// Complete theme specification for a plot.
#[derive(Clone, Debug)]
pub struct Theme {
    // ── Existing fields ──
    pub text: ElementText,
    pub title: ElementText,
    pub axis_text_x: ElementText,
    pub axis_text_y: ElementText,
    pub axis_title_x: ElementText,
    pub axis_title_y: ElementText,
    pub axis_line: ElementLine,
    pub axis_ticks: ElementLine,
    pub panel_background: ElementRect,
    pub panel_grid_major: ElementLine,
    pub panel_grid_minor: ElementLine,
    pub plot_background: ElementRect,
    pub legend_position: LegendPosition,
    pub plot_margin: Margin,

    // ── New text elements ──
    pub subtitle: ElementText,
    pub caption: ElementText,
    pub legend_title: ElementText,
    pub legend_text: ElementText,
    pub strip_text: ElementText,

    // ── Per-axis optional overrides (None = inherit from parent) ──
    pub axis_line_x: Option<ElementLine>,
    pub axis_line_y: Option<ElementLine>,
    pub axis_ticks_x: Option<ElementLine>,
    pub axis_ticks_y: Option<ElementLine>,
    pub panel_grid_major_x: Option<ElementLine>,
    pub panel_grid_major_y: Option<ElementLine>,
    pub panel_grid_minor_x: Option<ElementLine>,
    pub panel_grid_minor_y: Option<ElementLine>,

    // ── New rect/line elements ──
    pub panel_border: ElementLine,
    pub legend_background: ElementRect,
    pub legend_key: ElementRect,
    pub strip_background: ElementRect,

    // ── Scalar spacing/sizing ──
    pub axis_ticks_length: f64,
    /// Number of rows to stagger x-axis tick labels across (R's
    /// `guide_axis(n.dodge = ...)`); 1 = no dodging.
    pub axis_text_x_dodge: usize,
    pub legend_key_width: f64,
    pub legend_key_height: f64,
    pub legend_spacing: f64,
    pub legend_margin: Margin,
    pub panel_spacing: f64,
    pub panel_spacing_x: Option<f64>,
    pub panel_spacing_y: Option<f64>,

    // ── Brand / primary color ──
    /// Optional brand color. When set, geoms that draw a single un-mapped series
    /// (no color/fill aesthetic) use it as their default instead of the geom's
    /// built-in color. Lets one render process serve multiple tenants' brands.
    pub primary: Option<(u8, u8, u8)>,

    // ── Advanced layout (R's theme() extras) ──
    /// Fix the panel's height:width ratio (R's `aspect.ratio`). `None` = free.
    pub aspect_ratio: Option<f64>,
    /// Draw gridlines on top of the data layers (R's `panel.ontop`).
    pub panel_ontop: bool,
    /// Draw minor tick marks between major ticks (R's `axis.minor.ticks`).
    pub axis_minor_ticks: bool,
    /// Whether title/subtitle/caption align to the panel or the whole plot
    /// (R's `plot.title.position` / `plot.caption.position`).
    pub title_position: TitlePosition,
    /// Corner for the `labs(tag)` label (R's `plot.tag.position`).
    pub tag_position: TagPosition,
    /// Legend key layout direction; `None` = auto from `legend_position`.
    pub legend_direction: Option<LegendDirection>,
    /// Month outlines of `geom_calendar`; `ElementLine::blank()` hides them.
    pub calendar_month_border: ElementLine,
    /// Frame around a continuous colourbar. `None` keeps the default
    /// (`legend_key.color`, else dark grey, 0.5 wide); `ElementLine::blank()`
    /// hides it.
    pub legend_bar_border: Option<ElementLine>,
    /// Geom ink/paper/accent/family palette (ggplot2 4.0's `element_geom`,
    /// B2-7). Every field `None` (the default) changes nothing.
    pub geom: ElementGeom,

    /// How an outer x-axis label that would cross the SVG edge is handled
    /// (B3-1, D6). Default [`AxisLabelFit::Reserve`].
    pub axis_text_x_fit: AxisLabelFit,
    /// Thin crowded x-axis labels in ggplot2's `check.overlap` priority
    /// order (B3-1, D5): off by default, so default output is unaffected.
    pub axis_text_x_check_overlap: bool,
}

impl Theme {
    /// Hide only the y position guides (axis text, title, line, ticks and
    /// the horizontal grid lines) — layout charts with a real x axis but a
    /// meaningless y (`geom_timeline`).
    pub fn hide_y_guides(&mut self) {
        self.axis_text_y.visible = false;
        self.axis_title_y.visible = false;
        let hidden = |base: &ElementLine| ElementLine {
            visible: false,
            ..base.clone()
        };
        self.axis_line_y = Some(hidden(self.axis_line_y.as_ref().unwrap_or(&self.axis_line)));
        self.axis_ticks_y = Some(hidden(
            self.axis_ticks_y.as_ref().unwrap_or(&self.axis_ticks),
        ));
        self.panel_grid_major_y = Some(hidden(
            self.panel_grid_major_y
                .as_ref()
                .unwrap_or(&self.panel_grid_major),
        ));
        self.panel_grid_minor_y = Some(hidden(
            self.panel_grid_minor_y
                .as_ref()
                .unwrap_or(&self.panel_grid_minor),
        ));
    }

    /// Hide the position guides — axis text, titles, lines, ticks and panel
    /// grid — for layout-only charts whose x/y carry no data meaning
    /// (`geom_sankey`). Backgrounds, legend and titles are untouched.
    pub fn hide_position_guides(&mut self) {
        for t in [
            &mut self.axis_text_x,
            &mut self.axis_text_y,
            &mut self.axis_title_x,
            &mut self.axis_title_y,
        ] {
            t.visible = false;
        }
        for l in [
            &mut self.axis_line,
            &mut self.axis_ticks,
            &mut self.panel_grid_major,
            &mut self.panel_grid_minor,
        ] {
            l.visible = false;
        }
        for l in [
            &mut self.axis_line_x,
            &mut self.axis_line_y,
            &mut self.axis_ticks_x,
            &mut self.axis_ticks_y,
            &mut self.panel_grid_major_x,
            &mut self.panel_grid_major_y,
            &mut self.panel_grid_minor_x,
            &mut self.panel_grid_minor_y,
        ]
        .into_iter()
        .flatten()
        {
            l.visible = false;
        }
    }

    /// Resolve the effective series color: the theme's brand color if set,
    /// otherwise the geom's own default.
    pub fn primary_or(&self, fallback: (u8, u8, u8)) -> (u8, u8, u8) {
        self.primary.unwrap_or(fallback)
    }

    /// Set the brand/primary color (builder style).
    pub fn with_primary(mut self, color: (u8, u8, u8)) -> Self {
        self.primary = Some(color);
        self
    }

    /// Set the geom ink/paper/accent/family palette (B2-7).
    pub fn set_geom(mut self, el: ElementGeom) -> Self {
        self.geom = el;
        self
    }

    /// Set just the geom ink and paper colours (B2-7's stretch goal),
    /// leaving `accent`/`family` as they were.
    pub fn with_ink_paper(mut self, ink: (u8, u8, u8), paper: (u8, u8, u8)) -> Self {
        self.geom.ink = Some(ink);
        self.geom.paper = Some(paper);
        self
    }

    /// Resolve R-style element inheritance: text-child elements still at the
    /// default `family`/`color` inherit them from the root `text` element, so
    /// setting only `theme.text.family` restyles every text element. Child
    /// elements that already override a property keep it. Called once before
    /// rendering.
    pub fn resolve_inheritance(&mut self) {
        let root = self.text.clone();
        let def = ElementText::default();
        let children: [&mut ElementText; 10] = [
            &mut self.title,
            &mut self.subtitle,
            &mut self.caption,
            &mut self.axis_text_x,
            &mut self.axis_text_y,
            &mut self.axis_title_x,
            &mut self.axis_title_y,
            &mut self.legend_title,
            &mut self.legend_text,
            &mut self.strip_text,
        ];
        for el in children {
            if el.family == def.family {
                el.family = root.family.clone();
            }
            if el.color == def.color {
                el.color = root.color;
            }
        }
    }

    // ── Fallback accessors for Option fields ──

    pub fn get_axis_line_x(&self) -> &ElementLine {
        self.axis_line_x.as_ref().unwrap_or(&self.axis_line)
    }

    pub fn get_axis_line_y(&self) -> &ElementLine {
        self.axis_line_y.as_ref().unwrap_or(&self.axis_line)
    }

    pub fn get_axis_ticks_x(&self) -> &ElementLine {
        self.axis_ticks_x.as_ref().unwrap_or(&self.axis_ticks)
    }

    pub fn get_axis_ticks_y(&self) -> &ElementLine {
        self.axis_ticks_y.as_ref().unwrap_or(&self.axis_ticks)
    }

    pub fn get_panel_grid_major_x(&self) -> &ElementLine {
        self.panel_grid_major_x
            .as_ref()
            .unwrap_or(&self.panel_grid_major)
    }

    pub fn get_panel_grid_major_y(&self) -> &ElementLine {
        self.panel_grid_major_y
            .as_ref()
            .unwrap_or(&self.panel_grid_major)
    }

    pub fn get_panel_grid_minor_x(&self) -> &ElementLine {
        self.panel_grid_minor_x
            .as_ref()
            .unwrap_or(&self.panel_grid_minor)
    }

    pub fn get_panel_grid_minor_y(&self) -> &ElementLine {
        self.panel_grid_minor_y
            .as_ref()
            .unwrap_or(&self.panel_grid_minor)
    }

    pub fn get_panel_spacing_x(&self) -> f64 {
        self.panel_spacing_x.unwrap_or(self.panel_spacing)
    }

    pub fn get_panel_spacing_y(&self) -> f64 {
        self.panel_spacing_y.unwrap_or(self.panel_spacing)
    }

    // ── Existing setters ──

    pub fn set_axis_text_x(mut self, el: ElementText) -> Self {
        self.axis_text_x = el;
        self
    }

    pub fn set_axis_text_y(mut self, el: ElementText) -> Self {
        self.axis_text_y = el;
        self
    }

    pub fn set_axis_title_x(mut self, el: ElementText) -> Self {
        self.axis_title_x = el;
        self
    }

    pub fn set_axis_title_y(mut self, el: ElementText) -> Self {
        self.axis_title_y = el;
        self
    }

    pub fn set_axis_line(mut self, el: ElementLine) -> Self {
        self.axis_line = el;
        self
    }

    pub fn set_axis_ticks(mut self, el: ElementLine) -> Self {
        self.axis_ticks = el;
        self
    }

    pub fn set_panel_background(mut self, el: ElementRect) -> Self {
        self.panel_background = el;
        self
    }

    pub fn set_panel_grid_major(mut self, el: ElementLine) -> Self {
        self.panel_grid_major = el;
        self
    }

    pub fn set_panel_grid_minor(mut self, el: ElementLine) -> Self {
        self.panel_grid_minor = el;
        self
    }

    pub fn set_plot_background(mut self, el: ElementRect) -> Self {
        self.plot_background = el;
        self
    }

    pub fn set_legend_position(mut self, pos: LegendPosition) -> Self {
        self.legend_position = pos;
        self
    }

    pub fn set_plot_margin(mut self, margin: Margin) -> Self {
        self.plot_margin = margin;
        self
    }

    pub fn set_title(mut self, el: ElementText) -> Self {
        self.title = el;
        self
    }

    pub fn set_text(mut self, el: ElementText) -> Self {
        self.text = el;
        self
    }

    // ── New setters ──

    pub fn set_subtitle(mut self, el: ElementText) -> Self {
        self.subtitle = el;
        self
    }

    pub fn set_caption(mut self, el: ElementText) -> Self {
        self.caption = el;
        self
    }

    pub fn set_legend_title(mut self, el: ElementText) -> Self {
        self.legend_title = el;
        self
    }

    pub fn set_legend_text(mut self, el: ElementText) -> Self {
        self.legend_text = el;
        self
    }

    pub fn set_strip_text(mut self, el: ElementText) -> Self {
        self.strip_text = el;
        self
    }

    pub fn set_axis_line_x(mut self, el: Option<ElementLine>) -> Self {
        self.axis_line_x = el;
        self
    }

    pub fn set_axis_line_y(mut self, el: Option<ElementLine>) -> Self {
        self.axis_line_y = el;
        self
    }

    pub fn set_axis_ticks_x(mut self, el: Option<ElementLine>) -> Self {
        self.axis_ticks_x = el;
        self
    }

    pub fn set_axis_ticks_y(mut self, el: Option<ElementLine>) -> Self {
        self.axis_ticks_y = el;
        self
    }

    pub fn set_panel_grid_major_x(mut self, el: Option<ElementLine>) -> Self {
        self.panel_grid_major_x = el;
        self
    }

    pub fn set_panel_grid_major_y(mut self, el: Option<ElementLine>) -> Self {
        self.panel_grid_major_y = el;
        self
    }

    pub fn set_panel_grid_minor_x(mut self, el: Option<ElementLine>) -> Self {
        self.panel_grid_minor_x = el;
        self
    }

    pub fn set_panel_grid_minor_y(mut self, el: Option<ElementLine>) -> Self {
        self.panel_grid_minor_y = el;
        self
    }

    pub fn set_panel_border(mut self, el: ElementLine) -> Self {
        self.panel_border = el;
        self
    }

    /// The effective colourbar frame: [`legend_bar_border`](Self::legend_bar_border),
    /// else `legend_key.color` (dark grey when unset), 0.5 wide.
    pub fn legend_bar_border_line(&self) -> ElementLine {
        self.legend_bar_border.clone().unwrap_or(ElementLine {
            color: self.legend_key.color.unwrap_or((50, 50, 50)),
            width: 0.5,
            visible: true,
            linetype: crate::render::backend::Linetype::Solid,
        })
    }

    pub fn set_calendar_month_border(mut self, el: ElementLine) -> Self {
        self.calendar_month_border = el;
        self
    }

    pub fn set_legend_bar_border(mut self, el: Option<ElementLine>) -> Self {
        self.legend_bar_border = el;
        self
    }

    pub fn set_legend_background(mut self, el: ElementRect) -> Self {
        self.legend_background = el;
        self
    }

    pub fn set_legend_key(mut self, el: ElementRect) -> Self {
        self.legend_key = el;
        self
    }

    pub fn set_strip_background(mut self, el: ElementRect) -> Self {
        self.strip_background = el;
        self
    }

    pub fn set_axis_ticks_length(mut self, val: f64) -> Self {
        self.axis_ticks_length = val;
        self
    }

    pub fn set_legend_key_width(mut self, val: f64) -> Self {
        self.legend_key_width = val;
        self
    }

    pub fn set_legend_key_height(mut self, val: f64) -> Self {
        self.legend_key_height = val;
        self
    }

    pub fn set_legend_spacing(mut self, val: f64) -> Self {
        self.legend_spacing = val;
        self
    }

    pub fn set_legend_margin(mut self, margin: Margin) -> Self {
        self.legend_margin = margin;
        self
    }

    pub fn set_panel_spacing(mut self, val: f64) -> Self {
        self.panel_spacing = val;
        self
    }

    pub fn set_panel_spacing_x(mut self, val: Option<f64>) -> Self {
        self.panel_spacing_x = val;
        self
    }

    pub fn set_panel_spacing_y(mut self, val: Option<f64>) -> Self {
        self.panel_spacing_y = val;
        self
    }

    /// Apply incremental theme modifications.
    /// Only fields that are `Some` in the update are applied.
    pub fn update(mut self, upd: ThemeUpdate) -> Self {
        if let Some(v) = upd.text {
            self.text = v;
        }
        if let Some(v) = upd.title {
            self.title = v;
        }
        if let Some(v) = upd.subtitle {
            self.subtitle = v;
        }
        if let Some(v) = upd.caption {
            self.caption = v;
        }
        if let Some(v) = upd.axis_text_x {
            self.axis_text_x = v;
        }
        if let Some(v) = upd.axis_text_y {
            self.axis_text_y = v;
        }
        if let Some(v) = upd.axis_title_x {
            self.axis_title_x = v;
        }
        if let Some(v) = upd.axis_title_y {
            self.axis_title_y = v;
        }
        if let Some(v) = upd.axis_line {
            self.axis_line = v;
        }
        if let Some(v) = upd.axis_ticks {
            self.axis_ticks = v;
        }
        if let Some(v) = upd.panel_background {
            self.panel_background = v;
        }
        if let Some(v) = upd.panel_grid_major {
            self.panel_grid_major = v;
        }
        if let Some(v) = upd.panel_grid_minor {
            self.panel_grid_minor = v;
        }
        if let Some(v) = upd.panel_border {
            self.panel_border = v;
        }
        if let Some(v) = upd.calendar_month_border {
            self.calendar_month_border = v;
        }
        if let Some(v) = upd.legend_bar_border {
            self.legend_bar_border = Some(v);
        }
        if let Some(v) = upd.plot_background {
            self.plot_background = v;
        }
        if let Some(v) = upd.legend_position {
            self.legend_position = v;
        }
        if let Some(v) = upd.legend_title {
            self.legend_title = v;
        }
        if let Some(v) = upd.legend_text {
            self.legend_text = v;
        }
        if let Some(v) = upd.legend_background {
            self.legend_background = v;
        }
        if let Some(v) = upd.strip_text {
            self.strip_text = v;
        }
        if let Some(v) = upd.strip_background {
            self.strip_background = v;
        }
        if let Some(v) = upd.plot_margin {
            self.plot_margin = v;
        }
        if let Some(v) = upd.geom {
            self.geom = v;
        }
        self
    }
}

/// Incremental theme modifications. All fields are optional — only `Some` values are applied.
/// Like R's `theme(axis.text.x = element_text(...))`.
#[derive(Clone, Debug, Default)]
pub struct ThemeUpdate {
    pub text: Option<ElementText>,
    pub title: Option<ElementText>,
    pub subtitle: Option<ElementText>,
    pub caption: Option<ElementText>,
    pub axis_text_x: Option<ElementText>,
    pub axis_text_y: Option<ElementText>,
    pub axis_title_x: Option<ElementText>,
    pub axis_title_y: Option<ElementText>,
    pub axis_line: Option<ElementLine>,
    pub axis_ticks: Option<ElementLine>,
    pub panel_background: Option<ElementRect>,
    pub panel_grid_major: Option<ElementLine>,
    pub panel_grid_minor: Option<ElementLine>,
    pub panel_border: Option<ElementLine>,
    pub calendar_month_border: Option<ElementLine>,
    pub legend_bar_border: Option<ElementLine>,
    pub plot_background: Option<ElementRect>,
    pub legend_position: Option<LegendPosition>,
    pub legend_title: Option<ElementText>,
    pub legend_text: Option<ElementText>,
    pub legend_background: Option<ElementRect>,
    pub strip_text: Option<ElementText>,
    pub strip_background: Option<ElementRect>,
    pub plot_margin: Option<Margin>,
    /// Geom ink/paper/accent/family palette (B2-7).
    pub geom: Option<ElementGeom>,
}

impl Default for Theme {
    fn default() -> Self {
        presets::theme_gray()
    }
}
