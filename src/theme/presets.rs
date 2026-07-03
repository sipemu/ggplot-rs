use super::elements::{ElementLine, ElementRect, ElementText};
use super::{LegendPosition, Margin, Theme};

/// Default base font size (matches ggplot2's default of 11pt).
const DEFAULT_BASE_SIZE: f64 = 11.0;

/// Compute text sizes relative to a base size, matching ggplot2 proportions.
fn text_sizes(base_size: f64) -> (f64, f64, f64) {
    let title = base_size * 1.2; // rel(1.2)
    let axis_title = base_size; // inherits from text (rel(1.0))
    let axis_text = base_size * 0.8; // rel(0.8)
    (title, axis_title, axis_text)
}

// ─── theme_gray ──────────────────────────────────────────────────

/// Classic ggplot2 gray theme with default base size.
pub fn theme_gray() -> Theme {
    theme_gray_base(DEFAULT_BASE_SIZE)
}

/// Classic ggplot2 gray theme with custom base font size.
///
/// Matches ggplot2's `theme_grey()` defaults:
/// - Black text, grey30 axis text, grey20 ticks
/// - Grey92 panel background with white gridlines
/// - Axis lines blank (panel defined by background, not lines)
/// - Grey85 strip background, grey10 strip text
pub fn theme_gray_base(base_size: f64) -> Theme {
    let (title_size, axis_title_size, axis_text_size) = text_sizes(base_size);
    let half_line = base_size / 2.0;
    Theme {
        text: ElementText {
            size: base_size,
            color: (0, 0, 0),
            ..Default::default()
        },
        title: ElementText {
            size: title_size,
            color: (0, 0, 0),
            ..Default::default()
        },
        axis_text_x: ElementText {
            size: axis_text_size,
            color: (77, 77, 77), // grey30
            ..Default::default()
        },
        axis_text_y: ElementText {
            size: axis_text_size,
            color: (77, 77, 77), // grey30
            hjust: 1.0,
            ..Default::default()
        },
        axis_title_x: ElementText {
            size: axis_title_size,
            color: (0, 0, 0),
            ..Default::default()
        },
        axis_title_y: ElementText {
            size: axis_title_size,
            color: (0, 0, 0),
            angle: 90.0,
            ..Default::default()
        },
        axis_line: ElementLine::blank(), // ggplot2: element_blank()
        axis_ticks: ElementLine {
            color: (51, 51, 51), // grey20
            width: 0.5,
            visible: true,
            linetype: crate::render::backend::Linetype::Solid,
        },
        panel_background: ElementRect {
            fill: Some((235, 235, 235)), // grey92
            color: None,
            width: 0.0,
            visible: true,
        },
        panel_grid_major: ElementLine {
            color: (255, 255, 255), // white
            width: 1.0,
            visible: true,
            linetype: crate::render::backend::Linetype::Solid,
        },
        panel_grid_minor: ElementLine {
            color: (255, 255, 255), // white (same as major, just thinner)
            width: 0.5,
            visible: true,
            linetype: crate::render::backend::Linetype::Solid,
        },
        plot_background: ElementRect {
            fill: Some((255, 255, 255)),
            color: None,
            width: 0.0,
            visible: true,
        },
        legend_position: LegendPosition::Right,
        plot_margin: Margin {
            top: half_line,
            right: half_line,
            bottom: half_line,
            left: half_line,
        },

        // ── New text elements ──
        subtitle: ElementText {
            size: base_size,
            color: (0, 0, 0),
            ..Default::default()
        },
        caption: ElementText {
            size: axis_text_size, // rel(0.8)
            color: (0, 0, 0),
            ..Default::default()
        },
        legend_title: ElementText {
            size: base_size,
            color: (0, 0, 0),
            ..Default::default()
        },
        legend_text: ElementText {
            size: axis_text_size, // rel(0.8)
            color: (0, 0, 0),
            ..Default::default()
        },
        strip_text: ElementText {
            size: axis_text_size, // rel(0.8)
            color: (26, 26, 26),  // grey10
            ..Default::default()
        },

        // ── Per-axis overrides (None = inherit) ──
        axis_line_x: None,
        axis_line_y: None,
        axis_ticks_x: None,
        axis_ticks_y: None,
        panel_grid_major_x: None,
        panel_grid_major_y: None,
        panel_grid_minor_x: None,
        panel_grid_minor_y: None,

        // ── New rect/line elements ──
        panel_border: ElementLine::blank(),
        legend_background: ElementRect {
            fill: Some((255, 255, 255)),
            color: None,
            width: 0.0,
            visible: true,
        },
        legend_key: ElementRect {
            fill: Some((242, 242, 242)), // grey95
            color: None,
            width: 0.0,
            visible: true,
        },
        strip_background: ElementRect {
            fill: Some((217, 217, 217)), // grey85
            color: None,                 // colour = NA
            width: 0.0,
            visible: true,
        },

        // ── Scalar spacing/sizing ──
        axis_ticks_length: half_line / 2.0,
        axis_text_x_dodge: 1,
        legend_key_width: 12.0,
        legend_key_height: 18.0,
        legend_spacing: 4.0,
        legend_margin: Margin {
            top: 10.0,
            right: 15.0,
            bottom: 10.0,
            left: 10.0,
        },
        panel_spacing: half_line,
        panel_spacing_x: None,
        panel_spacing_y: None,
        primary: None,
    }
}

