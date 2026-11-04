//! Guides for radar and polar coordinates: rings at the radius breaks,
//! spokes at the angle breaks, and labels around the outside.

use crate::coord::Coord;
use crate::render::backend::{DrawBackend, LineStyle, TextAnchor, TextStyle};
use crate::render::{Rect, RenderError};
use crate::scale::Scale;
use crate::theme::Theme;

/// Draw radar rings, spokes and labels (instead of cartesian gridlines/axes).
pub fn draw_radar_guides(
    x_scale: &dyn Scale,
    y_scale: &dyn Scale,
    coord: &dyn Coord,
    theme: &Theme,
    area: &Rect,
    backend: &mut dyn DrawBackend,
) -> Result<(), RenderError> {
    let x_breaks = x_scale.breaks();
    let y_breaks = y_scale.breaks();
    // Spoke positions: the x breaks; a continuous x gets a smooth ring.
    let ring_pos: Vec<f64> = if x_scale.is_discrete() && x_breaks.len() >= 3 {
        x_breaks.iter().map(|(p, _)| *p).collect()
    } else {
        (0..72).map(|k| k as f64 / 72.0).collect()
    };
    let centre = coord.transform((0.0, f64::NEG_INFINITY), area);

    let grid = theme.get_panel_grid_major_y();
    if grid.visible {
        let style = LineStyle {
            color: grid.color,
            width: grid.width,
            alpha: 1.0,
            linetype: grid.linetype.clone(),
            ..Default::default()
        };
        for (ny, _) in &y_breaks {
            if !(0.0..=1.0 + 1e-9).contains(ny) {
                continue;
            }
            let mut ring: Vec<(f64, f64)> = ring_pos
                .iter()
                .map(|&p| coord.transform((p, *ny), area))
                .collect();
            if let Some(&first) = ring.first() {
                ring.push(first);
            }
            backend.draw_line(&ring, &style)?;
        }
    }
    let spoke = theme.get_panel_grid_major_x();
    if spoke.visible {
        let style = LineStyle {
            color: spoke.color,
            width: spoke.width,
            alpha: 1.0,
            linetype: spoke.linetype.clone(),
            ..Default::default()
        };
        for (p, _) in &x_breaks {
            backend.draw_line(&[centre, coord.transform((*p, 1.0), area)], &style)?;
        }
    }

    let family = |f: &str| (!f.is_empty()).then(|| f.to_string());
    if theme.axis_text_x.visible {
        for (p, label) in &x_breaks {
            let outer = coord.transform((*p, 1.0), area);
            let (dx, dy) = (outer.0 - centre.0, outer.1 - centre.1);
            let len = (dx * dx + dy * dy).sqrt().max(1e-9);
            let pad = theme.axis_text_x.size * 0.9;
            let pos = (outer.0 + dx / len * pad, outer.1 + dy / len * pad);
            let anchor = if dx.abs() < len * 0.1 {
                TextAnchor::Middle
            } else if dx > 0.0 {
                TextAnchor::Start
            } else {
                TextAnchor::End
            };
            backend.draw_text(
                label,
                pos,
                &TextStyle {
                    color: theme.axis_text_x.color,
                    size: theme.axis_text_x.size,
                    anchor,
                    angle: 0.0,
                    family: family(&theme.axis_text_x.family),
                    face: theme.axis_text_x.face,
                    bg_colour: None,
                    bg_r: 0.0,
                },
            )?;
        }
    }
    if theme.axis_text_y.visible {
        // Ring values along the first spoke, nudged right of it.
        let p0 = x_breaks.first().map(|(p, _)| *p).unwrap_or(0.0);
        for (ny, label) in &y_breaks {
            if !(0.0..=1.0 + 1e-9).contains(ny) {
                continue;
            }
            let (x, y) = coord.transform((p0, *ny), area);
            backend.draw_text(
                label,
                (x + 3.0, y),
                &TextStyle {
                    color: theme.axis_text_y.color,
                    size: theme.axis_text_y.size * 0.85,
                    anchor: TextAnchor::Start,
                    angle: 0.0,
                    family: family(&theme.axis_text_y.family),
                    face: theme.axis_text_y.face,
                    bg_colour: None,
                    bg_r: 0.0,
                },
            )?;
        }
    }
    Ok(())
}

