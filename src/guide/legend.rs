use crate::aes::Aesthetic;
use crate::data::Value;
use crate::guide::config::GuideLegend;
use crate::render::backend::{
    DrawBackend, LineStyle, Linetype, PointStyle, RectStyle, TextAnchor, TextStyle,
};
use crate::render::{Rect, RenderError};
use crate::scale::ScaleSet;
use crate::theme::{LegendPosition, Theme};

/// Which aesthetics should generate legends.
const LEGEND_AESTHETICS: &[Aesthetic] = &[
    Aesthetic::Color,
    Aesthetic::Fill,
    Aesthetic::Shape,
    Aesthetic::Linetype,
    Aesthetic::Size,
    Aesthetic::Alpha,
];

/// Draw all legends for the plot.
pub fn draw_legend(
    scales: &ScaleSet,
    theme: &Theme,
    plot_area: &Rect,
    backend: &mut dyn DrawBackend,
    guide: &GuideLegend,
    suppressed: &std::collections::HashSet<Aesthetic>,
) -> Result<(), RenderError> {
    if matches!(theme.legend_position, LegendPosition::None) {
        return Ok(());
    }

    // Collect all aesthetics that have a scale with breaks
    let mut legend_scales: Vec<&Aesthetic> = Vec::new();
    for aes in LEGEND_AESTHETICS {
        // Skip suppressed aesthetics
        if suppressed.contains(aes) {
            continue;
        }
        if let Some(scale) = scales.get(aes) {
            if !scale.breaks().is_empty() {
                // Don't duplicate Color/Fill if both exist with same breaks
                if *aes == Aesthetic::Fill && legend_scales.contains(&&Aesthetic::Color) {
                    continue;
                }
                legend_scales.push(aes);
            }
        }
    }

    if legend_scales.is_empty() {
        return Ok(());
    }

    // Compute legend origin based on position
    let (legend_x, legend_y, mut is_horizontal) = legend_position(theme, plot_area);
    // legend.direction overrides the auto layout from the position.
    if let Some(dir) = theme.legend_direction {
        is_horizontal = matches!(dir, crate::theme::LegendDirection::Horizontal);
    }

    let mut offset_y = legend_y;
    let mut offset_x = legend_x;

    for aes in &legend_scales {
        let scale = scales.get(aes).unwrap();

        if scale.is_discrete() {
            if is_horizontal {
                let width = draw_discrete_legend_at(
                    scales, aes, scale, theme, offset_x, offset_y, backend, guide,
                )?;
                offset_x += width + theme.legend_spacing * 2.0;
            } else {
                let height = draw_discrete_legend_at(
                    scales, aes, scale, theme, offset_x, offset_y, backend, guide,
                )?;
                offset_y += height + theme.legend_spacing * 2.0;
            }
        } else {
            // Continuous legend (colorbar) — only for color/fill
            if matches!(aes, Aesthetic::Color | Aesthetic::Fill) {
                let height =
                    draw_continuous_legend_at(scale, theme, offset_x, offset_y, backend, guide)?;
                if is_horizontal {
                    offset_x += theme.legend_key_width
                        + theme.legend_text.size * 6.0
                        + theme.legend_spacing * 2.0;
                } else {
                    offset_y += height + theme.legend_spacing * 2.0;
                }
            } else {
                // Continuous size/alpha — draw as discrete-like with sampled breaks
                let height = draw_discrete_legend_at(
                    scales, aes, scale, theme, offset_x, offset_y, backend, guide,
                )?;
                if is_horizontal {
                    offset_x += theme.legend_key_width
                        + theme.legend_text.size * 6.0
                        + theme.legend_spacing * 2.0;
                } else {
                    offset_y += height + theme.legend_spacing * 2.0;
                }
            }
        }
    }

    Ok(())
}

/// Compute legend origin based on position setting.
/// Returns (x, y, is_horizontal).
fn legend_position(theme: &Theme, plot_area: &Rect) -> (f64, f64, bool) {
    match theme.legend_position {
        LegendPosition::Right => (
            plot_area.x + plot_area.width + theme.legend_margin.left,
            plot_area.y + theme.legend_margin.top,
            false,
        ),
        LegendPosition::Left => (
            theme.legend_margin.left,
            plot_area.y + theme.legend_margin.top,
            false,
        ),
        LegendPosition::Top => (
            plot_area.x + theme.legend_margin.left,
            theme.legend_margin.top,
            true,
        ),
        LegendPosition::Bottom => (
            plot_area.x + theme.legend_margin.left,
            plot_area.y + plot_area.height + theme.legend_margin.top + 30.0,
            true,
        ),
        LegendPosition::None => (0.0, 0.0, false),
        LegendPosition::Inside(fx, fy) => (
            plot_area.x + fx * plot_area.width,
            plot_area.y + (1.0 - fy) * plot_area.height,
            false,
        ),
    }
}

