use crate::coord::Coord;
use crate::render::backend::{DrawBackend, LineStyle, TextAnchor, TextStyle};
use crate::render::text::{axis_label_priority, label_bbox, text_width};
use crate::render::{Rect, RenderError};
use crate::scale::Scale;
use crate::theme::{AxisLabelFit, Theme};

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
///
/// `canvas` is the full SVG viewport (not just the panel): B3-1's outer-
/// label fit drops a label whose tick has scrolled off it, and shifts a
/// label bbox that crosses it back inside, unless
/// `theme.axis_text_x_fit` is [`AxisLabelFit::Clip`] (the 0.21 behaviour —
/// no correction). Ticks and gridlines are drawn regardless.
pub fn draw_x_axis(
    scale: &dyn Scale,
    coord: &dyn Coord,
    theme: &Theme,
    plot_area: &Rect,
    canvas: &Rect,
    backend: &mut dyn DrawBackend,
) -> Result<(), RenderError> {
    draw_x_axis_with(scale, coord, theme, plot_area, canvas, true, backend)
}

/// [`draw_x_axis`], optionally without its title — a faceted grid draws one
/// title for all panels instead (#348).
pub(crate) fn draw_x_axis_with(
    scale: &dyn Scale,
    coord: &dyn Coord,
    theme: &Theme,
    plot_area: &Rect,
    canvas: &Rect,
    with_title: bool,
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
                linetype: axis_line.linetype.clone(),
                ..Default::default()
            },
        )?;
    }

    // Precompute, per break, whether/where its label is drawn — but keep a
    // single loop below in the *original* per-break (tick, label) order, so
    // a plot whose labels all fit and don't overlap renders byte-identical
    // SVG to 0.21 (same element order), even though the decision logic ran.
    let label_draw: Vec<Option<f64>> = if theme.axis_text_x.visible {
        let angle = theme.axis_text_x.angle;
        let rotated = angle.abs() > 10.0;
        let anchor_frac = if rotated {
            crate::render::text::rotated_anchor_frac(angle, dir)
        } else {
            0.5
        };
        let fit = theme.axis_text_x_fit;
        let canvas_lo = canvas.x;
        let canvas_hi = canvas.x + canvas.width;
        const EDGE: f64 = 1.0;
        let size = theme.axis_text_x.size;

        // (index, px, label) for breaks whose tick is on-canvas. A label
        // whose tick has scrolled off the canvas names nothing that is
        // shown — unconditional, even under `Clip`.
        let onscreen: Vec<(usize, f64, &str)> = breaks
            .iter()
            .enumerate()
            .map(|(i, (pos, label))| (i, along_x(coord, *pos, plot_area), label.as_str()))
            .filter(|(_, px, _)| (canvas_lo..=canvas_hi).contains(px))
            .collect();

        // ggplot2's `check.overlap`: draw in priority order (first, last,
        // then recursive midpoint bisection — D5), skipping any label that
        // would overlap one already kept.
        let mut keep = vec![true; onscreen.len()];
        if theme.axis_text_x_check_overlap && onscreen.len() > 1 {
            let n = onscreen.len();
            let mut kept: Vec<(f64, f64, f64)> = Vec::with_capacity(n); // (x0, x2, px)
            keep = vec![false; n];
            // A minimum legible gap between two *kept* labels, not just "the
            // bboxes don't literally touch" (sipemu/ggplot-rs#212): a label-
            // width estimate close enough to a font's real metrics (`size`'s
            // own table is tuned for one house font) can leave adjacent
            // labels a pixel or two apart on paper but visually touching
            // once actually rendered — a run like "25" next to "50" then
            // reads as the single garbled "2550". ggplot2's own
            // check.overlap has no such margin (a print figure can afford a
            // tight fit); this is for an interactive dashboard, where a
            // readable gap matters more than keeping one extra label. One
            // space character's width is a natural, already-tabulated unit
            // for "a visually clear gap" — not print-figure kerning, just
            // enough that two numbers never read as one.
            let min_gap = text_width(" ", size);
            for p in axis_label_priority(n) {
                let idx = p - 1; // 1-indexed in the ggplot2 sense
                let (_, px, label) = onscreen[idx];
                let bbox = label_bbox(label, size, px, 0.0, angle, anchor_frac);
                let overlaps = kept.iter().any(|&(kx0, kx2, kpx)| {
                    if !rotated {
                        bbox[0] < kx2 + min_gap && bbox[2] > kx0 - min_gap
                    } else {
                        // Parallel rotated labels on one baseline: the
                        // perpendicular distance across the text direction
                        // must leave room for a line of text.
                        (px - kpx).abs() * angle.to_radians().sin().abs() < size
                    }
                });
                if !overlaps {
                    keep[idx] = true;
                    kept.push((bbox[0], bbox[2], px));
                }
            }
        }

        let mut draw = vec![None; breaks.len()];
        for (k, &(i, px, label)) in onscreen.iter().enumerate() {
            if !keep[k] {
                continue;
            }
            // Shift a bbox that crosses the canvas edge back inside (moving
            // the rotate centre too), unless `fit` is `Clip` (0.21 behaviour).
            let lx = if fit == AxisLabelFit::Clip {
                px
            } else {
                // `ly` doesn't affect the x-bbox; 0.0 is a placeholder.
                let bbox = label_bbox(label, size, px, 0.0, angle, anchor_frac);
                let room_hi = canvas_hi - EDGE;
                let room_lo = canvas_lo + EDGE;
                if bbox[0] < room_lo {
                    px + (room_lo - bbox[0])
                } else if bbox[2] > room_hi {
                    px - (bbox[2] - room_hi).min(bbox[0] - room_lo)
                } else {
                    px
                }
            };
            draw[i] = Some(lx);
        }
        draw
    } else {
        Vec::new()
    };

    // Ticks and labels, interleaved in break order (ticks always stay).
    let dodge = theme.axis_text_x_dodge.max(1);
    let family = if theme.axis_text_x.family.is_empty() {
        None
    } else {
        Some(theme.axis_text_x.family.clone())
    };
    let rotated = theme.axis_text_x.angle.abs() > 10.0;
    // Which end of the text pins to the tick, so the near-axis end stays
    // there instead of the whole label sliding away (sipemu/ggplot-rs#163)
    // — depends on both the rotation's direction and which side of the
    // axis line the panel is on (`dir`); see `rotated_anchor_frac`.
    let draw_anchor_frac = if rotated {
        crate::render::text::rotated_anchor_frac(theme.axis_text_x.angle, dir)
    } else {
        0.5
    };
    let anchor = if !rotated {
        TextAnchor::Middle
    } else if draw_anchor_frac >= 1.0 {
        TextAnchor::End
    } else {
        TextAnchor::Start
    };
    // A rotated label's far end swings back *toward* the panel on one side
    // and away from it on the other (sipemu/ggplot-rs#162) — toward the
    // panel is `rise` for a bottom axis (`dir > 0`, the panel sits above
    // the axis line) and `fall` for a top axis (`dir < 0`, the panel sits
    // below it). Reserving only `rotated_label_rise_fall`'s sum (as
    // `layout_built` does, for the *other* side's on-canvas extent) isn't
    // enough on its own: this offset must actually place the rotation
    // anchor that far from the axis line, or a long enough label swings
    // right back over the panel no matter how much height was reserved.
    let rotation_margin = if rotated {
        let (rise, fall) = crate::render::text::rotated_label_rise_fall(
            breaks
                .iter()
                .filter(|(_, l)| !l.is_empty())
                .map(|(_, l)| l.as_str()),
            theme.axis_text_x.size,
            theme.axis_text_x.angle,
            draw_anchor_frac,
        );
        if dir > 0.0 {
            rise
        } else {
            fall
        }
    } else {
        0.0
    };
    for (i, (pos, label)) in breaks.iter().enumerate() {
        let px = along_x(coord, *pos, plot_area);

        if axis_ticks.visible {
            backend.draw_line(
                &[(px, edge_y), (px, edge_y + dir * tick_len)],
                &LineStyle {
                    color: axis_ticks.color,
                    width: axis_ticks.width,
                    alpha: 1.0,
                    linetype: axis_ticks.linetype.clone(),
                    ..Default::default()
                },
            )?;
        }

        if let Some(Some(lx)) = label_draw.get(i) {
            // Stagger labels across `dodge` rows to avoid overlap.
            let row_offset = (i % dodge) as f64 * (theme.axis_text_x.size + 2.0);
            let ly = edge_y
                + dir
                    * (tick_len
                        + theme.legend_spacing / 2.0
                        + theme.axis_text_x.size / 2.0
                        + row_offset
                        + rotation_margin);
            backend.draw_text(
                label,
                (*lx, ly),
                &TextStyle {
                    color: theme.axis_text_x.color,
                    size: theme.axis_text_x.size,
                    anchor: anchor.clone(),
                    angle: theme.axis_text_x.angle,
                    family: family.clone(),
                    face: theme.axis_text_x.face,
                    bg_colour: None,
                    bg_r: 0.0,
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
                    linetype: axis_ticks.linetype.clone(),
                    ..Default::default()
                },
            )?;
        }
    }

    if with_title {
        draw_x_axis_title(scale, theme, plot_area, backend)?;
    }

    Ok(())
}

