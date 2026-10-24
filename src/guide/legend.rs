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
    Aesthetic::Linewidth,
];

/// The aesthetics that would get a legend, in the same order and with the
/// same color/fill de-duplication `draw_legend` and `draw_legend_planned`
/// use. Shared with `layout_built` (B3-2), so the "is this the single
/// discrete guide D7 requires" check is the same one the drawing code
/// uses.
pub(crate) fn legend_scales_for(
    scales: &ScaleSet,
    suppressed: &std::collections::HashSet<Aesthetic>,
) -> Vec<Aesthetic> {
    let mut legend_scales: Vec<Aesthetic> = Vec::new();
    for aes in LEGEND_AESTHETICS {
        if suppressed.contains(aes) {
            continue;
        }
        if let Some(scale) = scales.get(aes) {
            if !scale.breaks().is_empty() {
                if *aes == Aesthetic::Fill && legend_scales.contains(&Aesthetic::Color) {
                    continue;
                }
                legend_scales.push(aes.clone());
            }
        }
    }
    legend_scales
}

/// Draw all legends for the plot.
///
/// No longer called internally: `compose::capture_legend` (the only
/// caller) was rerouted to `draw_legend_planned` in 0.23 so a `PlotGrid`'s
/// collected legend gets the same B3-2 wrap/grid treatment a single
/// plot's legend does (sipemu/ggplot-rs#155). `draw_legend_planned` is
/// crate-internal (`pub(crate)`), so there is no direct public
/// replacement — legend drawing is otherwise handled automatically by
/// `GGPlot`'s own render methods, which is the supported way to draw a
/// legend; this function was never meant to be called by consumers for
/// that purpose and will be removed in a later minor release.
#[deprecated(
    since = "0.23.0",
    note = "unused internally since 0.23 (superseded by the crate-internal draw_legend_planned, \
             sipemu/ggplot-rs#155); no direct public replacement — legends are drawn automatically \
             by GGPlot's render methods. Will be removed in a later minor release."
)]
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

    // Compute legend origin based on position. This deprecated path never
    // wraps a legend onto more than one row, so `legend_extra_rows` is
    // always 0 — see `legend_position`'s doc comment.
    let (legend_x, legend_y, mut is_horizontal) = legend_position(theme, plot_area, 0);
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
                    scales, aes, scale, theme, offset_x, offset_y, backend, guide, true,
                )?;
                offset_x += width + theme.legend_spacing * 2.0;
            } else {
                let height = draw_discrete_legend_at(
                    scales, aes, scale, theme, offset_x, offset_y, backend, guide, false,
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
                    scales,
                    aes,
                    scale,
                    theme,
                    offset_x,
                    offset_y,
                    backend,
                    guide,
                    is_horizontal,
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

/// How many rows beyond the first a horizontal legend wraps onto — `max`
/// across guides sharing one row (several discrete guides side by side),
/// `sum` for guides stacked vertically instead. Exactly `layout_built`'s
/// own `extras.legend_extra_rows` computation, so anything derived from
/// it here can't disagree with the `plot_area.y` that count already
/// produced.
fn legend_rows_beyond_first(
    is_horizontal: bool,
    legend_plans: &std::collections::HashMap<Aesthetic, LegendPlan>,
) -> usize {
    if is_horizontal {
        legend_plans
            .values()
            .map(|p| p.rows.saturating_sub(1))
            .max()
            .unwrap_or(0)
    } else {
        legend_plans
            .values()
            .map(|p| p.rows.saturating_sub(1))
            .sum()
    }
}

/// A horizontal (`Top`/`Bottom`) legend's own band height —
/// `legend_margin.top` plus `(1 + legend_extra_rows)` row-heights
/// (`legend_key_height + legend_spacing` each) plus `legend_margin.bottom`
/// — exactly what `PlotLayout::compute_with_extras` reserves for it
/// (#233).
pub(crate) fn legend_band_height(theme: &Theme, legend_extra_rows: usize) -> f64 {
    theme.legend_margin.top
        + (1.0 + legend_extra_rows as f64) * (theme.legend_key_height + theme.legend_spacing)
        + theme.legend_margin.bottom
}

/// #233: how much `renderer.rs`'s title/subtitle drawing must additionally
/// lift itself above `plot_area.y` so neither overlaps a `Top` legend —
/// the same idea as its existing `x_axis_lift` (a top x-axis also
/// reserves space above the panel). Zero when there's no legend to draw,
/// or its position isn't `Top`: byte-identical to before in every other
/// case.
pub(crate) fn top_legend_lift(
    theme: &Theme,
    scales: &ScaleSet,
    suppressed: &std::collections::HashSet<Aesthetic>,
    legend_plans: &std::collections::HashMap<Aesthetic, LegendPlan>,
) -> f64 {
    if !matches!(theme.legend_position, LegendPosition::Top) {
        return 0.0;
    }
    if legend_scales_for(scales, suppressed).is_empty() {
        return 0.0;
    }
    let is_horizontal = match theme.legend_direction {
        Some(dir) => matches!(dir, crate::theme::LegendDirection::Horizontal),
        None => true, // Top is always the horizontal auto-position.
    };
    legend_band_height(theme, legend_rows_beyond_first(is_horizontal, legend_plans))
}

/// Compute legend origin based on position setting.
///
/// `legend_extra_rows` is #233's `LayoutExtras::legend_extra_rows` — a
/// horizontal legend wrapped onto more than one row by `plan_legend`'s
/// auto-wrap/explicit grid. Only `Top` reads it: `PlotLayout::
/// compute_with_extras` reserves a `legend_band_height` row directly
/// above `plot_area` (ending exactly at `plot_area.y`), so backing off
/// `plot_area.y` by that same band content (everything but
/// `legend_margin.top`, which is the gap *above* where drawing starts)
/// lands the legend's first row exactly inside its reserved band — after
/// the title/subtitle and the plot's own outer margin, rather than
/// #233's bug (an absolute-from-canvas-origin `legend_margin.top`,
/// overlapping both). Every other position ignores this parameter; pass
/// `0` where the caller has no wrap plan in hand (the deprecated
/// `draw_legend`) or the returned `y` is unused (every `legend_position`
/// call before the final layout pass only needs its `x`/`is_horizontal`).
///
/// Returns (x, y, is_horizontal).
pub(crate) fn legend_position(
    theme: &Theme,
    plot_area: &Rect,
    legend_extra_rows: usize,
) -> (f64, f64, bool) {
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
            plot_area.y - legend_band_height(theme, legend_extra_rows) + theme.legend_margin.top,
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
    is_horizontal: bool,
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
                bg_colour: None,
                bg_r: 0.0,
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
                    corner_radius: 0.0,
                },
            )?;
        }
    }

    // Horizontal legends lay keys left-to-right; vertical ones stack downward.
    let mut cur_x = legend_x;
    for (i, (pos, label)) in breaks.iter().enumerate() {
        let x = if is_horizontal { cur_x } else { legend_x };
        let y = if is_horizontal {
            items_y
        } else {
            items_y + i as f64 * item_height
        };
        let center_x = x + swatch_size / 2.0;
        let center_y = y + swatch_size / 2.0;

        // Draw legend key background
        if theme.legend_key.visible {
            if let Some(fill) = theme.legend_key.fill {
                backend.draw_rect(
                    (x, y),
                    (x + swatch_size, y + swatch_size),
                    &RectStyle {
                        fill: Some(fill),
                        stroke: theme.legend_key.color,
                        stroke_width: theme.legend_key.width,
                        alpha: 1.0,
                        clip: false,
                        corner_radius: 0.0,
                    },
                )?;
            }
        }

        // Draw the appropriate swatch based on aesthetic type
        draw_legend_swatch(
            backend,
            scales,
            aes,
            scale,
            theme,
            label,
            *pos,
            x,
            y,
            center_x,
            center_y,
            swatch_size,
        )?;

        // Label
        let label_family = if theme.legend_text.family.is_empty() {
            None
        } else {
            Some(theme.legend_text.family.clone())
        };
        backend.draw_text(
            label,
            (x + swatch_size + theme.legend_spacing, center_y),
            &TextStyle {
                color: theme.legend_text.color,
                size: theme.legend_text.size,
                anchor: TextAnchor::Start,
                angle: 0.0,
                family: label_family,
                face: theme.legend_text.face,
                bg_colour: None,
                bg_r: 0.0,
            },
        )?;
        if is_horizontal {
            // advance past swatch + spacing + estimated label width + gap
            cur_x = x
                + swatch_size
                + theme.legend_spacing
                + (label.chars().count() as f64) * theme.legend_text.size * 0.62
                + theme.legend_spacing * 3.0;
        }
    }

    if is_horizontal {
        Ok(cur_x - legend_x) // width used
    } else {
        Ok(title_offset + breaks.len() as f64 * item_height)
    }
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
                bg_colour: None,
                bg_r: 0.0,
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
                    corner_radius: 0.0,
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
                corner_radius: 0.0,
            },
        )?;
    }

    // Draw border
    // (Ticks reuse its colour and width even when the frame is hidden.)
    let border = theme.legend_bar_border_line();
    let border_style = LineStyle {
        color: border.color,
        width: border.width,
        alpha: 1.0,
        linetype: border.linetype,
        ..Default::default()
    };
    if border.visible {
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
    }

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
                bg_colour: None,
                bg_r: 0.0,
            },
        )?;
    }

    Ok(title_offset + bar_height)
}

