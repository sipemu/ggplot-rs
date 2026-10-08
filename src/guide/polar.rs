//! Guides for radar coordinates: rings at the y breaks, spokes at the x
//! breaks, and labels around the outside.

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
            linetype: grid.linetype,
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
            linetype: spoke.linetype,
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
                },
            )?;
        }
    }
    Ok(())
}
