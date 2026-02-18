use crate::theme::{LegendPosition, Theme};

use super::Rect;

/// Computed layout areas for the plot.
pub struct PlotLayout {
    pub total: Rect,
    pub plot_area: Rect,
    pub title_area: Rect,
    pub subtitle_area: Rect,
    pub caption_area: Rect,
    pub x_axis_area: Rect,
    pub y_axis_area: Rect,
    pub legend_area: Rect,
}

impl PlotLayout {
    /// Compute layout from total dimensions and theme settings.
    pub fn compute(
        width: f64,
        height: f64,
        theme: &Theme,
        has_title: bool,
        has_legend: bool,
    ) -> Self {
        Self::compute_full(width, height, theme, has_title, false, false, has_legend)
    }

    /// Compute layout with full subtitle/caption support.
    pub fn compute_full(
        width: f64,
        height: f64,
        theme: &Theme,
        has_title: bool,
        has_subtitle: bool,
        has_caption: bool,
        has_legend: bool,
    ) -> Self {
        let margin = &theme.plot_margin;

        let title_height = if has_title {
            theme.title.size * 2.0
        } else {
            0.0
        };

        let subtitle_height = if has_subtitle {
            theme.subtitle.size * 1.5
        } else {
            0.0
        };

        let caption_height = if has_caption {
            theme.caption.size * 1.8
        } else {
            0.0
        };

        let x_axis_height = theme.axis_ticks_length
            + if theme.axis_text_x.visible {
                theme.axis_text_x.size + 4.0
            } else {
                0.0
            }
            + if theme.axis_title_x.visible {
                theme.axis_title_x.size + 8.0
            } else {
                0.0
            };

        let y_axis_width = theme.axis_ticks_length
            + if theme.axis_text_y.visible {
                theme.axis_text_y.size * 3.5 + 4.0
            } else {
                0.0
            }
            + if theme.axis_title_y.visible {
                theme.axis_title_y.size + 8.0
            } else {
                0.0
            };

        let legend_size = if has_legend {
            theme.legend_margin.left
                + theme.legend_key_width
                + theme.legend_spacing
                + theme.legend_text.size * 6.0
                + theme.legend_margin.right
        } else {
            0.0
        };

        // Determine legend space allocation per position
        let (legend_right, legend_left, legend_top, legend_bottom) = if has_legend {
            match theme.legend_position {
                LegendPosition::Right => (legend_size, 0.0, 0.0, 0.0),
                LegendPosition::Left => (0.0, legend_size, 0.0, 0.0),
                LegendPosition::Top => (0.0, 0.0, legend_size, 0.0),
                LegendPosition::Bottom => (0.0, 0.0, 0.0, legend_size),
                LegendPosition::None => (0.0, 0.0, 0.0, 0.0),
            }
        } else {
            (0.0, 0.0, 0.0, 0.0)
        };

        let plot_x = margin.left + y_axis_width + legend_left;
        let plot_y = margin.top + title_height + subtitle_height + legend_top;
        let plot_width =
            width - margin.left - margin.right - y_axis_width - legend_right - legend_left;
        let plot_height = height
            - margin.top
            - margin.bottom
            - title_height
            - subtitle_height
            - caption_height
            - x_axis_height
            - legend_top
            - legend_bottom;

        let plot_width = plot_width.max(50.0);
        let plot_height = plot_height.max(50.0);

        PlotLayout {
            total: Rect {
                x: 0.0,
                y: 0.0,
                width,
                height,
            },
            plot_area: Rect {
                x: plot_x,
                y: plot_y,
                width: plot_width,
                height: plot_height,
            },
            title_area: Rect {
                x: plot_x,
                y: margin.top,
                width: plot_width,
                height: title_height,
            },
            subtitle_area: Rect {
                x: plot_x,
                y: margin.top + title_height,
                width: plot_width,
                height: subtitle_height,
            },
            caption_area: Rect {
                x: plot_x,
                y: plot_y + plot_height + x_axis_height,
                width: plot_width,
                height: caption_height,
            },
            x_axis_area: Rect {
                x: plot_x,
                y: plot_y + plot_height,
                width: plot_width,
                height: x_axis_height,
            },
            y_axis_area: Rect {
                x: margin.left + legend_left,
                y: plot_y,
                width: y_axis_width,
                height: plot_height,
            },
            legend_area: Rect {
                x: plot_x + plot_width,
                y: plot_y,
                width: legend_size,
                height: plot_height,
            },
        }
    }
}
