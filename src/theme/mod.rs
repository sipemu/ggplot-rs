pub mod elements;
pub mod presets;

pub use elements::{ElementLine, ElementRect, ElementText};

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
}

impl Theme {
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
    pub plot_background: Option<ElementRect>,
    pub legend_position: Option<LegendPosition>,
    pub legend_title: Option<ElementText>,
    pub legend_text: Option<ElementText>,
    pub legend_background: Option<ElementRect>,
    pub strip_text: Option<ElementText>,
    pub strip_background: Option<ElementRect>,
    pub plot_margin: Option<Margin>,
}

impl Default for Theme {
    fn default() -> Self {
        presets::theme_gray()
    }
}
