use crate::coord::Coord;
use crate::render::backend::{DrawBackend, LineStyle, TextAnchor, TextStyle};
use crate::render::{Rect, RenderError};
use crate::scale::Scale;
use crate::theme::Theme;

/// Horizontal pixel for a break at normalized `pos` along the *horizontal* axis.
/// `coord_flip` swaps which normalized component drives horizontal position, so
/// route through the coord with the component that lands on x either way.
fn along_x(coord: &dyn Coord, pos: f64, area: &Rect) -> f64 {
    let p = if coord.is_flipped() {
        (0.0, pos)
    } else {
        (pos, 0.0)
    };
    coord.transform(p, area).0
}

/// Vertical pixel for a break at normalized `pos` along the *vertical* axis.
fn along_y(coord: &dyn Coord, pos: f64, area: &Rect) -> f64 {
    let p = if coord.is_flipped() {
        (pos, 0.0)
    } else {
        (0.0, pos)
    };
    coord.transform(p, area).1
}

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

    // Axis position: bottom (default) or top. `dir` points away from the panel.
    let top = scale.axis_position_opposite();
    let edge_y = if top {
        plot_area.y
    } else {
        plot_area.y + plot_area.height
    };
    let dir = if top { -1.0 } else { 1.0 };

    // Axis line
    if axis_line.visible {
        backend.draw_line(
            &[
                (plot_area.x, edge_y),
                (plot_area.x + plot_area.width, edge_y),
            ],
            &LineStyle {
                color: axis_line.color,
                width: axis_line.width,
                alpha: 1.0,
                linetype: axis_line.linetype,
            },
        )?;
    }

    // Ticks and labels
    let dodge = theme.axis_text_x_dodge.max(1);
    for (i, (pos, label)) in breaks.iter().enumerate() {
        let px = along_x(coord, *pos, plot_area);

        if axis_ticks.visible {
            backend.draw_line(
                &[(px, edge_y), (px, edge_y + dir * tick_len)],
                &LineStyle {
                    color: axis_ticks.color,
                    width: axis_ticks.width,
                    alpha: 1.0,
                    linetype: axis_ticks.linetype,
                },
            )?;
        }

        if theme.axis_text_x.visible {
            let family = if theme.axis_text_x.family.is_empty() {
                None
            } else {
                Some(theme.axis_text_x.family.clone())
            };
            // Adjust anchor for rotated labels
            let anchor = if theme.axis_text_x.angle.abs() > 10.0 {
                TextAnchor::End
            } else {
                TextAnchor::Middle
            };
            // Stagger labels across `dodge` rows to avoid overlap.
            let row_offset = (i % dodge) as f64 * (theme.axis_text_x.size + 2.0);
            backend.draw_text(
                label,
                (
                    px,
                    edge_y
                        + dir
                            * (tick_len
                                + theme.legend_spacing / 2.0
                                + theme.axis_text_x.size / 2.0
                                + row_offset),
                ),
                &TextStyle {
                    color: theme.axis_text_x.color,
                    size: theme.axis_text_x.size,
                    anchor,
                    angle: theme.axis_text_x.angle,
                    family,
                    face: theme.axis_text_x.face,
                },
            )?;
        }
    }

    // Minor ticks between majors.
    if theme.axis_minor_ticks && axis_ticks.visible {
        for pos in minor_breaks(&breaks) {
            let px = along_x(coord, pos, plot_area);
            backend.draw_line(
                &[(px, edge_y), (px, edge_y + dir * tick_len * 0.5)],
                &LineStyle {
                    color: axis_ticks.color,
                    width: axis_ticks.width,
                    alpha: 1.0,
                    linetype: axis_ticks.linetype,
                },
            )?;
        }
    }

    // Axis title
    let title = scale.name();
    if !title.is_empty() && theme.axis_title_x.visible {
        let hjust = theme.axis_title_x.hjust.clamp(0.0, 1.0);
        let center_x = plot_area.x + hjust * plot_area.width;
        let anchor = if hjust <= 0.02 {
            TextAnchor::Start
        } else if hjust >= 0.98 {
            TextAnchor::End
        } else {
            TextAnchor::Middle
        };
        let title_y = edge_y
            + dir * (tick_len + theme.axis_text_x.size + 8.0 + theme.axis_title_x.size / 2.0);
        let family = if theme.axis_title_x.family.is_empty() {
            None
        } else {
            Some(theme.axis_title_x.family.clone())
        };
        backend.draw_text(
            title,
            (center_x, title_y),
            &TextStyle {
                color: theme.axis_title_x.color,
                size: theme.axis_title_x.size,
                anchor,
                angle: 0.0,
                family,
                face: theme.axis_title_x.face,
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
                linetype: axis_line.linetype,
            },
        )?;
    }

    // Ticks and labels
    for (pos, label) in &breaks {
        let py = along_y(coord, *pos, plot_area);

        if axis_ticks.visible {
            backend.draw_line(
                &[(plot_area.x - tick_len, py), (plot_area.x, py)],
                &LineStyle {
                    color: axis_ticks.color,
                    width: axis_ticks.width,
                    alpha: 1.0,
                    linetype: axis_ticks.linetype,
                },
            )?;
        }

        if theme.axis_text_y.visible {
            let family = if theme.axis_text_y.family.is_empty() {
                None
            } else {
                Some(theme.axis_text_y.family.clone())
            };
            backend.draw_text(
                label,
                (plot_area.x - tick_len - theme.legend_spacing, py),
                &TextStyle {
                    color: theme.axis_text_y.color,
                    size: theme.axis_text_y.size,
                    anchor: TextAnchor::End,
                    angle: theme.axis_text_y.angle,
                    family,
                    face: theme.axis_text_y.face,
                },
            )?;
        }
    }

    // Minor ticks between majors.
    if theme.axis_minor_ticks && axis_ticks.visible {
        for pos in minor_breaks(&breaks) {
            let py = along_y(coord, pos, plot_area);
            backend.draw_line(
                &[(plot_area.x - tick_len * 0.5, py), (plot_area.x, py)],
                &LineStyle {
                    color: axis_ticks.color,
                    width: axis_ticks.width,
                    alpha: 1.0,
                    linetype: axis_ticks.linetype,
                },
            )?;
        }
    }

    // Axis title
    let title = scale.name();
    if !title.is_empty() && theme.axis_title_y.visible {
        let title_x = plot_area.x - tick_len - theme.axis_text_y.size * 3.5 - theme.legend_spacing;
        let center_y = plot_area.y + plot_area.height / 2.0;
        let family = if theme.axis_title_y.family.is_empty() {
            None
        } else {
            Some(theme.axis_title_y.family.clone())
        };
        backend.draw_text(
            title,
            (title_x, center_y),
            &TextStyle {
                color: theme.axis_title_y.color,
                size: theme.axis_title_y.size,
                anchor: TextAnchor::Middle,
                angle: 270.0,
                family,
                face: theme.axis_title_y.face,
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
                linetype: axis_line.linetype,
            },
        )?;
    }

    // Ticks and labels at primary break positions, but with transformed labels
    for (pos, label) in &breaks {
        let py = along_y(coord, *pos, plot_area);

        if axis_ticks.visible {
            backend.draw_line(
                &[(right_x, py), (right_x + tick_len, py)],
                &LineStyle {
                    color: axis_ticks.color,
                    width: axis_ticks.width,
                    alpha: 1.0,
                    linetype: axis_ticks.linetype,
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

            let family = if theme.axis_text_y.family.is_empty() {
                None
            } else {
                Some(theme.axis_text_y.family.clone())
            };
            backend.draw_text(
                &sec_label,
                (right_x + tick_len + theme.legend_spacing, py),
                &TextStyle {
                    color: theme.axis_text_y.color,
                    size: theme.axis_text_y.size,
                    anchor: TextAnchor::Start,
                    angle: theme.axis_text_y.angle,
                    family,
                    face: theme.axis_text_y.face,
                },
            )?;
        }
    }

    // Secondary axis title
    if !sec_axis.name.is_empty() && theme.axis_title_y.visible {
        let title_x = right_x + tick_len + theme.axis_text_y.size * 3.5 + theme.legend_spacing;
        let center_y = plot_area.y + plot_area.height / 2.0;
        let family = if theme.axis_title_y.family.is_empty() {
            None
        } else {
            Some(theme.axis_title_y.family.clone())
        };
        backend.draw_text(
            &sec_axis.name,
            (title_x, center_y),
            &TextStyle {
                color: theme.axis_title_y.color,
                size: theme.axis_title_y.size,
                anchor: TextAnchor::Middle,
                angle: 90.0,
                family,
                face: theme.axis_title_y.face,
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
            let px = along_x(coord, pos, plot_area);
            backend.draw_line(
                &[(px, plot_area.y), (px, plot_area.y + plot_area.height)],
                &LineStyle {
                    color: minor_x.color,
                    width: minor_x.width,
                    alpha: 1.0,
                    linetype: minor_x.linetype,
                },
            )?;
        }
    }

    // Minor Y gridlines (horizontal)
    if minor_y.visible {
        for pos in minor_breaks(&y_breaks) {
            let py = along_y(coord, pos, plot_area);
            backend.draw_line(
                &[(plot_area.x, py), (plot_area.x + plot_area.width, py)],
                &LineStyle {
                    color: minor_y.color,
                    width: minor_y.width,
                    alpha: 1.0,
                    linetype: minor_y.linetype,
                },
            )?;
        }
    }

    // Major X gridlines (vertical)
    if major_x.visible {
        for (pos, _) in &x_breaks {
            let px = along_x(coord, *pos, plot_area);
            backend.draw_line(
                &[(px, plot_area.y), (px, plot_area.y + plot_area.height)],
                &LineStyle {
                    color: major_x.color,
                    width: major_x.width,
                    alpha: 1.0,
                    linetype: major_x.linetype,
                },
            )?;
        }
    }

    // Major Y gridlines (horizontal)
    if major_y.visible {
        for (pos, _) in &y_breaks {
            let py = along_y(coord, *pos, plot_area);
            backend.draw_line(
                &[(plot_area.x, py), (plot_area.x + plot_area.width, py)],
                &LineStyle {
                    color: major_y.color,
                    width: major_y.width,
                    alpha: 1.0,
                    linetype: major_y.linetype,
                },
            )?;
        }
    }

    Ok(())
}