/// Draw one legend key's swatch (not its label) for `aes`, at `(x, y)`
/// (top-left) / `(center_x, center_y)` (centre), `swatch_size` square.
/// Shared by the unchanged [`draw_discrete_legend_at`] and the B3-2
/// planner-driven [`draw_legend_planned`], so both draw pixel-identical
/// swatches.
#[allow(clippy::too_many_arguments)]
fn draw_legend_swatch(
    backend: &mut dyn DrawBackend,
    scales: &ScaleSet,
    aes: &Aesthetic,
    scale: &dyn crate::scale::Scale,
    theme: &Theme,
    label: &str,
    pos: f64,
    x: f64,
    y: f64,
    center_x: f64,
    center_y: f64,
    swatch_size: f64,
) -> Result<(), RenderError> {
    let value = Value::Str(label.to_string());
    match aes {
        Aesthetic::Color | Aesthetic::Fill => {
            let color = scales.map_color(aes, &value).unwrap_or((127, 127, 127));
            let style = RectStyle {
                fill: Some(color),
                stroke: None,
                stroke_width: 0.0,
                alpha: 1.0,
                clip: false,
                corner_radius: 0.0,
            };
            // #206: the "no data" key draws its scale's na_pattern
            // instead of a flat swatch, when set.
            if scale.is_na_break(label) {
                if let Some(p) = scale.na_fill_pattern() {
                    return backend.draw_rect_pattern(
                        (x, y),
                        (x + swatch_size, y + swatch_size),
                        &p,
                        &style,
                    );
                }
            }
            backend.draw_rect((x, y), (x + swatch_size, y + swatch_size), &style)?;
        }
        Aesthetic::Shape => {
            let shape = scales
                .map_shape(&value)
                .unwrap_or(crate::render::backend::PointShape::Circle);
            backend.draw_shape(
                (center_x, center_y),
                swatch_size / 3.0,
                &PointStyle {
                    color: theme.geom.ink.unwrap_or((50, 50, 50)),
                    alpha: 1.0,
                    filled: true,
                    shape,
                },
            )?;
        }
        Aesthetic::Linetype => {
            let lt = scales.map_linetype(&value).unwrap_or(Linetype::Solid);
            backend.draw_line(
                &[(x + 2.0, center_y), (x + swatch_size - 2.0, center_y)],
                &LineStyle {
                    color: theme.geom.ink.unwrap_or((50, 50, 50)),
                    width: 1.5,
                    alpha: 1.0,
                    linetype: lt,
                    ..Default::default()
                },
            )?;
        }
        Aesthetic::Size => {
            let size = scale
                .map_fraction_to_size(pos)
                .or_else(|| scales.map_size(&value))
                .unwrap_or(3.0);
            backend.draw_shape(
                (center_x, center_y),
                size.min(swatch_size / 2.0),
                &PointStyle {
                    color: theme.geom.ink.unwrap_or((50, 50, 50)),
                    alpha: 1.0,
                    filled: true,
                    shape: crate::render::backend::PointShape::Circle,
                },
            )?;
        }
        Aesthetic::Alpha => {
            let alpha = scale
                .map_fraction_to_alpha(pos)
                .or_else(|| scales.map_alpha(&value))
                .unwrap_or(1.0);
            backend.draw_rect(
                (x, y),
                (x + swatch_size, y + swatch_size),
                &RectStyle {
                    fill: Some(theme.geom.ink.unwrap_or((50, 50, 50))),
                    stroke: None,
                    stroke_width: 0.0,
                    alpha,
                    clip: false,
                    corner_radius: 0.0,
                },
            )?;
        }
        Aesthetic::Linewidth => {
            let width = scale.map_fraction_to_size(pos).unwrap_or(1.0);
            backend.draw_line(
                &[(x + 2.0, center_y), (x + swatch_size - 2.0, center_y)],
                &LineStyle {
                    color: theme.geom.ink.unwrap_or((50, 50, 50)),
                    width: width.min(swatch_size),
                    alpha: 1.0,
                    linetype: Linetype::Solid,
                    ..Default::default()
                },
            )?;
        }
        _ => {}
    }
    Ok(())
}