/// The x axis title, placed against `area`'s bottom (or top, for an
/// opposite axis) edge and spanning its width.
pub(crate) fn draw_x_axis_title(
    scale: &dyn Scale,
    theme: &Theme,
    area: &Rect,
    backend: &mut dyn DrawBackend,
) -> Result<(), RenderError> {
    let plot_area = area;
    let tick_len = theme.axis_ticks_length;
    let top = scale.axis_position_opposite();
    let edge_y = if top {
        plot_area.y
    } else {
        plot_area.y + plot_area.height
    };
    let dir = if top { -1.0 } else { 1.0 };
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
                bg_colour: None,
                bg_r: 0.0,
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
    draw_y_axis_with(scale, coord, theme, plot_area, true, backend)
}

/// [`draw_y_axis`], optionally without its title — a faceted grid draws one
/// title for all panels instead (#348).
pub(crate) fn draw_y_axis_with(
    scale: &dyn Scale,
    coord: &dyn Coord,
    theme: &Theme,
    plot_area: &Rect,
    with_title: bool,
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
                linetype: axis_line.linetype.clone(),
                ..Default::default()
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
                    linetype: axis_ticks.linetype.clone(),
                    ..Default::default()
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
                    bg_colour: None,
                    bg_r: 0.0,
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
                    linetype: axis_ticks.linetype.clone(),
                    ..Default::default()
                },
            )?;
        }
    }

    if with_title {
        let reserve = y_label_reserve(theme, &breaks);
        draw_y_axis_title(scale, theme, plot_area, reserve, backend)?;
    }

    Ok(())
}