/// Draw a discrete legend at a given position. Returns the height used.
#[allow(clippy::too_many_arguments)]
fn draw_discrete_legend_at(
    scales: &ScaleSet,
    aes: &Aesthetic,
    scale: &dyn crate::scale::Scale,
    theme: &Theme,
    legend_x: f64,
    legend_y: f64,
    backend: &mut dyn DrawBackend,
    guide: &GuideLegend,
) -> Result<f64, RenderError> {
    let mut breaks = scale.breaks();
    if breaks.is_empty() {
        return Ok(0.0);
    }

    // Apply guide reverse
    if guide.reverse {
        breaks.reverse();
    }

    let item_height = theme.legend_key_height;
    let swatch_size = theme.legend_key_width;

    // Draw legend title (guide title overrides scale name)
    let title = guide.title.as_deref().unwrap_or_else(|| scale.name());
    let legend_family = if theme.legend_title.family.is_empty() {
        None
    } else {
        Some(theme.legend_title.family.clone())
    };
    let title_offset = if !title.is_empty() {
        backend.draw_text(
            title,
            (legend_x, legend_y),
            &TextStyle {
                color: theme.legend_title.color,
                size: theme.legend_title.size,
                anchor: TextAnchor::Start,
                angle: 0.0,
                family: legend_family,
                face: theme.legend_title.face,
            },
        )?;
        theme.legend_title.size + 4.0
    } else {
        0.0
    };

    let items_y = legend_y + title_offset;

    // Draw legend background
    if theme.legend_background.visible {
        let total_height = breaks.len() as f64 * item_height;
        let total_width = swatch_size + theme.legend_spacing + theme.legend_text.size * 6.0;
        if let Some(fill) = theme.legend_background.fill {
            backend.draw_rect(
                (legend_x - 2.0, items_y - 2.0),
                (legend_x + total_width + 2.0, items_y + total_height + 2.0),
                &RectStyle {
                    fill: Some(fill),
                    stroke: theme.legend_background.color,
                    stroke_width: theme.legend_background.width,
                    alpha: 1.0,
                    clip: false,
                },
            )?;
        }
    }

    for (i, (_, label)) in breaks.iter().enumerate() {
        let y = items_y + i as f64 * item_height;
        let center_x = legend_x + swatch_size / 2.0;
        let center_y = y + swatch_size / 2.0;

        // Draw legend key background
        if theme.legend_key.visible {
            if let Some(fill) = theme.legend_key.fill {
                backend.draw_rect(
                    (legend_x, y),
                    (legend_x + swatch_size, y + swatch_size),
                    &RectStyle {
                        fill: Some(fill),
                        stroke: theme.legend_key.color,
                        stroke_width: theme.legend_key.width,
                        alpha: 1.0,
                        clip: false,
                    },
                )?;
            }
        }

        // Draw the appropriate swatch based on aesthetic type
        let value = Value::Str(label.clone());
        match aes {
            Aesthetic::Color | Aesthetic::Fill => {
                let color = scales.map_color(aes, &value).unwrap_or((127, 127, 127));
                backend.draw_rect(
                    (legend_x, y),
                    (legend_x + swatch_size, y + swatch_size),
                    &RectStyle {
                        fill: Some(color),
                        stroke: None,
                        stroke_width: 0.0,
                        alpha: 1.0,
                        clip: false,
                    },
                )?;
            }
            Aesthetic::Shape => {
                let shape = scales
                    .map_shape(&value)
                    .unwrap_or(crate::render::backend::PointShape::Circle);
                backend.draw_shape(
                    (center_x, center_y),
                    swatch_size / 3.0,
                    &PointStyle {
                        color: (50, 50, 50),
                        alpha: 1.0,
                        filled: true,
                        shape,
                    },
                )?;
            }
            Aesthetic::Linetype => {
                let lt = scales.map_linetype(&value).unwrap_or(Linetype::Solid);
                backend.draw_line(
                    &[
                        (legend_x + 2.0, center_y),
                        (legend_x + swatch_size - 2.0, center_y),
                    ],
                    &LineStyle {
                        color: (50, 50, 50),
                        width: 1.5,
                        alpha: 1.0,
                        linetype: lt,
                    },
                )?;
            }
            Aesthetic::Size => {
                // For size, show varying circle sizes
                let size = scales.map_size(&value).unwrap_or(3.0);
                backend.draw_shape(
                    (center_x, center_y),
                    size.min(swatch_size / 2.0),
                    &PointStyle {
                        color: (50, 50, 50),
                        alpha: 1.0,
                        filled: true,
                        shape: crate::render::backend::PointShape::Circle,
                    },
                )?;
            }
            Aesthetic::Alpha => {
                let alpha = scales.map_alpha(&value).unwrap_or(1.0);
                backend.draw_rect(
                    (legend_x, y),
                    (legend_x + swatch_size, y + swatch_size),
                    &RectStyle {
                        fill: Some((50, 50, 50)),
                        stroke: None,
                        stroke_width: 0.0,
                        alpha,
                        clip: false,
                    },
                )?;
            }
            _ => {}
        }

        // Label
        let label_family = if theme.legend_text.family.is_empty() {
            None
        } else {
            Some(theme.legend_text.family.clone())
        };
        backend.draw_text(
            label,
            (legend_x + swatch_size + theme.legend_spacing, center_y),
            &TextStyle {
                color: theme.legend_text.color,
                size: theme.legend_text.size,
                anchor: TextAnchor::Start,
                angle: 0.0,
                family: label_family,
                face: theme.legend_text.face,
            },
        )?;
    }

    Ok(title_offset + breaks.len() as f64 * item_height)
}