// ─── B3-2: nrow/ncol/byrow, auto-wrap, "+N more" ───────────────────────────

/// One legend key's grid placement, in the planner's own pixel space
/// (relative to the legend's own origin — `row` is 0-indexed top row,
/// `x_offset` is the horizontal pitch from the first column).
#[derive(Clone, Debug)]
pub(crate) struct LegendEntry {
    pub break_idx: usize,
    pub row: usize,
    pub x_offset: f64,
    /// A truncated-with-ellipsis display label (#159), set only when a
    /// vertical multi-column grid still doesn't fit the canvas after
    /// reducing its column count as far as it can go: the full label is
    /// still available (e.g. for a `<title>` tooltip) via the scale's own
    /// `breaks()[break_idx]`. `None` draws the break's label unchanged —
    /// the common, byte-identical case.
    pub label_override: Option<String>,
}

/// A trailing "+{k} more" marker when the legend doesn't have room for
/// every key (D7). Not a mark (`draw_text` only — no swatch, no
/// `data-geom`/class).
#[derive(Clone, Debug)]
pub(crate) struct LegendOverflow {
    pub dropped: usize,
    pub row: usize,
    pub x_offset: f64,
    pub text: String,
}

/// A non-default legend layout (B3-2): an explicit `nrow`/`ncol` grid, or
/// an auto-wrapped horizontal legend. `None` from [`plan_legend`] means
/// "nothing changes" — the caller keeps the 0.21 single-row/single-column
/// path verbatim, so a plot that doesn't need this is byte-identical.
#[derive(Clone, Debug)]
pub(crate) struct LegendPlan {
    pub rows: usize,
    pub entries: Vec<LegendEntry>,
    pub overflow: Option<LegendOverflow>,
    /// Columns in an explicit grid (always 1 for the horizontal auto-wrap
    /// path, which never adds columns). Review #158 finding 3: a vertical
    /// legend with `ncol > 1` needs `(cols − 1) · col_pitch` of extra
    /// width reserved, the same way extra rows are reserved for height.
    pub cols: usize,
    /// The per-column horizontal pitch an explicit grid uses (0 for the
    /// auto-wrap path, which doesn't have columns in this sense).
    pub col_pitch: f64,
}