/// Gap between the y tick labels and the y axis title (#348).
const Y_TITLE_GAP: f64 = 4.0;

/// Width the layout reserves for the y tick labels with these `breaks`
/// (estimated from their length; never below the theme default).
pub(crate) fn y_label_reserve(theme: &Theme, breaks: &[(f64, String)]) -> f64 {
    let max_ch = breaks
        .iter()
        .map(|(_, l)| l.chars().count())
        .max()
        .unwrap_or(0);
    let measured = max_ch as f64 * theme.axis_text_y.size * 0.6 + 4.0;
    let default = theme.axis_text_y.size * 3.5 + 4.0;
    measured.max(default)
}

/// The y axis title, left of `area`'s tick labels (`label_reserve` wide)
/// and centred on its height.
pub(crate) fn draw_y_axis_title(
    scale: &dyn Scale,
    theme: &Theme,
    area: &Rect,
    label_reserve: f64,
    backend: &mut dyn DrawBackend,
) -> Result<(), RenderError> {
    let title = scale.name();
    if !title.is_empty() && theme.axis_title_y.visible {
        // The rotated (270°) title's glyphs span roughly [x − 0.75·size,
        // x + 0.25·size]; keep its right edge a fixed gap (ggplot2's
        // 2.75pt title margin) left of the tick-label column the layout
        // reserved — never overlapping short labels — and on the canvas.
        let size = theme.axis_title_y.size;
        let right = area.x - theme.axis_ticks_length - label_reserve - Y_TITLE_GAP;
        let min_x = backend.total_area().x + 0.75 * size;
        let title_x = (right - 0.25 * size).max(min_x);
        let center_y = area.y + area.height / 2.0;
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
                bg_colour: None,
                bg_r: 0.0,
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
    let own_ticks = sec_axis.own_ticks(primary_scale);
    let breaks = own_ticks.clone().unwrap_or_else(|| primary_scale.breaks());
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
                linetype: axis_line.linetype.clone(),
                ..Default::default()
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
                    linetype: axis_ticks.linetype.clone(),
                    ..Default::default()
                },
            )?;
        }

        if theme.axis_text_y.visible {
            // Parse the primary label back to a number, transform it
            let locale = primary_scale.locale();
            let parsed = match locale {
                Some(l) => l.parse_number(label),
                None => label.parse::<f64>().ok(),
            };
            let sec_label = if own_ticks.is_some() {
                label.clone()
            } else if let Some(v) = parsed {
                let transformed = sec_axis.transform_value(v);
                crate::scale::util::format_number_opt(transformed, locale)
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
                    bg_colour: None,
                    bg_r: 0.0,
                },
            )?;
        }
    }

    // Secondary axis title
    if !sec_axis.name.is_empty() && theme.axis_title_y.visible {
        // Clear the widest tick label (own percent ticks can exceed the
        // 3.5-character default).
        let max_ch = own_ticks
            .as_ref()
            .and_then(|t| t.iter().map(|(_, l)| l.chars().count()).max())
            .unwrap_or(0);
        let text_w =
            (max_ch as f64 * theme.axis_text_y.size * 0.6).max(theme.axis_text_y.size * 3.5);
        let title_x = right_x + tick_len + text_w + theme.legend_spacing;
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
                bg_colour: None,
                bg_r: 0.0,
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
                    linetype: minor_x.linetype.clone(),
                    ..Default::default()
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
                    linetype: minor_y.linetype.clone(),
                    ..Default::default()
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
                    linetype: major_x.linetype.clone(),
                    ..Default::default()
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
                    linetype: major_y.linetype.clone(),
                    ..Default::default()
                },
            )?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aes::Aesthetic;
    use crate::coord::cartesian::CoordCartesian;
    use crate::data::Value;
    use crate::render::svg_backend::SvgBackend;
    use crate::theme::presets::theme_gray;

    /// A scale with hand-picked break positions/labels, so a test can place
    /// one off the canvas without fighting a real break algorithm.
    struct FixedBreaksScale(Vec<(f64, String)>);

    impl Scale for FixedBreaksScale {
        fn aesthetic(&self) -> Aesthetic {
            Aesthetic::X
        }
        fn train(&mut self, _values: &[Value]) {}
        fn map(&self, _value: &Value) -> f64 {
            0.0
        }
        fn breaks(&self) -> Vec<(f64, String)> {
            self.0.clone()
        }
        fn name(&self) -> &str {
            ""
        }
        fn set_name(&mut self, _name: &str) {}
        fn clone_box(&self) -> Box<dyn Scale> {
            Box::new(FixedBreaksScale(self.0.clone()))
        }
    }

    fn backend(w: u32, h: u32, plot_area: Rect) -> SvgBackend {
        SvgBackend::new(w, h, plot_area)
    }

    #[test]
    fn a_tick_outside_the_svg_is_dropped() {
        // Normalized position 5.0 is nowhere near [0, 1]; through
        // `CoordCartesian` that lands far outside the 200×100 canvas.
        let scale = FixedBreaksScale(vec![
            (0.2, "kept".to_string()),
            (5.0, "off_canvas".to_string()),
        ]);
        let coord = CoordCartesian::new();
        let theme = theme_gray();
        let plot_area = Rect {
            x: 10.0,
            y: 10.0,
            width: 180.0,
            height: 70.0,
        };
        let canvas = Rect {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 100.0,
        };
        let mut backend = backend(200, 100, plot_area.clone());
        draw_x_axis(&scale, &coord, &theme, &plot_area, &canvas, &mut backend).unwrap();
        let svg = backend.finish();
        assert!(svg.contains(">kept<"), "on-canvas label dropped: {svg}");
        assert!(
            !svg.contains(">off_canvas<"),
            "off-canvas label was drawn: {svg}"
        );
    }

    #[test]
    fn clip_draws_the_label_exactly_at_the_tick_even_past_the_edge() {
        // A very wide label at the rightmost tick, Clip mode: no shift.
        let scale = FixedBreaksScale(vec![
            (0.0, "a".to_string()),
            (1.0, "2021-01-01".to_string()),
        ]);
        let coord = CoordCartesian::new();
        let mut theme = theme_gray();
        theme.axis_text_x_fit = crate::theme::AxisLabelFit::Clip;
        let plot_area = Rect {
            x: 0.0,
            y: 0.0,
            width: 60.0,
            height: 50.0,
        };
        let canvas = plot_area.clone();
        let mut backend = backend(60, 50, plot_area.clone());
        draw_x_axis(&scale, &coord, &theme, &plot_area, &canvas, &mut backend).unwrap();
        let svg = backend.finish();
        // Clip: the label sits exactly on the tick (x="60.00"), unmoved,
        // even though its bbox clearly overflows a 60px canvas.
        assert!(
            svg.contains("x=\"60.00\" y=\"") && svg.contains(">2021-01-01<"),
            "clip mode should not shift the label: {svg}"
        );
    }

    #[test]
    fn shift_moves_the_same_label_inside_the_canvas() {
        let scale = FixedBreaksScale(vec![
            (0.0, "a".to_string()),
            (1.0, "2021-01-01".to_string()),
        ]);
        let coord = CoordCartesian::new();
        let mut theme = theme_gray();
        theme.axis_text_x_fit = crate::theme::AxisLabelFit::Shift;
        let plot_area = Rect {
            x: 0.0,
            y: 0.0,
            width: 60.0,
            height: 50.0,
        };
        let canvas = plot_area.clone();
        let mut backend = backend(60, 50, plot_area.clone());
        draw_x_axis(&scale, &coord, &theme, &plot_area, &canvas, &mut backend).unwrap();
        let svg = backend.finish();
        assert!(
            !svg.contains("x=\"60.00\" y=\""),
            "shift mode should move the label off the bare tick: {svg}"
        );
        let w = crate::render::text::text_width("2021-01-01", theme.axis_text_x.size);
        // The shifted label's right edge must land inside the canvas.
        let needle = ">2021-01-01<";
        let tag_end = svg.find(needle).unwrap();
        let tag_start = svg[..tag_end].rfind("<text ").unwrap();
        let tag = &svg[tag_start..tag_end];
        let x_attr = tag.rsplit("x=\"").next().unwrap();
        let x: f64 = x_attr.split('"').next().unwrap().parse().unwrap();
        // `x` round-trips through a `"{:.2}"`-formatted SVG attribute, so
        // allow a hair of slack for that rounding.
        assert!(
            x + w / 2.0 <= 60.0 - 1.0 + 0.01,
            "right edge still overflows: x={x} w={w}"
        );
    }
}