/// Draw a continuous colorbar legend at a given position. Returns the height used.
fn draw_continuous_legend_at(
    scale: &dyn crate::scale::Scale,
    theme: &Theme,
    legend_x: f64,
    legend_y: f64,
    backend: &mut dyn DrawBackend,
    guide: &GuideLegend,
) -> Result<f64, RenderError> {
    let breaks = scale.breaks();
    if breaks.is_empty() {
        return Ok(0.0);
    }

    let bar_width = theme.legend_key_width;
    let bar_height = theme.legend_key_height * 8.0;

    // Draw legend title (guide title overrides scale name)
    let title = guide.title.as_deref().unwrap_or_else(|| scale.name());
    let cont_family = if theme.legend_title.family.is_empty() {
        None
    } else {
        Some(theme.legend_title.family.clone())
    };
    let title_offset = if !title.is_empty() {
        backend.draw_text(
            title,
            (legend_x, legend_y),
            &TextStyle {
                color: theme.legend_title.color,
                size: theme.legend_title.size,
                anchor: TextAnchor::Start,
                angle: 0.0,
                family: cont_family,
                face: theme.legend_title.face,
            },
        )?;
        theme.legend_title.size + 4.0
    } else {
        0.0
    };

    let bar_top = legend_y + title_offset;

    // Draw legend background
    if theme.legend_background.visible {
        let total_width = bar_width + theme.legend_spacing + theme.legend_text.size * 6.0;
        if let Some(fill) = theme.legend_background.fill {
            backend.draw_rect(
                (legend_x - 2.0, bar_top - 2.0),
                (legend_x + total_width + 2.0, bar_top + bar_height + 2.0),
                &RectStyle {
                    fill: Some(fill),
                    stroke: theme.legend_background.color,
                    stroke_width: theme.legend_background.width,
                    alpha: 1.0,
                    clip: false,
                },
            )?;
        }
    }

    // Draw gradient bar as N thin horizontal slices
    // Use data-domain values to avoid double-normalization in map_to_color()
    let (data_min, data_max) = scale.domain().unwrap_or((0.0, 1.0));
    let n_slices = 50;
    let slice_height = bar_height / n_slices as f64;
    for i in 0..n_slices {
        let t = 1.0 - i as f64 / n_slices as f64;
        let data_val = data_min + t * (data_max - data_min);
        let color = scale
            .map_to_color(&Value::Float(data_val))
            .unwrap_or((127, 127, 127));
        let sy = bar_top + i as f64 * slice_height;
        backend.draw_rect(
            (legend_x, sy),
            (legend_x + bar_width, sy + slice_height + 0.5),
            &RectStyle {
                fill: Some(color),
                stroke: None,
                stroke_width: 0.0,
                alpha: 1.0,
                clip: false,
            },
        )?;
    }

    // Draw border
    let border_style = LineStyle {
        color: theme.legend_key.color.unwrap_or((50, 50, 50)),
        width: 0.5,
        alpha: 1.0,
        linetype: Linetype::Solid,
    };
    backend.draw_line(
        &[
            (legend_x, bar_top),
            (legend_x + bar_width, bar_top),
            (legend_x + bar_width, bar_top + bar_height),
            (legend_x, bar_top + bar_height),
            (legend_x, bar_top),
        ],
        &border_style,
    )?;

    // Draw tick marks and labels
    let tick_len = 3.0;
    for (pos, label) in &breaks {
        let tick_y = bar_top + bar_height * (1.0 - pos);
        backend.draw_line(
            &[
                (legend_x + bar_width, tick_y),
                (legend_x + bar_width + tick_len, tick_y),
            ],
            &border_style,
        )?;
        let tick_family = if theme.legend_text.family.is_empty() {
            None
        } else {
            Some(theme.legend_text.family.clone())
        };
        backend.draw_text(
            label,
            (
                legend_x + bar_width + tick_len + theme.legend_spacing,
                tick_y,
            ),
            &TextStyle {
                color: theme.legend_text.color,
                size: theme.legend_text.size,
                anchor: TextAnchor::Start,
                angle: 0.0,
                family: tick_family,
                face: theme.legend_text.face,
            },
        )?;
    }

    Ok(title_offset + bar_height)
}