/// Per-key horizontal pitch (swatch + spacing + label + gap): the larger of
/// today's crude per-character advance (`draw_discrete_legend_at`'s
/// `cur_x` formula) and a `text_width`-based measurement, so a label that
/// ties under both estimates stays on the byte-identical single-row path.
fn legend_pitch(label: &str, theme: &Theme) -> f64 {
    let swatch = theme.legend_key_width;
    let spacing = theme.legend_spacing;
    let text_size = theme.legend_text.size;
    let legacy =
        swatch + spacing + (label.chars().count() as f64) * text_size * 0.62 + spacing * 3.0;
    let measured = swatch + spacing + crate::render::text::text_width(label, text_size) + 8.0;
    legacy.max(measured)
}

/// Plan a non-default legend layout (B3-2, D7). `avail_width` is the
/// horizontal room for a horizontal legend's flow-pack / overflow check
/// (`plot_area.x + plot_area.width - legend_x`); `avail_height` only feeds
/// the default `max_rows` formula. Returns `None` when nothing changes:
/// no explicit `nrow`/`ncol`, and (for a horizontal legend) the one-line
/// layout already fits — or the legend is vertical, where auto-wrap never
/// applies (D7 is horizontal-only; `nrow`/`ncol` still work on either).
pub(crate) fn plan_legend(
    breaks: &[(f64, String)],
    guide: &GuideLegend,
    theme: &Theme,
    avail_width: f64,
    avail_height: f64,
    is_horizontal: bool,
) -> Option<LegendPlan> {
    let n = breaks.len();
    if n == 0 {
        return None;
    }

    // Explicit grid: ggplot2's `nrow`/`ncol`, column-major unless `byrow`.
    if guide.nrow.is_some() || guide.ncol.is_some() {
        let (rows, cols) = match (guide.nrow, guide.ncol) {
            (Some(nrow), Some(ncol)) => (nrow, ncol),
            (Some(nrow), None) => (nrow.max(1), n.div_ceil(nrow.max(1))),
            (None, Some(ncol)) => (n.div_ceil(ncol.max(1)), ncol.max(1)),
            (None, None) => unreachable!("checked above"),
        };
        let rows = rows.max(1);
        let cols = cols.max(1);
        let col_pitch = breaks
            .iter()
            .map(|(_, l)| legend_pitch(l, theme))
            .fold(0.0_f64, f64::max);
        let entries = (0..n)
            .map(|i| {
                let (row, col) = if guide.byrow {
                    (i / cols, i % cols)
                } else {
                    (i % rows, i / rows)
                };
                LegendEntry {
                    break_idx: i,
                    row,
                    x_offset: col as f64 * col_pitch,
                    label_override: None,
                }
            })
            .collect();
        return Some(LegendPlan {
            rows,
            entries,
            overflow: None,
            cols,
            col_pitch,
        });
    }

    // Auto mode: D7 scopes auto-wrap to horizontal legends only.
    if !is_horizontal {
        return None;
    }

    let pitches: Vec<f64> = breaks.iter().map(|(_, l)| legend_pitch(l, theme)).collect();
    let total_width: f64 = pitches.iter().sum();
    if total_width <= avail_width.max(0.0) {
        return None; // fits on one line — byte-identical to 0.21.
    }

    let item_height = theme.legend_key_height.max(1.0);
    let row_limit = guide
        .max_rows
        .unwrap_or_else(|| 1 + (avail_height / 3.0 / item_height).floor().max(0.0) as usize)
        .max(1);

    let mut entries: Vec<LegendEntry> = Vec::with_capacity(n);
    let mut row = 0usize;
    let mut cur_x = 0.0_f64;
    for (i, &p) in pitches.iter().enumerate() {
        if cur_x > 0.0 && cur_x + p > avail_width {
            if row + 1 >= row_limit {
                break; // out of rows — the rest overflow into "+N more".
            }
            row += 1;
            cur_x = 0.0;
        }
        entries.push(LegendEntry {
            break_idx: i,
            row,
            x_offset: cur_x,
            label_override: None,
        });
        cur_x += p;
    }

    if entries.len() >= n {
        return Some(LegendPlan {
            rows: row + 1,
            entries,
            overflow: None,
            cols: 1,
            col_pitch: 0.0,
        });
    }

    // Overflow: make room for "+{k} more" on the last row, dropping
    // entries from its tail until it fits (or the row is empty).
    let mut dropped = n - entries.len();
    loop {
        let text = format!("+{dropped} more");
        let more_pitch = legend_pitch(&text, theme);
        let row_count = entries.iter().filter(|e| e.row == row).count();
        let last_x = entries
            .iter()
            .filter(|e| e.row == row)
            .map(|e| e.x_offset + pitches[e.break_idx])
            .fold(0.0_f64, f64::max);
        if last_x + more_pitch <= avail_width || row_count == 0 {
            return Some(LegendPlan {
                rows: row + 1,
                entries,
                overflow: Some(LegendOverflow {
                    dropped,
                    row,
                    x_offset: last_x,
                    text,
                }),
                cols: 1,
                col_pitch: 0.0,
            });
        }
        entries.pop();
        dropped += 1;
    }
}

