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

/// Extra layout reservations beyond the theme-driven defaults (B3-1/B3-2):
/// additional left/right panel margin to keep outer x-axis labels on-canvas
/// under [`AxisLabelFit::Reserve`](crate::theme::AxisLabelFit::Reserve), and
/// extra legend rows from the B3-2 auto-wrap/`nrow`/`ncol` planner. All-zero
/// (the `Default`) reproduces `compute_full`'s own byte-identical math.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct LayoutExtras {
    pub left: f64,
    pub right: f64,
    pub legend_extra_rows: usize,
    /// Extra legend band width (B3-2, review #158 finding 3): a vertical
    /// explicit grid with `ncol > 1` needs `(cols − 1) · col_pitch` beyond
    /// the single-column `legend_size` estimate, or its later columns run
    /// off the canvas.
    pub legend_extra_width: f64,
    /// #236: the largest mapped `size` legend key's diameter, across every
    /// `size`-mapped scale being drawn — `0.0` (the default) when there's
    /// no size legend, or its largest key already fits the theme's
    /// `legend_key_width`/`legend_key_height`. `legend_size`/`legend_band`
    /// grow the key box (and the band reserved for it) to this instead of
    /// the flat theme default, the same way `draw_discrete_legend_at`
    /// grows the key it actually draws — otherwise a key too big for the
    /// reserved band spills into the panel.
    pub legend_max_key_size: f64,
    /// #211: how much taller the Top/Bottom legend band must be, beyond
    /// the flat discrete-row height (`key_h`) `legend_band` already
    /// assumes, to fit a horizontally-drawn continuous colourbar's
    /// title/bar/ticks/labels (see
    /// [`continuous_horizontal_bar_height`](crate::guide::legend::continuous_horizontal_bar_height)).
    /// `0.0` (the default) when there's no continuous color/fill legend
    /// drawn horizontally, or its own band already fits in `key_h`.
    pub continuous_horizontal_legend_extra_height: f64,
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
        Self::compute_full(
            width, height, theme, has_title, false, false, has_legend, false, None, None,
        )
    }

    /// Compute layout with full subtitle/caption support.
    ///
    /// `y_label_width` optionally overrides the reserved width (px) of the
    /// y-axis tick-label text — measured from the actual break labels by the
    /// caller so long categorical labels (e.g. a flipped bar chart's rows)
    /// aren't clipped. `None` keeps the fixed `axis_text_y.size * 3.5` estimate.
    #[allow(clippy::too_many_arguments)]
    pub fn compute_full(
        width: f64,
        height: f64,
        theme: &Theme,
        has_title: bool,
        has_subtitle: bool,
        has_caption: bool,
        has_legend: bool,
        x_axis_top: bool,
        y_label_width: Option<f64>,
        x_label_height: Option<f64>,
    ) -> Self {
        Self::compute_with_extras(
            width,
            height,
            theme,
            has_title,
            has_subtitle,
            has_caption,
            has_legend,
            x_axis_top,
            y_label_width,
            x_label_height,
            LayoutExtras::default(),
        )
    }

    /// Height the layout reserves for the bottom x axis (ticks, labels,
    /// title); `x_label_height` is the measured rotated-label height, if any.
    pub(crate) fn x_axis_band(theme: &Theme, x_label_height: Option<f64>) -> f64 {
        theme.axis_ticks_length
            + if theme.axis_text_x.visible {
                // Rotated labels extend vertically, so reserve more bottom space —
                // measured from the actual labels by the caller when available.
                if theme.axis_text_x.angle.abs() > 10.0 {
                    x_label_height.unwrap_or(theme.axis_text_x.size * 5.0)
                } else {
                    // Dodged labels stack across N rows.
                    (theme.axis_text_x.size + 4.0) * theme.axis_text_x_dodge.max(1) as f64
                }
            } else {
                0.0
            }
            + if theme.axis_title_x.visible {
                theme.axis_title_x.size + 8.0
            } else {
                0.0
            }
    }

    /// Like [`compute_full`](Self::compute_full), plus [`LayoutExtras`] for
    /// the outer-label-fit left/right panel reservation (B3-1) and the
    /// legend auto-wrap extra rows (B3-2). `extras` all-zero is byte-identical
    /// to `compute_full`.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn compute_with_extras(
        width: f64,
        height: f64,
        theme: &Theme,
        has_title: bool,
        has_subtitle: bool,
        has_caption: bool,
        has_legend: bool,
        x_axis_top: bool,
        y_label_width: Option<f64>,
        x_label_height: Option<f64>,
        extras: LayoutExtras,
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

        let x_axis_height = Self::x_axis_band(theme, x_label_height);

        let y_axis_width = theme.axis_ticks_length
            + if theme.axis_text_y.visible {
                y_label_width.unwrap_or(theme.axis_text_y.size * 3.5 + 4.0)
            } else {
                0.0
            }
            + if theme.axis_title_y.visible {
                theme.axis_title_y.size + 8.0
            } else {
                0.0
            };

        // #236: a mapped `size` legend's largest key can need a bigger box
        // than the theme's flat defaults — grow the key (and so the band
        // reserved for it) to fit, the same way `draw_discrete_legend_at`
        // grows the key it actually draws. A no-op (both `.max()`s keep
        // the theme defaults) when there's no size legend.
        let key_w = theme.legend_key_width.max(extras.legend_max_key_size);
        let key_h = theme.legend_key_height.max(key_w);

        let legend_size = if has_legend {
            theme.legend_margin.left
                + key_w
                + theme.legend_spacing
                + theme.legend_text.size * 6.0
                + theme.legend_margin.right
                + extras.legend_extra_width
        } else {
            0.0
        };

        // A horizontal (top/bottom) legend only needs ONE row of height, not the
        // width-based `legend_size` — otherwise a big gap opens above the plot.
        let legend_band = if has_legend {
            theme.legend_margin.top
                + key_h
                // #211: a horizontally-drawn continuous colourbar's own
                // title/bar/ticks/labels can need more than the flat
                // discrete-row height above — added once, not per extra
                // row (a continuous colourbar never wraps rows).
                + extras.continuous_horizontal_legend_extra_height
                + theme.legend_spacing
                + theme.legend_margin.bottom
                + extras.legend_extra_rows as f64 * (key_h + theme.legend_spacing)
        } else {
            0.0
        };
        // Determine legend space allocation per position
        let (legend_right, legend_left, legend_top, legend_bottom) = if has_legend {
            match theme.legend_position {
                LegendPosition::Right => (legend_size, 0.0, 0.0, 0.0),
                LegendPosition::Left => (0.0, legend_size, 0.0, 0.0),
                LegendPosition::Top => (0.0, 0.0, legend_band, 0.0),
                LegendPosition::Bottom => (0.0, 0.0, 0.0, legend_band),
                // Inside/None overlay the panel, reserving no external space.
                LegendPosition::None | LegendPosition::Inside(..) => (0.0, 0.0, 0.0, 0.0),
            }
        } else {
            (0.0, 0.0, 0.0, 0.0)
        };

        let plot_x = margin.left + y_axis_width + legend_left + extras.left;
        // A top x-axis reserves its space above the panel instead of below.
        let plot_y = margin.top
            + title_height
            + subtitle_height
            + legend_top
            + if x_axis_top { x_axis_height } else { 0.0 };
        let plot_width = width
            - margin.left
            - margin.right
            - y_axis_width
            - legend_right
            - legend_left
            - extras.left
            - extras.right;
        let plot_height = height
            - margin.top
            - margin.bottom
            - title_height
            - subtitle_height
            - caption_height
            - x_axis_height
            - legend_top
            - legend_bottom;

        let mut plot_width = plot_width.max(50.0);
        let mut plot_height = plot_height.max(50.0);
        let mut plot_x = plot_x;
        let mut plot_y = plot_y;

        // Fix the panel aspect ratio (R's `aspect.ratio` = height/width),
        // shrinking the larger dimension and centering within the available box.
        if let Some(r) = theme.aspect_ratio {
            if r > 0.0 {
                let (avail_w, avail_h) = (plot_width, plot_height);
                if avail_w * r <= avail_h {
                    plot_height = avail_w * r;
                    plot_y += (avail_h - plot_height) / 2.0;
                } else {
                    plot_width = avail_h / r;
                    plot_x += (avail_w - plot_width) / 2.0;
                }
            }
        }

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::presets::theme_gray;

    /// `compute_full` is kept as a public byte-identical shim over
    /// `compute_with_extras(..., LayoutExtras::default())` (B3-1) — no
    /// longer called internally (`layout_built` calls `compute_with_extras`
    /// directly), but still part of the public API.
    #[test]
    fn compute_full_matches_compute_with_extras_default() {
        let theme = theme_gray();
        let a = PlotLayout::compute_full(
            640.0, 400.0, &theme, true, false, false, true, false, None, None,
        );
        let b = PlotLayout::compute_with_extras(
            640.0,
            400.0,
            &theme,
            true,
            false,
            false,
            true,
            false,
            None,
            None,
            LayoutExtras::default(),
        );
        assert_eq!(a.plot_area.x, b.plot_area.x);
        assert_eq!(a.plot_area.width, b.plot_area.width);
        assert_eq!(a.plot_area.height, b.plot_area.height);
    }

    #[test]
    fn compute_delegates_to_compute_full() {
        let theme = theme_gray();
        let a = PlotLayout::compute(640.0, 400.0, &theme, true, false);
        let b = PlotLayout::compute_full(
            640.0, 400.0, &theme, true, false, false, false, false, None, None,
        );
        assert_eq!(a.plot_area.x, b.plot_area.x);
    }
}