// ─── theme_bw ────────────────────────────────────────────────────

/// Black and white theme (default base size).
pub fn theme_bw() -> Theme {
    theme_bw_base(DEFAULT_BASE_SIZE)
}

/// Black and white theme with custom base font size.
///
/// Inherits from theme_grey. White panel with grey20 border,
/// grey92 gridlines on white background.
pub fn theme_bw_base(base_size: f64) -> Theme {
    Theme {
        panel_background: ElementRect {
            fill: Some((255, 255, 255)),
            color: None,
            width: 0.0,
            visible: true,
        },
        panel_border: ElementLine {
            color: (51, 51, 51), // grey20
            width: 1.0,
            visible: true,
            linetype: crate::render::backend::Linetype::Solid,
        },
        panel_grid_major: ElementLine {
            color: (235, 235, 235), // grey92
            width: 0.5,
            visible: true,
            linetype: crate::render::backend::Linetype::Solid,
        },
        panel_grid_minor: ElementLine {
            color: (235, 235, 235), // grey92, thinner
            width: 0.25,
            visible: true,
            linetype: crate::render::backend::Linetype::Solid,
        },
        strip_background: ElementRect {
            fill: Some((217, 217, 217)), // grey85
            color: Some((51, 51, 51)),   // grey20
            width: 0.5,
            visible: true,
        },
        legend_key: ElementRect {
            fill: Some((255, 255, 255)), // white
            color: None,
            width: 0.0,
            visible: true,
        },
        ..theme_gray_base(base_size)
    }
}

// ─── theme_minimal ───────────────────────────────────────────────

/// Minimal theme with no panel background (default base size).
pub fn theme_minimal() -> Theme {
    theme_minimal_base(DEFAULT_BASE_SIZE)
}

/// Minimal theme with no backgrounds. Inherits from theme_bw.
pub fn theme_minimal_base(base_size: f64) -> Theme {
    Theme {
        axis_ticks: ElementLine::blank(),
        panel_background: ElementRect::blank(),
        panel_border: ElementLine::blank(),
        panel_grid_major: ElementLine {
            color: (235, 235, 235), // grey92 (from bw)
            width: 0.5,
            visible: true,
            linetype: crate::render::backend::Linetype::Solid,
        },
        panel_grid_minor: ElementLine {
            color: (235, 235, 235),
            width: 0.25,
            visible: true,
            linetype: crate::render::backend::Linetype::Solid,
        },
        plot_background: ElementRect::blank(),
        legend_background: ElementRect::blank(),
        legend_key: ElementRect::blank(),
        strip_background: ElementRect::blank(),
        ..theme_bw_base(base_size)
    }
}

// ─── theme_classic ───────────────────────────────────────────────

/// Classic theme: white background, no gridlines, L-shaped axis lines only.
/// Traditional academic/publication style. Inherits from theme_bw.
pub fn theme_classic() -> Theme {
    theme_classic_base(DEFAULT_BASE_SIZE)
}