/// How many of a plot's legend scales are discrete (the ones B3-2's
/// wrap/grid applies to) — used to split the available horizontal room
/// among them when several sit side by side (sipemu/ggplot-rs#155).
fn discrete_guide_count(scales: &ScaleSet, legend_scales: &[Aesthetic]) -> usize {
    legend_scales
        .iter()
        .filter(|aes| scales.get(aes).map(|s| s.is_discrete()).unwrap_or(false))
        .count()
        .max(1)
}

/// How many of a plot's legend scales are continuous (a colourbar, or a
/// sampled continuous size/alpha legend) — each reserves its own fixed
/// width share of a horizontal row (#170 / review #167 finding 7: a
/// bottom legend mixing a colourbar with a discrete guide ignored the
/// colourbar's share, so the discrete guide's auto-wrap/grid was planned
/// against the *whole* row and could still run off the right edge).
fn continuous_guide_count(scales: &ScaleSet, legend_scales: &[Aesthetic]) -> usize {
    legend_scales
        .iter()
        .filter(|aes| !scales.get(aes).map(|s| s.is_discrete()).unwrap_or(false))
        .count()
}

/// The fixed horizontal width one continuous guide reserves in a
/// horizontal row — the same per-guide step `draw_legend_planned` already
/// advances `offset_x` by once it has drawn one (see its two
/// `is_horizontal` arms for a continuous scale).
fn continuous_guide_width(theme: &Theme) -> f64 {
    theme.legend_key_width + theme.legend_text.size * 6.0 + theme.legend_spacing * 2.0
}

