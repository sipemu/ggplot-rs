use crate::coord::Coord;
use crate::render::backend::{DrawBackend, LineStyle, Linetype, TextAnchor, TextStyle};
use crate::render::{Rect, RenderError};
use crate::scale::Scale;
use crate::theme::Theme;

/// Draw the X axis: ticks, labels, and title.
pub fn draw_x_axis(
    scale: &dyn Scale,
    coord: &dyn Coord,
    theme: &Theme,
    plot_area: &Rect,
    backend: &mut dyn DrawBackend,
) -> Result<(), RenderError> {
    let breaks = scale.breaks();
    let tick_len = theme.axis_ticks_length;
    let axis_line = theme.get_axis_line_x();
    let axis_ticks = theme.get_axis_ticks_x();

    // Axis line
    if axis_line.visible {
        let left = (plot_area.x, plot_area.y + plot_area.height);
        let right = (
            plot_area.x + plot_area.width,
            plot_area.y + plot_area.height,
        );
        backend.draw_line(
            &[left, right],
            &LineStyle {
                color: axis_line.color,
                width: axis_line.width,
                alpha: 1.0,
                linetype: Linetype::Solid,
            },
        )?;
    }

    // Ticks and labels
    for (pos, label) in &breaks {
        let (px, _py) = coord.transform((*pos, 0.0), plot_area);
        let tick_y = plot_area.y + plot_area.height;

        if axis_ticks.visible {
            backend.draw_line(
                &[(px, tick_y), (px, tick_y + tick_len)],
                &LineStyle {
                    color: axis_ticks.color,
                    width: axis_ticks.width,
                    alpha: 1.0,
                    linetype: Linetype::Solid,
                },
            )?;
        }

        if theme.axis_text_x.visible {
            backend.draw_text(
                label,
                (
                    px,
                    tick_y + tick_len + theme.legend_spacing / 2.0 + theme.axis_text_x.size / 2.0,
                ),
                &TextStyle {
                    color: theme.axis_text_x.color,
                    size: theme.axis_text_x.size,
                    anchor: TextAnchor::Middle,
                    angle: theme.axis_text_x.angle,
                },
            )?;
        }
    }

    // Axis title
    let title = scale.name();
    if !title.is_empty() && theme.axis_title_x.visible {
        let center_x = plot_area.x + plot_area.width / 2.0;
        let title_y = plot_area.y
            + plot_area.height
            + tick_len
            + theme.axis_text_x.size
            + 8.0
            + theme.axis_title_x.size / 2.0;
        backend.draw_text(
            title,
            (center_x, title_y),
            &TextStyle {
                color: theme.axis_title_x.color,
                size: theme.axis_title_x.size,
                anchor: TextAnchor::Middle,
                angle: 0.0,
            },
        )?;
    }

    Ok(())
}

/// Draw the Y axis: ticks, labels, and title.
pub fn draw_y_axis(
    scale: &dyn Scale,
    coord: &dyn Coord,
    theme: &Theme,
    plot_area: &Rect,
    backend: &mut dyn DrawBackend,
) -> Result<(), RenderError> {
    let breaks = scale.breaks();
    let tick_len = theme.axis_ticks_length;
    let axis_line = theme.get_axis_line_y();
    let axis_ticks = theme.get_axis_ticks_y();

    // Axis line
    if axis_line.visible {
        let top = (plot_area.x, plot_area.y);
        let bottom = (plot_area.x, plot_area.y + plot_area.height);
        backend.draw_line(
            &[top, bottom],
            &LineStyle {
                color: axis_line.color,
                width: axis_line.width,
                alpha: 1.0,
                linetype: Linetype::Solid,
            },
        )?;
    }

    // Ticks and labels
    for (pos, label) in &breaks {
        let (_, py) = coord.transform((0.0, *pos), plot_area);

        if axis_ticks.visible {
            backend.draw_line(
                &[(plot_area.x - tick_len, py), (plot_area.x, py)],
                &LineStyle {
                    color: axis_ticks.color,
                    width: axis_ticks.width,
                    alpha: 1.0,
                    linetype: Linetype::Solid,
                },
            )?;
        }

        if theme.axis_text_y.visible {
            backend.draw_text(
                label,
                (plot_area.x - tick_len - theme.legend_spacing, py),
                &TextStyle {
                    color: theme.axis_text_y.color,
                    size: theme.axis_text_y.size,
                    anchor: TextAnchor::End,
                    angle: 0.0,
                },
            )?;
        }
    }

    // Axis title
    let title = scale.name();
    if !title.is_empty() && theme.axis_title_y.visible {
        let title_x = plot_area.x - tick_len - theme.axis_text_y.size * 3.5 - theme.legend_spacing;
        let center_y = plot_area.y + plot_area.height / 2.0;
        backend.draw_text(
            title,
            (title_x, center_y),
            &TextStyle {
                color: theme.axis_title_y.color,
                size: theme.axis_title_y.size,
                anchor: TextAnchor::Middle,
                angle: 270.0,
            },
        )?;
    }

    Ok(())
}