/// Classic theme with custom base font size.
pub fn theme_classic_base(base_size: f64) -> Theme {
    Theme {
        panel_border: ElementLine::blank(),
        panel_grid_major: ElementLine::blank(),
        panel_grid_minor: ElementLine::blank(),
        axis_line: ElementLine {
            color: (0, 0, 0),
            width: 0.5,
            visible: true,
            linetype: crate::render::backend::Linetype::Solid,
        },
        axis_ticks: ElementLine {
            color: (0, 0, 0),
            width: 0.5,
            visible: true,
            linetype: crate::render::backend::Linetype::Solid,
        },
        strip_background: ElementRect {
            fill: Some((255, 255, 255)),
            color: Some((0, 0, 0)),
            width: 1.0,
            visible: true,
        },
        ..theme_bw_base(base_size)
    }
}

// ─── theme_linedraw ──────────────────────────────────────────────

/// Linedraw theme: white background, black panel border, very thin black gridlines.
/// Technical drawing aesthetic. Inherits from theme_bw.
pub fn theme_linedraw() -> Theme {
    theme_linedraw_base(DEFAULT_BASE_SIZE)
}

/// Linedraw theme with custom base font size.
pub fn theme_linedraw_base(base_size: f64) -> Theme {
    Theme {
        panel_border: ElementLine {
            color: (0, 0, 0),
            width: 1.0,
            visible: true,
            linetype: crate::render::backend::Linetype::Solid,
        },
        panel_grid_major: ElementLine {
            color: (0, 0, 0), // black, very thin
            width: 0.1,
            visible: true,
            linetype: crate::render::backend::Linetype::Solid,
        },
        panel_grid_minor: ElementLine {
            color: (0, 0, 0), // black, extremely thin
            width: 0.05,
            visible: true,
            linetype: crate::render::backend::Linetype::Solid,
        },
        axis_ticks: ElementLine {
            color: (0, 0, 0),
            width: 0.5,
            visible: true,
            linetype: crate::render::backend::Linetype::Solid,
        },
        strip_background: ElementRect {
            fill: Some((0, 0, 0)), // black
            color: None,
            width: 0.0,
            visible: true,
        },
        strip_text: ElementText {
            size: base_size * 0.8,
            color: (255, 255, 255), // white text on black strip
            ..Default::default()
        },
        ..theme_bw_base(base_size)
    }
}

// ─── theme_light ─────────────────────────────────────────────────

/// Light theme: white background, light gray panel border and gridlines.
/// Softer version with grey70 accents. Inherits from theme_grey.
pub fn theme_light() -> Theme {
    theme_light_base(DEFAULT_BASE_SIZE)
}

/// Light theme with custom base font size.
pub fn theme_light_base(base_size: f64) -> Theme {
    Theme {
        panel_background: ElementRect {
            fill: Some((255, 255, 255)),
            color: None,
            width: 0.0,
            visible: true,
        },
        panel_border: ElementLine {
            color: (179, 179, 179), // grey70
            width: 1.0,
            visible: true,
            linetype: crate::render::backend::Linetype::Solid,
        },
        panel_grid_major: ElementLine {
            color: (222, 222, 222), // grey87
            width: 0.5,
            visible: true,
            linetype: crate::render::backend::Linetype::Solid,
        },
        panel_grid_minor: ElementLine {
            color: (222, 222, 222), // grey87, thinner
            width: 0.25,
            visible: true,
            linetype: crate::render::backend::Linetype::Solid,
        },
        axis_ticks: ElementLine {
            color: (179, 179, 179), // grey70
            width: 0.5,
            visible: true,
            linetype: crate::render::backend::Linetype::Solid,
        },
        legend_key: ElementRect {
            fill: Some((255, 255, 255)),
            color: None,
            width: 0.0,
            visible: true,
        },
        strip_background: ElementRect {
            fill: Some((179, 179, 179)), // grey70
            color: None,
            width: 0.0,
            visible: true,
        },
        strip_text: ElementText {
            size: base_size * 0.8,
            color: (255, 255, 255), // white text on grey70 strip
            ..Default::default()
        },
        ..theme_gray_base(base_size)
    }
}