/// Plan every discrete legend scale's B3-2 layout (explicit `nrow`/`ncol`
/// grid, or auto-wrap) independently (sipemu/ggplot-rs#155): previously
/// this only ran for the plot's single discrete guide, leaving a second
/// simultaneous discrete guide (e.g. both `colour` and `shape` mapped to
/// discrete variables) always on the unwrapped 0.21 path. When several
/// discrete guides sit side by side (`is_horizontal`), the available
/// width is split evenly among them first, so each one wraps against its
/// own share rather than the whole row (which would let it claim more
/// room than its neighbours leave). Guides that stack vertically instead
/// each get the full `avail_width`, since they don't compete for it.
/// Returns one entry per discrete guide that needs anything beyond the
/// single-row/single-column path — a guide absent from the map is
/// byte-identical to 0.21/0.22.
#[allow(clippy::too_many_arguments)]
pub(crate) fn plan_all_legends(
    scales: &ScaleSet,
    guide: &GuideLegend,
    theme: &Theme,
    suppressed: &std::collections::HashSet<Aesthetic>,
    avail_width: f64,
    avail_height: f64,
    is_horizontal: bool,
    warnings: &mut Vec<String>,
) -> std::collections::HashMap<Aesthetic, LegendPlan> {
    let legend_scales = legend_scales_for(scales, suppressed);
    let n_discrete = discrete_guide_count(scales, &legend_scales);
    let per_guide_width = if is_horizontal {
        // #170: leave the discrete guides only the width left over after
        // every continuous guide's own fixed share (a colourbar sitting
        // in the same row) — previously `avail_width` was split among
        // the discrete guides alone, so a colourbar plus a discrete
        // guide could together still overflow the row.
        let n_continuous = continuous_guide_count(scales, &legend_scales);
        let continuous_width = n_continuous as f64 * continuous_guide_width(theme);
        ((avail_width - continuous_width) / n_discrete as f64).max(0.0)
    } else {
        avail_width
    };

    let mut plans = std::collections::HashMap::new();
    for aes in &legend_scales {
        let Some(scale) = scales.get(aes) else {
            continue;
        };
        if !scale.is_discrete() {
            continue;
        }
        let breaks = scale.breaks();
        let validated_guide = guide.validated(breaks.len(), warnings);
        if let Some(plan) = plan_legend(
            &breaks,
            &validated_guide,
            theme,
            per_guide_width,
            avail_height,
            is_horizontal,
        ) {
            plans.insert(aes.clone(), plan);
        }
    }
    plans
}

