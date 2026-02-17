use crate::aes::Aesthetic;
use crate::data::Value;
use crate::render::backend::{DrawBackend, LineStyle, Linetype, RectStyle, TextAnchor, TextStyle};
use crate::render::{Rect, RenderError};
use crate::scale::ScaleSet;
use crate::theme::{LegendPosition, Theme};

/// Draw a legend for color/fill aesthetics.
pub fn draw_legend(
    scales: &ScaleSet,
    theme: &Theme,
    plot_area: &Rect,
    backend: &mut dyn DrawBackend,
) -> Result<(), RenderError> {
    if matches!(theme.legend_position, LegendPosition::None) {
        return Ok(());
    }

    // Check if there's a color or fill scale
    let color_scale = scales.get(&Aesthetic::Color);
    let fill_scale = scales.get(&Aesthetic::Fill);

    let (scale, aes) = if let Some(s) = color_scale {
        (s, Aesthetic::Color)
    } else if let Some(s) = fill_scale {
        (s, Aesthetic::Fill)
    } else {
        return Ok(());
    };

    if scale.is_discrete() {
        draw_discrete_legend(scales, &aes, scale, theme, plot_area, backend)
    } else {
        draw_continuous_legend(scales, &aes, scale, theme, plot_area, backend)
    }
}

/// Draw a discrete legend with color swatches.
fn draw_discrete_legend(
    scales: &ScaleSet,
    aes: &Aesthetic,
    scale: &dyn crate::scale::Scale,
    theme: &Theme,
    plot_area: &Rect,
    backend: &mut dyn DrawBackend,
) -> Result<(), RenderError> {
    let legend_items: Vec<(String, String)> = scale
        .breaks()
        .iter()
        .map(|(_, label)| (label.clone(), label.clone()))
        .collect();

    if legend_items.is_empty() {
        return Ok(());
    }

    let item_height = theme.legend_key_height;
    let swatch_size = theme.legend_key_width;
    let legend_x = plot_area.x + plot_area.width + theme.legend_margin.left;
    let legend_y = plot_area.y + theme.legend_margin.top;

    // Draw legend background
    if theme.legend_background.visible {
        let total_height = legend_items.len() as f64 * item_height;
        let total_width = swatch_size + theme.legend_spacing + theme.legend_text.size * 6.0;
        if let Some(fill) = theme.legend_background.fill {
            backend.draw_rect(
                (legend_x - 2.0, legend_y - 2.0),
                (legend_x + total_width + 2.0, legend_y + total_height + 2.0),
                &RectStyle {
                    fill: Some(fill),
                    stroke: theme.legend_background.color,
                    stroke_width: theme.legend_background.width,
                    alpha: 1.0,
                },
            )?;
        }
    }

    for (i, (value_key, label)) in legend_items.iter().enumerate() {
        let y = legend_y + i as f64 * item_height;

        let color = scales
            .map_color(aes, &Value::Str(value_key.clone()))
            .unwrap_or((127, 127, 127));

        // Legend key background
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
                    },
                )?;
            }
        }

        // Color swatch
        backend.draw_rect(
            (legend_x, y),
            (legend_x + swatch_size, y + swatch_size),
            &RectStyle {
                fill: Some(color),
                stroke: None,
                stroke_width: 0.0,
                alpha: 1.0,
            },
        )?;

        // Label
        backend.draw_text(
            label,
            (
                legend_x + swatch_size + theme.legend_spacing,
                y + swatch_size / 2.0,
            ),
            &TextStyle {
                color: theme.legend_text.color,
                size: theme.legend_text.size,
                anchor: TextAnchor::Start,
                angle: 0.0,
            },
        )?;
    }

    Ok(())
}

/// Draw a continuous colorbar legend (gradient bar with tick labels).
fn draw_continuous_legend(
    _scales: &ScaleSet,
    _aes: &Aesthetic,
    scale: &dyn crate::scale::Scale,
    theme: &Theme,
    plot_area: &Rect,
    backend: &mut dyn DrawBackend,
) -> Result<(), RenderError> {
    let breaks = scale.breaks();
    if breaks.is_empty() {
        return Ok(());
    }

    let bar_width = theme.legend_key_width;
    let bar_height = theme.legend_key_height * 8.0; // Taller bar for continuous
    let legend_x = plot_area.x + plot_area.width + theme.legend_margin.left;
    let legend_y = plot_area.y + theme.legend_margin.top;

    // Draw legend title (scale name)
    let scale_name = scale.name();
    let title_offset = if !scale_name.is_empty() {
        backend.draw_text(
            scale_name,
            (legend_x, legend_y),
            &TextStyle {
                color: theme.legend_title.color,
                size: theme.legend_title.size,
                anchor: TextAnchor::Start,
                angle: 0.0,
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
                },
            )?;
        }
    }

    // Draw gradient bar as N thin horizontal slices
    let n_slices = 50;
    let slice_height = bar_height / n_slices as f64;
    for i in 0..n_slices {
        // t goes from 1.0 (top = high) to 0.0 (bottom = low)
        let t = 1.0 - i as f64 / n_slices as f64;
        let color = scale
            .map_to_color(&Value::Float(t))
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
            },
        )?;
    }

    // Draw border around the bar
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

    // Draw tick marks and labels at break positions
    let tick_len = 3.0;
    for (pos, label) in &breaks {
        // pos is in [0, 1], where 0 = low (bottom), 1 = high (top)
        let tick_y = bar_top + bar_height * (1.0 - pos);

        // Tick mark
        backend.draw_line(
            &[
                (legend_x + bar_width, tick_y),
                (legend_x + bar_width + tick_len, tick_y),
            ],
            &border_style,
        )?;

        // Label
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
            },
        )?;
    }

    Ok(())
}