/// Draw `coord_polar` / `coord_radial` guides (#411): rings at the r
/// breaks (major y grid style), spokes at the theta breaks (major x grid),
/// theta labels just outside the circle (turned along it with
/// `rotate_angle`) and r labels at the panel's left edge, or along the start
/// spoke for `r_axis_inside` (the default for partial arcs).
pub fn draw_polar_guides(
    x_scale: &dyn Scale,
    y_scale: &dyn Scale,
    coord: &dyn Coord,
    theme: &Theme,
    area: &Rect,
    backend: &mut dyn DrawBackend,
) -> Result<(), RenderError> {
    let Some(g) = coord.polar_guide(area) else {
        return Ok(());
    };
    let theta_x = coord.polar_theta_is_x();
    let (theta_scale, r_scale) = if theta_x {
        (x_scale, y_scale)
    } else {
        (y_scale, x_scale)
    };
    // A point at normalised (theta, r).
    let at = |t: f64, r: f64| {
        if theta_x {
            coord.transform((t, r), area)
        } else {
            coord.transform((r, t), area)
        }
    };
    // Breaks beyond the swept range would wrap round onto others.
    let mut theta_breaks: Vec<(f64, String)> = theta_scale
        .breaks()
        .into_iter()
        .filter(|(p, _)| (-1e-9..=1.0 + 1e-9).contains(p))
        .collect();
    // A full circle's last break sits on its first: keep one label there.
    if g.full_circle
        && theta_breaks.len() > 1
        && theta_breaks.first().is_some_and(|(p, _)| p.abs() < 1e-9)
    {
        theta_breaks.retain(|(p, _)| (*p - 1.0).abs() > 1e-9);
    }
    let r_breaks: Vec<(f64, String)> = r_scale
        .breaks()
        .into_iter()
        .filter(|(p, _)| (-1e-9..=1.0 + 1e-9).contains(p))
        .collect();
    let line = |el: &crate::theme::elements::ElementLine| LineStyle {
        color: el.color,
        width: el.width,
        alpha: 1.0,
        linetype: el.linetype.clone(),
        ..Default::default()
    };

    let ring = theme.get_panel_grid_major_y();
    if ring.visible {
        let n = 96;
        for (r, _) in &r_breaks {
            let pts: Vec<(f64, f64)> = (0..=n).map(|k| at(k as f64 / n as f64, *r)).collect();
            backend.draw_line(&pts, &line(ring))?;
        }
    }
    let spoke = theme.get_panel_grid_major_x();
    if spoke.visible {
        for (t, _) in &theta_breaks {
            backend.draw_line(&[at(*t, 0.0), at(*t, 1.0)], &line(spoke))?;
        }
    }

    let family = |f: &str| (!f.is_empty()).then(|| f.to_string());
    if theme.axis_text_x.visible {
        let el = &theme.axis_text_x;
        for (t, label) in &theta_breaks {
            let outer = at(*t, 1.0);
            let (dx, dy) = (outer.0 - g.centre.0, outer.1 - g.centre.1);
            let len = (dx * dx + dy * dy).sqrt().max(1e-9);
            let pad = el.size * 0.9;
            let pos = (outer.0 + dx / len * pad, outer.1 + dy / len * pad);
            let (anchor, angle) = if g.rotate_labels {
                // Tangent to the circle, kept upright.
                let mut a = dy.atan2(dx).to_degrees() + 90.0;
                if a > 90.0 && a < 270.0 {
                    a -= 180.0;
                }
                (TextAnchor::Middle, a)
            } else if dx.abs() < len * 0.1 {
                (TextAnchor::Middle, 0.0)
            } else if dx > 0.0 {
                (TextAnchor::Start, 0.0)
            } else {
                (TextAnchor::End, 0.0)
            };
            backend.draw_text(
                label,
                pos,
                &TextStyle {
                    color: el.color,
                    size: el.size,
                    anchor,
                    angle,
                    family: family(&el.family),
                    face: el.face,
                    bg_colour: None,
                    bg_r: 0.0,
                },
            )?;
        }
    }
    if theme.axis_text_y.visible {
        let el = &theme.axis_text_y;
        for (r, label) in &r_breaks {
            let (pos, anchor) = if g.r_axis_inside {
                // Along the start spoke, just off it.
                let p = at(0.0, *r);
                ((p.0 - 3.0, p.1), TextAnchor::End)
            } else {
                // At the panel's left edge, level with the ring's top.
                let top = g.centre.1 - (g.centre.1 - at(0.0, *r).1).abs();
                (
                    (area.x - theme.axis_ticks_length - 2.0, top),
                    TextAnchor::End,
                )
            };
            backend.draw_text(
                label,
                pos,
                &TextStyle {
                    color: el.color,
                    size: el.size,
                    anchor,
                    angle: 0.0,
                    family: family(&el.family),
                    face: el.face,
                    bg_colour: None,
                    bg_r: 0.0,
                },
            )?;
        }
    }
    Ok(())
}