/// Draw all legends for the plot, honouring each discrete guide's B3-2
/// `plan_legend` layout (explicit `nrow`/`ncol`, or an overflowing
/// horizontal legend's auto-wrap) when it has one — several simultaneous
/// discrete guides each get their own independent plan
/// (sipemu/ggplot-rs#155). A guide missing from `legend_plans` falls back
/// to the unchanged [`draw_legend`] path — same calls, same order — which
/// keeps this byte-identical to 0.21/0.22 for every plot nothing in
/// `legend_plans` touches. Used by `renderer.rs` (single and faceted) and
/// `compose::capture_legend` (PlotGrid-collected legends).
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_legend_planned(
    scales: &ScaleSet,
    theme: &Theme,
    plot_area: &Rect,
    // Review #167 finding 1: the width share each discrete guide was
    // *planned* against (the same `avail_width` passed to
    // `plan_all_legends`), not `plot_area.width`. For a single plot these
    // are the same value in practice, but a PlotGrid's collected legend
    // (`compose::capture_legend`) measures itself against a fake
    // zero-width panel so its origin lands at (0, 0) — stepping by
    // `plot_area.width / n` there always advanced by zero, so the second
    // discrete guide (e.g. `shape` after `colour`) was drawn directly on
    // top of the first instead of beside it.
    legend_avail_width: f64,
    backend: &mut dyn DrawBackend,
    guide: &GuideLegend,
    suppressed: &std::collections::HashSet<Aesthetic>,
    // Review #158 finding 2: each discrete guide's plan, computed exactly
    // once in `layout_built` (`BuiltPlot::legend_plans`) — drawn verbatim
    // here rather than recomputed against this (possibly shorter, post-
    // row-reservation) `plot_area`, so the drawn "+N more" always matches
    // the warning that plan already produced.
    legend_plans: &std::collections::HashMap<Aesthetic, LegendPlan>,
) -> Result<(), RenderError> {
    if matches!(theme.legend_position, LegendPosition::None) {
        return Ok(());
    }

    let legend_scales = legend_scales_for(scales, suppressed);
    if legend_scales.is_empty() {
        return Ok(());
    }

    // #233: `legend_position`'s `Top` arm needs the same wrapped-row
    // count `layout_built` fed `PlotLayout::compute_with_extras` to
    // produce this `plot_area` in the first place — recomputed here from
    // `legend_plans` (the exact plans that count came from) the same way
    // `layout_built` did, so it can never disagree with the band
    // `plot_area.y` was actually reserved against. `is_horizontal`'s
    // auto value only depends on the position itself (`Top`/`Bottom`),
    // overridable by `legend_direction` same as below — computed first
    // since `legend_position` needs it to pick `max` (several guides
    // sharing a row) vs. `sum` (several guides stacked), the same
    // either/or `layout_built` used.
    let is_horizontal_hint = match theme.legend_direction {
        Some(dir) => matches!(dir, crate::theme::LegendDirection::Horizontal),
        None => matches!(
            theme.legend_position,
            LegendPosition::Top | LegendPosition::Bottom
        ),
    };
    let legend_extra_rows = legend_rows_beyond_first(is_horizontal_hint, legend_plans);
    let (legend_x, legend_y, mut is_horizontal) =
        legend_position(theme, plot_area, legend_extra_rows);
    if let Some(dir) = theme.legend_direction {
        is_horizontal = matches!(dir, crate::theme::LegendDirection::Horizontal);
    }
    let n_discrete = discrete_guide_count(scales, &legend_scales);
    // #170: match `plan_all_legends`'s share — a continuous guide (e.g. a
    // colourbar) sharing this row has already claimed its own fixed
    // width, so step the discrete guides by what's left, not the whole
    // `legend_avail_width`.
    let n_continuous = continuous_guide_count(scales, &legend_scales);
    let continuous_width = n_continuous as f64 * continuous_guide_width(theme);
    let per_guide_width = ((legend_avail_width - continuous_width) / n_discrete as f64).max(0.0);

    let mut offset_y = legend_y;
    let mut offset_x = legend_x;

    for aes in &legend_scales {
        let scale = scales.get(aes).unwrap();
        if !scale.is_discrete() {
            // Continuous legends (colorbar, or size/alpha sampled breaks)
            // aren't in scope for B3-2 — unchanged.
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
                let height = draw_discrete_legend_at(
                    scales,
                    aes,
                    scale,
                    theme,
                    offset_x,
                    offset_y,
                    backend,
                    guide,
                    is_horizontal,
                )?;
                if is_horizontal {
                    offset_x += theme.legend_key_width
                        + theme.legend_text.size * 6.0
                        + theme.legend_spacing * 2.0;
                } else {
                    offset_y += height + theme.legend_spacing * 2.0;
                }
            }
            continue;
        }

        match legend_plans.get(aes) {
            None => {
                if is_horizontal {
                    let width = draw_discrete_legend_at(
                        scales, aes, scale, theme, offset_x, offset_y, backend, guide, true,
                    )?;
                    offset_x += width + theme.legend_spacing * 2.0;
                } else {
                    let height = draw_discrete_legend_at(
                        scales, aes, scale, theme, offset_x, offset_y, backend, guide, false,
                    )?;
                    offset_y += height + theme.legend_spacing * 2.0;
                }
            }
            Some(plan) => {
                let height = draw_discrete_legend_grid(
                    scales, aes, scale, theme, offset_x, offset_y, backend, guide, plan,
                )?;
                if is_horizontal {
                    // Each discrete guide gets an equal share of the row
                    // (sipemu/ggplot-rs#155) rather than the whole row,
                    // which only held for exactly one discrete guide.
                    offset_x += per_guide_width;
                } else {
                    offset_y += height + theme.legend_spacing * 2.0;
                }
            }
        }
    }

    Ok(())
}