/// Draw a secondary Y axis on the right side.
pub fn draw_sec_y_axis(
    primary_scale: &dyn Scale,
    sec_axis: &crate::scale::sec_axis::SecAxis,
    coord: &dyn Coord,
    theme: &Theme,
    plot_area: &Rect,
    backend: &mut dyn DrawBackend,
) -> Result<(), RenderError> {
    let breaks = primary_scale.breaks();
    let tick_len = theme.axis_ticks_length;
    let axis_line = theme.get_axis_line_y();
    let axis_ticks = theme.get_axis_ticks_y();

    let right_x = plot_area.x + plot_area.width;

    // Axis line on right side
    if axis_line.visible {
        backend.draw_line(
            &[
                (right_x, plot_area.y),
                (right_x, plot_area.y + plot_area.height),
            ],
            &LineStyle {
                color: axis_line.color,
                width: axis_line.width,
                alpha: 1.0,
                linetype: Linetype::Solid,
            },
        )?;
    }

    // Ticks and labels at primary break positions, but with transformed labels
    for (pos, label) in &breaks {
        let (_, py) = coord.transform((0.0, *pos), plot_area);

        if axis_ticks.visible {
            backend.draw_line(
                &[(right_x, py), (right_x + tick_len, py)],
                &LineStyle {
                    color: axis_ticks.color,
                    width: axis_ticks.width,
                    alpha: 1.0,
                    linetype: Linetype::Solid,
                },
            )?;
        }

        if theme.axis_text_y.visible {
            // Parse the primary label back to a number, transform it
            let sec_label = if let Ok(v) = label.parse::<f64>() {
                let transformed = sec_axis.transform_value(v);
                crate::scale::util::format_number(transformed)
            } else {
                label.clone()
            };

            backend.draw_text(
                &sec_label,
                (right_x + tick_len + theme.legend_spacing, py),
                &TextStyle {
                    color: theme.axis_text_y.color,
                    size: theme.axis_text_y.size,
                    anchor: TextAnchor::Start,
                    angle: 0.0,
                },
            )?;
        }
    }

    // Secondary axis title
    if !sec_axis.name.is_empty() && theme.axis_title_y.visible {
        let title_x = right_x + tick_len + theme.axis_text_y.size * 3.5 + theme.legend_spacing;
        let center_y = plot_area.y + plot_area.height / 2.0;
        backend.draw_text(
            &sec_axis.name,
            (title_x, center_y),
            &TextStyle {
                color: theme.axis_title_y.color,
                size: theme.axis_title_y.size,
                anchor: TextAnchor::Middle,
                angle: 90.0,
            },
        )?;
    }

    Ok(())
}

/// Compute minor break positions as midpoints between major breaks.
fn minor_breaks(major: &[(f64, String)]) -> Vec<f64> {
    if major.len() < 2 {
        return vec![];
    }
    let mut minors = Vec::with_capacity(major.len() - 1);
    for pair in major.windows(2) {
        minors.push((pair[0].0 + pair[1].0) / 2.0);
    }
    minors
}

/// Draw gridlines for both axes.
pub fn draw_gridlines(
    x_scale: &dyn Scale,
    y_scale: &dyn Scale,
    coord: &dyn Coord,
    theme: &Theme,
    plot_area: &Rect,
    backend: &mut dyn DrawBackend,
) -> Result<(), RenderError> {
    let major_x = theme.get_panel_grid_major_x();
    let major_y = theme.get_panel_grid_major_y();
    let minor_x = theme.get_panel_grid_minor_x();
    let minor_y = theme.get_panel_grid_minor_y();

    let x_breaks = x_scale.breaks();
    let y_breaks = y_scale.breaks();

    // Minor X gridlines (vertical) — drawn first so majors paint over them
    if minor_x.visible {
        for pos in minor_breaks(&x_breaks) {
            let (px, _) = coord.transform((pos, 0.0), plot_area);
            backend.draw_line(
                &[(px, plot_area.y), (px, plot_area.y + plot_area.height)],
                &LineStyle {
                    color: minor_x.color,
                    width: minor_x.width,
                    alpha: 1.0,
                    linetype: Linetype::Solid,
                },
            )?;
        }
    }

    // Minor Y gridlines (horizontal)
    if minor_y.visible {
        for pos in minor_breaks(&y_breaks) {
            let (_, py) = coord.transform((0.0, pos), plot_area);
            backend.draw_line(
                &[(plot_area.x, py), (plot_area.x + plot_area.width, py)],
                &LineStyle {
                    color: minor_y.color,
                    width: minor_y.width,
                    alpha: 1.0,
                    linetype: Linetype::Solid,
                },
            )?;
        }
    }

    // Major X gridlines (vertical)
    if major_x.visible {
        for (pos, _) in &x_breaks {
            let (px, _) = coord.transform((*pos, 0.0), plot_area);
            backend.draw_line(
                &[(px, plot_area.y), (px, plot_area.y + plot_area.height)],
                &LineStyle {
                    color: major_x.color,
                    width: major_x.width,
                    alpha: 1.0,
                    linetype: Linetype::Solid,
                },
            )?;
        }
    }

    // Major Y gridlines (horizontal)
    if major_y.visible {
        for (pos, _) in &y_breaks {
            let (_, py) = coord.transform((0.0, *pos), plot_area);
            backend.draw_line(
                &[(plot_area.x, py), (plot_area.x + plot_area.width, py)],
                &LineStyle {
                    color: major_y.color,
                    width: major_y.width,
                    alpha: 1.0,
                    linetype: Linetype::Solid,
                },
            )?;
        }
    }

    Ok(())
}