// ─── theme_dark ──────────────────────────────────────────────────

/// Dark theme: white plot background with dark grey50 panel.
/// Makes colored data pop. Inherits from theme_grey.
pub fn theme_dark() -> Theme {
    theme_dark_base(DEFAULT_BASE_SIZE)
}

/// Dark theme with custom base font size.
///
/// Note: ggplot2's theme_dark has a white plot background but dark panel.
/// Text remains black (inherited from theme_grey).
pub fn theme_dark_base(base_size: f64) -> Theme {
    Theme {
        panel_background: ElementRect {
            fill: Some((127, 127, 127)), // grey50
            color: None,
            width: 0.0,
            visible: true,
        },
        panel_grid_major: ElementLine {
            color: (107, 107, 107), // ~grey42
            width: 0.5,
            visible: true,
            linetype: crate::render::backend::Linetype::Solid,
        },
        panel_grid_minor: ElementLine {
            color: (107, 107, 107), // ~grey42, thinner
            width: 0.25,
            visible: true,
            linetype: crate::render::backend::Linetype::Solid,
        },
        axis_ticks: ElementLine {
            color: (51, 51, 51), // grey20
            width: 0.5,
            visible: true,
            linetype: crate::render::backend::Linetype::Solid,
        },
        strip_background: ElementRect {
            fill: Some((38, 38, 38)), // ~grey15
            color: None,
            width: 0.0,
            visible: true,
        },
        strip_text: ElementText {
            size: base_size * 0.8,
            color: (230, 230, 230), // grey90
            ..Default::default()
        },
        legend_key: ElementRect {
            fill: Some((127, 127, 127)), // grey50, matches panel
            color: None,
            width: 0.0,
            visible: true,
        },
        ..theme_gray_base(base_size)
    }
}

// ─── theme_void ──────────────────────────────────────────────────

/// Void theme: completely blank — no axes, ticks, gridlines, labels, or background.
/// Canvas for maps or custom visualizations. Legend is retained.
pub fn theme_void() -> Theme {
    theme_void_base(DEFAULT_BASE_SIZE)
}

/// Void theme with custom base font size.
pub fn theme_void_base(base_size: f64) -> Theme {
    let (title_size, _, axis_text_size) = text_sizes(base_size);
    Theme {
        text: ElementText::blank(),
        title: ElementText {
            size: title_size,
            ..Default::default()
        },
        axis_text_x: ElementText::blank(),
        axis_text_y: ElementText::blank(),
        axis_title_x: ElementText::blank(),
        axis_title_y: ElementText::blank(),
        axis_line: ElementLine::blank(),
        axis_ticks: ElementLine::blank(),
        panel_background: ElementRect::blank(),
        panel_grid_major: ElementLine::blank(),
        panel_grid_minor: ElementLine::blank(),
        plot_background: ElementRect::blank(),
        legend_position: LegendPosition::Right, // ggplot2 keeps legend in void
        plot_margin: Margin {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        },

        subtitle: ElementText::blank(),
        caption: ElementText::blank(),
        legend_title: ElementText {
            size: axis_text_size, // rel(0.8)
            ..Default::default()
        },
        legend_text: ElementText {
            size: axis_text_size, // rel(0.8)
            ..Default::default()
        },
        strip_text: ElementText {
            size: axis_text_size, // rel(0.8)
            ..Default::default()
        },

        axis_line_x: None,
        axis_line_y: None,
        axis_ticks_x: None,
        axis_ticks_y: None,
        panel_grid_major_x: None,
        panel_grid_major_y: None,
        panel_grid_minor_x: None,
        panel_grid_minor_y: None,

        panel_border: ElementLine::blank(),
        legend_background: ElementRect::blank(),
        legend_key: ElementRect::blank(),
        strip_background: ElementRect::blank(),

        axis_ticks_length: 0.0,
        axis_text_x_dodge: 1,
        legend_key_width: 12.0,
        legend_key_height: 18.0,
        legend_spacing: 4.0,
        legend_margin: Margin {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        },
        panel_spacing: 0.0,
        panel_spacing_x: None,
        panel_spacing_y: None,
        primary: None,
    }
}