/// Draw a discrete legend using a [`LegendPlan`] (explicit grid or
/// auto-wrap). Returns the total height used (including any "+N more"
/// row).
#[allow(clippy::too_many_arguments)]
fn draw_discrete_legend_grid(
    scales: &ScaleSet,
    aes: &Aesthetic,
    scale: &dyn crate::scale::Scale,
    theme: &Theme,
    legend_x: f64,
    legend_y: f64,
    backend: &mut dyn DrawBackend,
    guide: &GuideLegend,
    plan: &LegendPlan,
) -> Result<f64, RenderError> {
    let mut breaks = scale.breaks();
    if breaks.is_empty() {
        return Ok(0.0);
    }
    if guide.reverse {
        breaks.reverse();
    }

    let swatch_size = theme.legend_key_width;
    let item_height = theme.legend_key_height;

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
                bg_colour: None,
                bg_r: 0.0,
            },
        )?;
        theme.legend_title.size + 4.0
    } else {
        0.0
    };
    let items_y = legend_y + title_offset;

    for entry in &plan.entries {
        let (pos, label) = &breaks[entry.break_idx];
        let x = legend_x + entry.x_offset;
        let y = items_y + entry.row as f64 * item_height;
        let center_x = x + swatch_size / 2.0;
        let center_y = y + swatch_size / 2.0;

        if theme.legend_key.visible {
            if let Some(fill) = theme.legend_key.fill {
                backend.draw_rect(
                    (x, y),
                    (x + swatch_size, y + swatch_size),
                    &RectStyle {
                        fill: Some(fill),
                        stroke: theme.legend_key.color,
                        stroke_width: theme.legend_key.width,
                        alpha: 1.0,
                        clip: false,
                        corner_radius: 0.0,
                    },
                )?;
            }
        }

        draw_legend_swatch(
            backend,
            scales,
            aes,
            scale,
            theme,
            label,
            *pos,
            x,
            y,
            center_x,
            center_y,
            swatch_size,
        )?;

        let label_family = if theme.legend_text.family.is_empty() {
            None
        } else {
            Some(theme.legend_text.family.clone())
        };
        // #159: a label truncated to fit a vertical multi-column grid
        // still shows its full text via a `<title>` tooltip (backends
        // without tooltip support just render the truncated text).
        let display = entry.label_override.as_deref().unwrap_or(label.as_str());
        backend.draw_text_with_title(
            display,
            (x + swatch_size + theme.legend_spacing, center_y),
            &TextStyle {
                color: theme.legend_text.color,
                size: theme.legend_text.size,
                anchor: TextAnchor::Start,
                angle: 0.0,
                family: label_family,
                face: theme.legend_text.face,
                bg_colour: None,
                bg_r: 0.0,
            },
            entry.label_override.as_deref().map(|_| label.as_str()),
        )?;
    }

    // The overflow marker is text only — not a mark, no swatch.
    if let Some(ov) = &plan.overflow {
        let x = legend_x + ov.x_offset;
        let y = items_y + ov.row as f64 * item_height + swatch_size / 2.0;
        let label_family = if theme.legend_text.family.is_empty() {
            None
        } else {
            Some(theme.legend_text.family.clone())
        };
        backend.draw_text(
            &ov.text,
            (x, y),
            &TextStyle {
                color: theme.legend_text.color,
                size: theme.legend_text.size,
                anchor: TextAnchor::Start,
                angle: 0.0,
                family: label_family,
                face: theme.legend_text.face,
                bg_colour: None,
                bg_r: 0.0,
            },
        )?;
    }

    Ok(title_offset + plan.rows as f64 * item_height)
}
