use crate::aes::Aesthetic;
use crate::data::Value;
use crate::render::backend::{DrawBackend, RectStyle, TextAnchor, TextStyle};
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

    // Check if there's a color or fill scale with breaks
    let (legend_items, legend_aes): (Vec<(String, String)>, Aesthetic) = {
        let color_scale = scales.get(&Aesthetic::Color);
        let fill_scale = scales.get(&Aesthetic::Fill);

        if let Some(s) = color_scale {
            if s.is_discrete() {
                let items: Vec<(String, String)> = s
                    .breaks()
                    .iter()
                    .map(|(_, label)| (label.clone(), label.clone()))
                    .collect();
                (items, Aesthetic::Color)
            } else {
                return Ok(());
            }
        } else if let Some(s) = fill_scale {
            if s.is_discrete() {
                let items: Vec<(String, String)> = s
                    .breaks()
                    .iter()
                    .map(|(_, label)| (label.clone(), label.clone()))
                    .collect();
                (items, Aesthetic::Fill)
            } else {
                return Ok(());
            }
        } else {
            return Ok(());
        }
    };

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
            .map_color(&legend_aes, &Value::Str(value_key.clone()))
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
