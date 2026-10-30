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

/// Whether `a` is one of [`LEGEND_AESTHETICS`] (#241: `has_legend_mapping`'s
/// own `is_legend_aes` check, in `plot.rs`'s layout-building pass, had
/// drifted into a second, incomplete copy of this list — missing
/// `Linewidth`, so a plot mapping only `linewidth` reserved *no* legend
/// space at all, and its legend then drew with nothing held back for it,
/// running off the canvas). Delegating to this one list means the two
/// checks — "should this aesthetic reserve layout space" and "should this
/// aesthetic actually draw a legend" — can't drift apart again.
pub(crate) fn is_legend_aesthetic(a: &Aesthetic) -> bool {
    LEGEND_AESTHETICS.contains(a)
}

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
                legend_scales.push(aes.clone());
            }
        }
    }
    legend_scales
}

/// Group `legend_scales` into mergeable sets (#239: ggplot2 merges `fill`
/// and `colour` guides mapped to the same measure through the same scale
/// spec into one legend with a combined key, rather than drawing one per
/// aesthetic — and, conversely, draws *two* legends when they don't
/// actually match, which an earlier, cruder rule here got wrong: it
/// unconditionally dropped `Fill`'s own legend whenever `Color` was also
/// mapped, even to an unrelated variable with a different title and
/// breaks). Two aesthetics merge when their scales share the same
/// effective title (`guide.title`, else `scale.name()`), the same
/// discrete/continuous-ness, and *identical* `breaks()` (same positions
/// and labels, in the same order) — the same practical "do these two
/// guides look identical" test ggplot2 itself applies; a binned scale's
/// breaks already carry its counts in their labels, so two binned scales
/// with the same counts per bin merge too. Order preserved: each group
/// keeps the position its first member had in `legend_scales`.
pub(crate) fn group_mergeable_legends(
    scales: &ScaleSet,
    legend_scales: &[Aesthetic],
    guide: &GuideLegend,
) -> Vec<Vec<Aesthetic>> {
    struct Key {
        title: String,
        discrete: bool,
        breaks: Vec<(f64, String)>,
    }
    let mut groups: Vec<(Key, Vec<Aesthetic>)> = Vec::new();
    for aes in legend_scales {
        let Some(scale) = scales.get(aes) else {
            continue;
        };
        let breaks = scale.breaks();
        let title = guide
            .title
            .clone()
            .unwrap_or_else(|| scale.name().to_string());
        let discrete = scale.is_discrete();
        match groups
            .iter_mut()
            .find(|(k, _)| k.title == title && k.discrete == discrete && k.breaks == breaks)
        {
            Some((_, members)) => members.push(aes.clone()),
            None => groups.push((
                Key {
                    title,
                    discrete,
                    breaks,
                },
                vec![aes.clone()],
            )),
        }
    }
    groups.into_iter().map(|(_, members)| members).collect()
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
    // always 0 — see `legend_position`'s doc comment. It also predates
    // #225's `Inside` justification and keeps the old top-left anchor
    // (`legend_size_hint: (0.0, 0.0)`) rather than estimate it, since
    // it's unused internally and has no direct public replacement to
    // keep behaviourally current.
    let (legend_x, legend_y, mut is_horizontal) =
        legend_position(theme, plot_area, 0, (0.0, 0.0), 0.0, plot_area);
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
                    scales,
                    &[((*aes).clone(), scale)],
                    theme,
                    offset_x,
                    offset_y,
                    backend,
                    guide,
                    true,
                )?;
                offset_x += width + theme.legend_spacing * 2.0;
            } else {
                let height = draw_discrete_legend_at(
                    scales,
                    &[((*aes).clone(), scale)],
                    theme,
                    offset_x,
                    offset_y,
                    backend,
                    guide,
                    false,
                )?;
                offset_y += height + theme.legend_spacing * 2.0;
            }
        } else {
            // Continuous legend (colorbar) — only for color/fill
            if matches!(aes, Aesthetic::Color | Aesthetic::Fill) {
                let height = draw_continuous_legend_at(
                    scale,
                    theme,
                    offset_x,
                    offset_y,
                    backend,
                    guide,
                    is_horizontal,
                )?;
                if is_horizontal {
                    // #211: the bar is now drawn along x (wide, not
                    // narrow) — step by its own length, not the old
                    // vertical-bar width-plus-side-label estimate.
                    let (bar_len, _) = continuous_bar_dims(theme, true);
                    offset_x += bar_len + theme.legend_spacing * 2.0;
                } else {
                    offset_y += height + theme.legend_spacing * 2.0;
                }
            } else {
                // Continuous size/alpha — draw as discrete-like with sampled breaks
                let height = draw_discrete_legend_at(
                    scales,
                    &[((*aes).clone(), scale)],
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
///
/// #577: `legend_band_height` alone only covers the flat discrete-row
/// case — a continuous, horizontally-drawn Color/Fill colourbar *with its
/// own title* needs the same extra height `plot.rs` already reserves in
/// `plot_area.y` via `continuous_horizontal_legend_extra_height` (#211).
/// Without adding that extra here too, this lift undershot by exactly
/// that amount, so the plot's own title/subtitle landed on top of the
/// legend's title instead of above it. `guide` is the same
/// [`GuideLegend`] `draw_legend_planned` draws with, so "does this guide
/// have a title" can't disagree between the two.
pub(crate) fn top_legend_lift(
    theme: &Theme,
    scales: &ScaleSet,
    suppressed: &std::collections::HashSet<Aesthetic>,
    legend_plans: &std::collections::HashMap<Aesthetic, LegendPlan>,
    guide: &GuideLegend,
) -> f64 {
    if !matches!(theme.legend_position, LegendPosition::Top) {
        return 0.0;
    }
    let legend_scales = legend_scales_for(scales, suppressed);
    if legend_scales.is_empty() {
        return 0.0;
    }
    let is_horizontal = match theme.legend_direction {
        Some(dir) => matches!(dir, crate::theme::LegendDirection::Horizontal),
        None => true, // Top is always the horizontal auto-position.
    };
    let base = legend_band_height(theme, legend_rows_beyond_first(is_horizontal, legend_plans));
    let colorbar_extra = if is_horizontal {
        legend_scales
            .iter()
            .find_map(|aes| {
                if !matches!(aes, Aesthetic::Color | Aesthetic::Fill) {
                    return None;
                }
                let scale = scales.get(aes)?;
                if scale.is_discrete() {
                    return None;
                }
                let title = guide.title.as_deref().unwrap_or_else(|| scale.name());
                let bar_height = continuous_horizontal_bar_height(theme, !title.is_empty());
                Some((bar_height - theme.legend_key_height).max(0.0))
            })
            .unwrap_or(0.0)
    } else {
        0.0
    };
    base + colorbar_extra
}

/// Estimate the on-canvas footprint `(width, height)` of every legend
/// scale about to be drawn, stacked the same way `draw_legend_planned`
/// stacks them (each one's own height summed, the widest width shared)
/// — used only by `legend_position`'s `Inside` arm (#225) to place the
/// *whole* legend block by justification instead of always anchoring
/// its top-left corner there. Mirrors each draw function's own returned
/// height/width formula exactly (`draw_continuous_legend_at`,
/// `draw_discrete_legend_at`, `draw_discrete_legend_grid`) without
/// drawing anything. A `legend_plans` entry (an explicit `nrow`/`ncol`
/// grid — auto-wrap never applies to a non-horizontal legend like
/// `Inside`) is honoured for an accurate multi-row/-column estimate;
/// everything else uses the single-column formula.
fn estimate_legend_box(
    scales: &ScaleSet,
    legend_scales: &[Aesthetic],
    theme: &Theme,
    guide: &GuideLegend,
    legend_plans: &std::collections::HashMap<Aesthetic, LegendPlan>,
) -> (f64, f64) {
    let mut total_h = 0.0_f64;
    let mut max_w = 0.0_f64;
    for aes in legend_scales {
        let Some(scale) = scales.get(aes) else {
            continue;
        };
        let breaks = scale.breaks();
        if breaks.is_empty() {
            continue;
        }
        let title = guide.title.as_deref().unwrap_or_else(|| scale.name());
        let title_offset = if !title.is_empty() {
            theme.legend_title.size + 4.0
        } else {
            0.0
        };
        // #236: a size legend's key box (and the row height it sits in)
        // grows to fit its largest key — see `size_key_diameter`.
        let diameter = size_key_diameter(aes, scales, scale, &breaks);
        let key_width = theme.legend_key_width.max(diameter);
        let key_height = theme.legend_key_height.max(key_width);
        // The widest label and the title as drawn (a fixed six-character
        // guess let a long label or title run off a right-corner legend).
        let label_w = breaks
            .iter()
            .map(|(_, l)| crate::render::text::text_width(l, theme.legend_text.size))
            .fold(0.0_f64, f64::max);
        let title_w = crate::render::text::text_width(title, theme.legend_title.size);
        let single_col_width = (key_width + theme.legend_spacing + label_w).max(title_w);
        let is_continuous_bar =
            matches!(aes, Aesthetic::Color | Aesthetic::Fill) && !scale.is_discrete();
        let (w, h) = if is_continuous_bar {
            // The bottom break label is centred on the bar's end: its lower
            // half hangs below the bar (#377).
            (
                single_col_width,
                title_offset + key_height * 8.0 + theme.legend_text.size / 2.0,
            )
        } else if let Some(plan) = legend_plans.get(aes) {
            let w = if plan.col_pitch > 0.0 {
                plan.cols as f64 * plan.col_pitch
            } else {
                single_col_width
            };
            (w, title_offset + plan.rows as f64 * key_height)
        } else {
            (
                single_col_width,
                title_offset + breaks.len() as f64 * key_height,
            )
        };
        total_h += h;
        max_w = max_w.max(w);
    }
    (max_w, total_h)
}

/// Clamp a `[pos, pos + size]` span inside `[lo, hi]` — `pos` alone when
/// `size` already overflows `[lo, hi]` (a legend taller/wider than the
/// canvas: nothing to do but left/top-align it and let it overflow, same
/// as #225's old unconditional top-left anchor did for every `Inside`
/// legend).
fn clamp_span(pos: f64, size: f64, lo: f64, hi: f64) -> f64 {
    if size >= hi - lo {
        lo
    } else {
        pos.clamp(lo, hi - size)
    }
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
/// overlapping both).
///
/// `legend_size_hint` (#225) is the legend's own estimated `(width,
/// height)` — see `estimate_legend_box` — read only by `Inside`. ggplot2
/// defaults `legend.justification.inside` to `legend.position.inside`:
/// the legend's own fractional point `(fx, fy)` (measured from its own
/// bottom-left, in the same `0..=1` convention `Inside`'s `(fx, fy)`
/// already uses) lands exactly on the panel's point `(fx, fy)`, rather
/// than always anchoring the legend's top-left corner there (#225's
/// bug: a legend taller than roughly `(1 - fy) * plot_area.height` —
/// trivially true for a tall continuous colourbar near `fy = 0`, "near
/// the bottom" — ran off the canvas, and a discrete legend with enough
/// levels did exactly the same). `canvas` is a last-resort clamp for a
/// legend taller or wider than the *panel*: still on the canvas, even if
/// that means spilling outside the panel itself.
///
/// Every other position ignores `legend_size_hint`; callers that don't
/// have a real estimate in hand (the deprecated `draw_legend`, or where
/// the returned `y` is unused) pass `(0.0, 0.0)` and `plot_area` itself
/// as `canvas` — harmless, since neither matters there.
///
/// Returns (x, y, is_horizontal).
pub(crate) fn legend_position(
    theme: &Theme,
    plot_area: &Rect,
    legend_extra_rows: usize,
    legend_size_hint: (f64, f64),
    title_lift: f64,
    canvas: &Rect,
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
            // Below the x axis — at most the 30px it used to assume, so an
            // axis-less theme (`theme_void`) keeps the legend inside the band
            // the layout reserved rather than off the canvas (#341).
            plot_area.y
                + plot_area.height
                + theme.legend_margin.top
                + crate::render::layout::PlotLayout::x_axis_band(theme, None).min(30.0),
            true,
        ),
        LegendPosition::None => (0.0, 0.0, false),
        LegendPosition::Inside(fx, fy) => {
            let (legend_w, legend_h) = legend_size_hint;
            // #377: inset by `legend_margin` from the panel edges, like
            // every outside position, and place the *visible* box: the
            // title is drawn centred on the origin's y, so its upper half
            // (`lift`) sits above the origin.
            let m = &theme.legend_margin;
            let lift = title_lift;
            let box_h = legend_h + lift;
            let raw_x = plot_area.x + m.left + fx * (plot_area.width - m.left - m.right - legend_w);
            // fy = 1 is the panel's top in ggplot-rs's convention (y grows
            // down in SVG space, same flip the old formula already had).
            let raw_top =
                plot_area.y + m.top + (1.0 - fy) * (plot_area.height - m.top - m.bottom - box_h);
            let x = clamp_span(raw_x, legend_w, canvas.x, canvas.x + canvas.width);
            let top = clamp_span(raw_top, box_h, canvas.y, canvas.y + canvas.height);
            (x, top + lift, false)
        }
    }
}

/// How far an inside legend's title reaches above the legend origin (it is
/// drawn centred on the origin's y): half the title's size when the first
/// guide has a title, else nothing (#377).
fn inside_title_lift(
    scales: &ScaleSet,
    representatives: &[Aesthetic],
    theme: &Theme,
    guide: &GuideLegend,
) -> f64 {
    let first = representatives
        .iter()
        .filter_map(|aes| scales.get(aes))
        .find(|scale| !scale.breaks().is_empty());
    match first {
        Some(scale)
            if theme.legend_title.visible
                && !guide
                    .title
                    .as_deref()
                    .unwrap_or_else(|| scale.name())
                    .is_empty() =>
        {
            theme.legend_title.size / 2.0
        }
        _ => 0.0,
    }
}

/// For an `Aesthetic::Size` scale, the largest mapped key's diameter
/// across `breaks` (#236: ggplot2 draws each size key at its break's own
/// mapped size and grows the key box to fit the largest one, rather than
/// clamping every key to the fixed default swatch size — which made
/// every key past a certain size draw identically, the reported bug).
/// `0.0` for every other aesthetic, or if every mapped size already fits
/// the theme's default `legend_key_width`, so a caller can just `.max()`
/// it into the usual fixed size with no special-casing and no change to
/// existing output when nothing needs the extra room.
pub(crate) fn size_key_diameter(
    aes: &Aesthetic,
    scales: &ScaleSet,
    scale: &dyn crate::scale::Scale,
    breaks: &[(f64, String)],
) -> f64 {
    if *aes != Aesthetic::Size {
        return 0.0;
    }
    let max_r = breaks
        .iter()
        .map(|(pos, label)| {
            scale
                .map_fraction_to_size(*pos)
                .or_else(|| scales.map_size(&Value::Str(label.clone())))
                .unwrap_or(3.0)
        })
        .fold(0.0_f64, f64::max);
    max_r * 2.0
}

/// Draw a discrete legend at a given position. Returns the height used.
#[allow(clippy::too_many_arguments)]
/// Draw one discrete legend — or, when `group` has more than one entry
/// (#239: guide merging), one *combined* legend whose every key overlays
/// each aesthetic's own glyph at the same position (e.g. a filled swatch
/// with a mapped-colour outline), the way ggplot2 merges `fill` and
/// `colour` guides mapped to the same measure through the same scale
/// spec into one legend instead of two. `group[0]` is the representative
/// scale for the title, breaks and labels — `group_mergeable_legends`
/// only ever groups aesthetics whose `breaks()` (and so their shared
/// title) are already identical, so any member would do.
fn draw_discrete_legend_at(
    scales: &ScaleSet,
    group: &[(Aesthetic, &dyn crate::scale::Scale)],
    theme: &Theme,
    legend_x: f64,
    legend_y: f64,
    backend: &mut dyn DrawBackend,
    guide: &GuideLegend,
    is_horizontal: bool,
) -> Result<f64, RenderError> {
    let (_, scale) = &group[0];
    let mut breaks = scale.breaks();
    if breaks.is_empty() {
        return Ok(0.0);
    }

    // Apply guide reverse
    if guide.reverse {
        breaks.reverse();
    }

    // #236: a size legend's key box (and the row height it sits in) grows
    // to fit its largest key — see `size_key_diameter`. A no-op (both
    // `.max()`s keep the theme defaults) for every other aesthetic, and
    // for a size legend whose largest key already fits them. At most one
    // `group` member is ever `Aesthetic::Size` (`LEGEND_AESTHETICS` has
    // only one), so taking the max across the group is exactly the same
    // as the single-aesthetic case when `group.len() == 1`.
    let diameter = group
        .iter()
        .map(|(a, s)| size_key_diameter(a, scales, *s, &breaks))
        .fold(0.0_f64, f64::max);
    let swatch_size = theme.legend_key_width.max(diameter);
    let item_height = theme.legend_key_height.max(swatch_size);

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

        // Draw every group member's own glyph, overlaid at the same
        // position (#239) — a single swatch for the common, unmerged
        // case (`group.len() == 1`).
        for (member_aes, member_scale) in group {
            draw_legend_swatch(
                backend,
                scales,
                member_aes,
                *member_scale,
                theme,
                label,
                *pos,
                x,
                y,
                center_x,
                center_y,
                swatch_size,
            )?;
        }

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

/// #211: the bar's own `(length, thickness)` in pixels for a continuous
/// colorbar — vertical keeps the original tall-and-narrow dims
/// (`legend_key_width` thick, `legend_key_height * 8.0` long); horizontal
/// swaps them (ggplot2's `guide_colourbar(direction = "horizontal")`),
/// so the bar is wide-and-short instead. Shared by
/// [`draw_continuous_legend_at`] (the real draw) and
/// [`continuous_horizontal_bar_height`] (the layout-time estimate), so
/// the two can't disagree.
fn continuous_bar_dims(theme: &Theme, is_horizontal: bool) -> (f64, f64) {
    if is_horizontal {
        (theme.legend_key_height * 8.0, theme.legend_key_width)
    } else {
        (theme.legend_key_width, theme.legend_key_height * 8.0)
    }
}

/// #211: the total height a *horizontal* continuous colorbar consumes —
/// title line (if `has_title`) + bar thickness + its tick marks + one
/// line of labels below. Used by `plot.rs` to reserve the Top/Bottom
/// legend band's height before layout runs; mirrors
/// [`draw_continuous_legend_at`]'s own math exactly so the reserved band
/// and the actual draw can't disagree.
pub(crate) fn continuous_horizontal_bar_height(theme: &Theme, has_title: bool) -> f64 {
    let title_offset = if has_title {
        theme.legend_title.size + 4.0
    } else {
        0.0
    };
    let (_, bar_thickness) = continuous_bar_dims(theme, true);
    let tick_len = 3.0;
    let label_line = theme.legend_text.size + 4.0;
    title_offset + bar_thickness + tick_len + theme.legend_spacing + label_line
}

/// Draw a continuous colorbar legend at a given position. `is_horizontal`
/// (#211) draws it wide × short — bar slices left to right from the
/// scale's min to max, ticks/labels below the bar, matching ggplot2's
/// `guide_colourbar(direction = "horizontal")` — instead of the default
/// tall × narrow bar with ticks/labels to its right. Returns the height
/// used.
fn draw_continuous_legend_at(
    scale: &dyn crate::scale::Scale,
    theme: &Theme,
    legend_x: f64,
    legend_y: f64,
    backend: &mut dyn DrawBackend,
    guide: &GuideLegend,
    is_horizontal: bool,
) -> Result<f64, RenderError> {
    let breaks = scale.breaks();
    if breaks.is_empty() {
        return Ok(0.0);
    }

    let (bar_width, bar_height) = continuous_bar_dims(theme, is_horizontal);

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
        let total_width = if is_horizontal {
            bar_width
        } else {
            bar_width + theme.legend_spacing + theme.legend_text.size * 6.0
        };
        let total_height = if is_horizontal {
            bar_height + theme.legend_spacing + theme.legend_text.size * 2.0
        } else {
            bar_height
        };
        if let Some(fill) = theme.legend_background.fill {
            backend.draw_rect(
                (legend_x - 2.0, bar_top - 2.0),
                (legend_x + total_width + 2.0, bar_top + total_height + 2.0),
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

    // Draw gradient bar as N thin slices — stacked top to bottom (max at
    // the top) when vertical, side by side left to right (min on the
    // left, like an x-axis) when horizontal.
    // Use data-domain values to avoid double-normalization in map_to_color()
    let (data_min, data_max) = scale.domain().unwrap_or((0.0, 1.0));
    let n_slices = 50;
    if is_horizontal {
        let slice_width = bar_width / n_slices as f64;
        for i in 0..n_slices {
            let t = i as f64 / n_slices as f64;
            let data_val = data_min + t * (data_max - data_min);
            let color = scale
                .map_to_color(&Value::Float(data_val))
                .unwrap_or((127, 127, 127));
            let sx = legend_x + i as f64 * slice_width;
            backend.draw_rect(
                (sx, bar_top),
                (sx + slice_width + 0.5, bar_top + bar_height),
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
    } else {
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

    // Draw tick marks and labels: to the bar's right (vertical, pos=1 at
    // the top) or below it (horizontal, pos=0 on the left), matching the
    // slice order drawn above.
    let tick_len = 3.0;
    let tick_family = if theme.legend_text.family.is_empty() {
        None
    } else {
        Some(theme.legend_text.family.clone())
    };
    for (pos, label) in &breaks {
        if is_horizontal {
            let tick_x = legend_x + bar_width * pos;
            backend.draw_line(
                &[
                    (tick_x, bar_top + bar_height),
                    (tick_x, bar_top + bar_height + tick_len),
                ],
                &border_style,
            )?;
            backend.draw_text(
                label,
                (
                    tick_x,
                    bar_top + bar_height + tick_len + theme.legend_text.size,
                ),
                &TextStyle {
                    color: theme.legend_text.color,
                    size: theme.legend_text.size,
                    anchor: TextAnchor::Middle,
                    angle: 0.0,
                    family: tick_family.clone(),
                    face: theme.legend_text.face,
                    bg_colour: None,
                    bg_r: 0.0,
                },
            )?;
        } else {
            let tick_y = bar_top + bar_height * (1.0 - pos);
            backend.draw_line(
                &[
                    (legend_x + bar_width, tick_y),
                    (legend_x + bar_width + tick_len, tick_y),
                ],
                &border_style,
            )?;
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
                    family: tick_family.clone(),
                    face: theme.legend_text.face,
                    bg_colour: None,
                    bg_r: 0.0,
                },
            )?;
        }
    }

    if is_horizontal {
        Ok(continuous_horizontal_bar_height(theme, !title.is_empty()))
    } else {
        Ok(title_offset + bar_height)
    }
}

/// Draw one legend key's swatch (not its label) for `aes`, at `(x, y)`
/// (top-left) / `(center_x, center_y)` (centre), `swatch_size` square.
/// Shared by the unchanged [`draw_discrete_legend_at`] and the B3-2
/// planner-driven [`draw_legend_planned`], so both draw pixel-identical
/// swatches.
///
/// A key with no aesthetic-mapped colour of its own (`Shape`, `Linetype`,
/// `Size`, `Alpha`, `Linewidth` — `Color`/`Fill` always have one, from
/// `scales.map_color`) falls back to `theme.geom.ink`, then
/// `theme.legend_text.color` (#237: a *fixed* dark grey here was almost
/// invisible on a dark panel in a theme that sets `legend_text.color`
/// for readability but never thought to also set the less commonly used
/// `geom.ink`) — the same two-tier fallback `GeomPoint`/`GeomText`'s own
/// `apply_element_geom` effectively uses for an unmapped panel mark
/// (`ink` if the theme sets it, otherwise its own default), extended one
/// more tier since a key glyph has no mapped colour of its own to start
/// from the way a panel mark's default colour does. **Visible default
/// change:** the base theme's `legend_text.color` is pure black
/// (`(0, 0, 0)`), not the old fallback's dark grey (`(50, 50, 50)`) — a
/// small, intentional difference (documented in the CHANGELOG), not a
/// behaviour-preserving refactor.
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
                    color: theme.geom.ink.unwrap_or(theme.legend_text.color),
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
                    color: theme.geom.ink.unwrap_or(theme.legend_text.color),
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
                    color: theme.geom.ink.unwrap_or(theme.legend_text.color),
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
                    fill: Some(theme.geom.ink.unwrap_or(theme.legend_text.color)),
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
                    color: theme.geom.ink.unwrap_or(theme.legend_text.color),
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

/// `Color`/`Fill` are the only aesthetics whose *continuous* legend draws
/// as an actual colourbar (`draw_continuous_legend_at`, fixed width). Every
/// other continuous legend (`Size`, `Linewidth`, `Alpha`, …) draws sampled
/// breaks the same way a discrete guide's keys do
/// (`draw_discrete_legend_at` / `draw_discrete_legend_grid`), so it needs
/// the same B3-2 wrap/grid planning a discrete guide gets rather than a
/// fixed-width reservation — sipemu/ggplot-rs#545: previously this wasn't
/// planned *at all*, so a `Size`/`Linewidth` legend with enough keys to
/// overflow a narrow `Top`/`Bottom` row just drew past the panel/canvas
/// edge with no wrap, shrink, or "+N more" to catch it.
pub(crate) fn is_colorbar_aesthetic(aes: &Aesthetic) -> bool {
    matches!(aes, Aesthetic::Color | Aesthetic::Fill)
}

/// How many of a plot's legend scales get B3-2's wrap/grid planning — every
/// discrete guide, plus a continuous guide that isn't a colourbar (#545:
/// its sampled breaks wrap/shrink exactly like a discrete guide's keys do)
/// — used to split the available horizontal room among them when several
/// sit side by side (sipemu/ggplot-rs#155).
fn wrap_guide_count(scales: &ScaleSet, legend_scales: &[Aesthetic]) -> usize {
    legend_scales
        .iter()
        .filter(|aes| {
            let is_discrete = scales.get(aes).map(|s| s.is_discrete()).unwrap_or(false);
            is_discrete || !is_colorbar_aesthetic(aes)
        })
        .count()
        .max(1)
}

/// How many of a plot's legend scales are a true colourbar (a continuous
/// `Color`/`Fill` scale) — each reserves its own fixed width share of a
/// horizontal row (#170 / review #167 finding 7: a bottom legend mixing a
/// colourbar with a discrete guide ignored the colourbar's share, so the
/// discrete guide's auto-wrap/grid was planned against the *whole* row and
/// could still run off the right edge).
fn colorbar_guide_count(scales: &ScaleSet, legend_scales: &[Aesthetic]) -> usize {
    legend_scales
        .iter()
        .filter(|aes| {
            is_colorbar_aesthetic(aes) && !scales.get(aes).map(|s| s.is_discrete()).unwrap_or(false)
        })
        .count()
}

/// The fixed horizontal width one colourbar guide reserves in a
/// horizontal row — the same per-guide step `draw_legend_planned` already
/// advances `offset_x` by once it has drawn one (see its `is_horizontal`
/// arm for a colourbar).
fn colorbar_guide_width(theme: &Theme) -> f64 {
    // #211: only used where a colourbar guide shares a *horizontal* row
    // with another one — its footprint there is the horizontal bar's
    // own length (ticks/labels sit below it now, not to its side), the
    // same width `draw_legend_planned` steps `offset_x` by.
    let (bar_len, _) = continuous_bar_dims(theme, true);
    bar_len + theme.legend_spacing * 2.0
}

/// Plan every wrap-eligible legend scale's B3-2 layout (explicit
/// `nrow`/`ncol` grid, or auto-wrap) independently (sipemu/ggplot-rs#155):
/// previously this only ran for the plot's single discrete guide, leaving
/// a second simultaneous discrete guide (e.g. both `colour` and `shape`
/// mapped to discrete variables) always on the unwrapped 0.21 path. A
/// guide is wrap-eligible when it's discrete, or continuous but not a
/// colourbar (#545: `Size`/`Linewidth`/… sampled breaks wrap/shrink the
/// same way a discrete guide's keys do — only a true continuous
/// `Color`/`Fill` colourbar is exempt). When several wrap-eligible guides
/// sit side by side (`is_horizontal`), the available width is split evenly
/// among them first, so each one wraps against its own share rather than
/// the whole row (which would let it claim more room than its neighbours
/// leave). Guides that stack vertically instead each get the full
/// `avail_width`, since they don't compete for it. Returns one entry per
/// wrap-eligible guide that needs anything beyond the single-row/single-
/// column path — a guide absent from the map is byte-identical to
/// 0.21/0.22.
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
    let n_wrap = wrap_guide_count(scales, &legend_scales);
    let per_guide_width = if is_horizontal {
        // #170: leave the wrap-planned guides only the width left over
        // after every colourbar guide's own fixed share (a continuous
        // Color/Fill legend sitting in the same row) — previously
        // `avail_width` was split among the discrete guides alone, so a
        // colourbar plus a discrete guide could together still overflow
        // the row. #545: a continuous, non-colourbar guide (e.g. `Size`/
        // `Linewidth`) is now one of the guides this width is split among
        // too, rather than drawing unconstrained.
        let n_colorbar = colorbar_guide_count(scales, &legend_scales);
        let colorbar_width = n_colorbar as f64 * colorbar_guide_width(theme);
        ((avail_width - colorbar_width) / n_wrap as f64).max(0.0)
    } else {
        avail_width
    };

    let mut plans = std::collections::HashMap::new();
    for aes in &legend_scales {
        let Some(scale) = scales.get(aes) else {
            continue;
        };
        // #545: plan a continuous, non-colourbar guide (`Size`/
        // `Linewidth`/… sampled breaks) exactly like a discrete one —
        // only a true colourbar (continuous `Color`/`Fill`) is exempt
        // from B3-2's wrap/grid planning.
        if !scale.is_discrete() && is_colorbar_aesthetic(aes) {
            continue;
        }
        let breaks = scale.breaks();
        let validated_guide = guide.validated(breaks.len(), warnings);
        // #271: size against `breaks_for_layout()`, not `breaks` itself —
        // identical for every scale except `ScaleFillBinned` under
        // `count_reserve`, where it reserves width for the widest count
        // digit width so the resulting row count doesn't depend on which
        // frame's counts happen to be measured.
        let layout_breaks = scale.breaks_for_layout();
        if let Some(plan) = plan_legend(
            &layout_breaks,
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
/// An inside legend's own bounding box ([`LegendPosition::Inside`] only —
/// every other position reserves a dedicated band outside `plot_area`, so
/// it can never overlap panel-corner furniture living inside it), without
/// drawing anything (#273). `renderer.rs` calls this *before* any geom
/// draws, so a corner-furniture geom drawn first (`annotation_scale`/
/// `annotation_attribution`/`bivariate_legend`) can still stack clear of
/// where the legend — drawn last, see `draw_legend_planned` — will land.
/// Mirrors that function's own prefix (`legend_scales_for` →
/// `group_mergeable_legends` → `estimate_legend_box` → `legend_position`,
/// with `legend_extra_rows` fixed at `0` since `Inside` never reads it)
/// so this never disagrees with where the legend is actually drawn.
pub(crate) fn inside_legend_box(
    scales: &ScaleSet,
    theme: &Theme,
    plot_area: &Rect,
    canvas: &Rect,
    guide: &GuideLegend,
    suppressed: &std::collections::HashSet<Aesthetic>,
    legend_plans: &std::collections::HashMap<Aesthetic, LegendPlan>,
) -> Option<Rect> {
    if !matches!(theme.legend_position, LegendPosition::Inside(..)) {
        return None;
    }
    let legend_scales = legend_scales_for(scales, suppressed);
    if legend_scales.is_empty() {
        return None;
    }
    let groups = group_mergeable_legends(scales, &legend_scales, guide);
    let representatives: Vec<Aesthetic> = groups.iter().map(|g| g[0].clone()).collect();
    let legend_size_hint =
        estimate_legend_box(scales, &representatives, theme, guide, legend_plans);
    let lift = inside_title_lift(scales, &representatives, theme, guide);
    let (legend_x, legend_y, _) =
        legend_position(theme, plot_area, 0, legend_size_hint, lift, canvas);
    // The visible box, title included (#377).
    Some(Rect {
        x: legend_x,
        y: legend_y - lift,
        width: legend_size_hint.0,
        height: legend_size_hint.1 + lift,
    })
}

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

    // #239: group aesthetics whose legend would be visually identical
    // (same title, breaks, discrete/continuous-ness) into one combined
    // guide, rather than drawing one per aesthetic. Everything below that
    // used to iterate `legend_scales` one aesthetic at a time now
    // iterates `groups` instead; a group's first member (`representatives`)
    // stands in for the whole group wherever only one scale's own
    // breaks/title/discreteness is needed (sizing, the B3-2 wrap plan
    // lookup, …) — `group_mergeable_legends` only ever groups scales
    // whose `breaks()` already match, so any member would do.
    let groups = group_mergeable_legends(scales, &legend_scales, guide);
    let representatives: Vec<Aesthetic> = groups.iter().map(|g| g[0].clone()).collect();

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
    // #225: `Inside`'s justification needs the legend's own estimated
    // footprint and the full canvas to clamp against — cheap to compute
    // unconditionally (ignored by every other position). Estimated from
    // `representatives`, not every raw aesthetic, so a merged group's
    // height is counted once, not once per aesthetic in it.
    let legend_size_hint =
        estimate_legend_box(scales, &representatives, theme, guide, legend_plans);
    let canvas = backend.total_area();
    let (legend_x, legend_y, mut is_horizontal) = legend_position(
        theme,
        plot_area,
        legend_extra_rows,
        legend_size_hint,
        inside_title_lift(scales, &representatives, theme, guide),
        &canvas,
    );
    if let Some(dir) = theme.legend_direction {
        is_horizontal = matches!(dir, crate::theme::LegendDirection::Horizontal);
    }
    let n_wrap = wrap_guide_count(scales, &representatives);
    // #170: match `plan_all_legends`'s share — a colourbar guide (a
    // continuous Color/Fill legend) sharing this row has already claimed
    // its own fixed width, so step the wrap-planned guides by what's
    // left, not the whole `legend_avail_width`. #545: a continuous,
    // non-colourbar guide (`Size`/`Linewidth`/…) is one of the
    // wrap-planned guides this is split among now, not an unconstrained
    // extra.
    let n_colorbar = colorbar_guide_count(scales, &representatives);
    let colorbar_width = n_colorbar as f64 * colorbar_guide_width(theme);
    let per_guide_width = ((legend_avail_width - colorbar_width) / n_wrap as f64).max(0.0);

    let mut offset_y = legend_y;
    let mut offset_x = legend_x;

    for group in &groups {
        let representative = &group[0];
        let rep_scale = scales.get(representative).unwrap();
        let scale_group: Vec<(Aesthetic, &dyn crate::scale::Scale)> = group
            .iter()
            .map(|a| (a.clone(), scales.get(a).unwrap()))
            .collect();

        if !rep_scale.is_discrete() && is_colorbar_aesthetic(representative) {
            // A true colourbar (continuous Color/Fill) draws as a fixed-
            // width bar — not in scope for B3-2's wrap/grid planning.
            // Every other continuous legend (Size/Linewidth/… sampled
            // breaks, #545) falls through to the same wrap/grid-aware
            // path a discrete guide uses, below.
            //
            // #239: a merged continuous fill+colour pair draws one
            // bar — `group_mergeable_legends` only merges scales
            // whose breaks (so, in practice, colours) already match,
            // so the other member(s) would draw the identical bar
            // again.
            let height = draw_continuous_legend_at(
                rep_scale,
                theme,
                offset_x,
                offset_y,
                backend,
                guide,
                is_horizontal,
            )?;
            if is_horizontal {
                // #211: the bar is now drawn along x (wide, not
                // narrow) — step by its own length, not the old
                // vertical-bar width-plus-side-label estimate.
                let (bar_len, _) = continuous_bar_dims(theme, true);
                offset_x += bar_len + theme.legend_spacing * 2.0;
            } else {
                offset_y += height + theme.legend_spacing * 2.0;
            }
            continue;
        }

        // A discrete guide, or a continuous non-colourbar guide (#545:
        // `Size`/`Linewidth`/… sampled breaks draw the same way a
        // discrete guide's keys do) — both get the same B3-2 wrap/grid
        // plan when `plan_all_legends` gave them one.
        match legend_plans.get(representative) {
            None => {
                if is_horizontal {
                    let width = draw_discrete_legend_at(
                        scales,
                        &scale_group,
                        theme,
                        offset_x,
                        offset_y,
                        backend,
                        guide,
                        true,
                    )?;
                    offset_x += width + theme.legend_spacing * 2.0;
                } else {
                    let height = draw_discrete_legend_at(
                        scales,
                        &scale_group,
                        theme,
                        offset_x,
                        offset_y,
                        backend,
                        guide,
                        false,
                    )?;
                    offset_y += height + theme.legend_spacing * 2.0;
                }
            }
            Some(plan) => {
                let height = draw_discrete_legend_grid(
                    scales,
                    &scale_group,
                    theme,
                    offset_x,
                    offset_y,
                    backend,
                    guide,
                    plan,
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
/// Wrapped-grid counterpart of [`draw_discrete_legend_at`] — same `group`
/// merging (#239).
fn draw_discrete_legend_grid(
    scales: &ScaleSet,
    group: &[(Aesthetic, &dyn crate::scale::Scale)],
    theme: &Theme,
    legend_x: f64,
    legend_y: f64,
    backend: &mut dyn DrawBackend,
    guide: &GuideLegend,
    plan: &LegendPlan,
) -> Result<f64, RenderError> {
    let (_, scale) = &group[0];
    let mut breaks = scale.breaks();
    if breaks.is_empty() {
        return Ok(0.0);
    }
    if guide.reverse {
        breaks.reverse();
    }

    // #236: same key-box growth as `draw_discrete_legend_at`.
    let diameter = group
        .iter()
        .map(|(a, s)| size_key_diameter(a, scales, *s, &breaks))
        .fold(0.0_f64, f64::max);
    let swatch_size = theme.legend_key_width.max(diameter);
    let item_height = theme.legend_key_height.max(swatch_size);

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

        for (member_aes, member_scale) in group {
            draw_legend_swatch(
                backend,
                scales,
                member_aes,
                *member_scale,
                theme,
                label,
                *pos,
                x,
                y,
                center_x,
                center_y,
                swatch_size,
            )?;
        }

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

#[cfg(test)]
mod group_mergeable_legends_tests {
    use super::*;
    use crate::scale::color::RGBAColor;
    use crate::scale::manual::ScaleManual;

    #[test]
    fn an_aesthetic_with_no_attached_scale_is_skipped_not_grouped() {
        // #239: in the normal flow, `group_mergeable_legends` is only ever
        // handed `legend_scales_for`'s output, which already filters to
        // aesthetics present in `scales` — so the `let-else`'s `continue`
        // branch (an aesthetic that slipped through with no attached
        // scale) is otherwise unreachable from `draw_legend_planned`.
        // Exercise it directly: `Fill` is listed but has no scale.
        let mut scales = ScaleSet::new();
        scales.add(Box::new(ScaleManual::new(
            Aesthetic::Color,
            vec![("a", RGBAColor::new(200, 0, 0))],
        )));
        let legend_scales = vec![Aesthetic::Color, Aesthetic::Fill];
        let guide = GuideLegend::default();
        let groups = group_mergeable_legends(&scales, &legend_scales, &guide);
        assert_eq!(groups, vec![vec![Aesthetic::Color]]);
    }
}

/// #211: direct unit coverage for the two private helpers
/// `draw_continuous_legend_at`'s vertical/horizontal split relies on —
/// exercised indirectly by every colourbar-drawing integration test
/// already, but called here directly too so their own branches/math
/// are pinned regardless of how the compiler happens to inline them.
#[cfg(test)]
mod continuous_bar_dims_tests {
    use super::*;

    #[test]
    fn dims_swap_between_vertical_and_horizontal() {
        let theme = Theme::default();
        let (w, h) = continuous_bar_dims(&theme, false);
        assert_eq!(w, theme.legend_key_width);
        assert_eq!(h, theme.legend_key_height * 8.0);

        let (w, h) = continuous_bar_dims(&theme, true);
        assert_eq!(w, theme.legend_key_height * 8.0);
        assert_eq!(h, theme.legend_key_width);
    }

    #[test]
    fn horizontal_bar_height_adds_a_title_line_only_when_there_is_a_title() {
        let theme = Theme::default();
        let without_title = continuous_horizontal_bar_height(&theme, false);
        let with_title = continuous_horizontal_bar_height(&theme, true);
        assert!(with_title > without_title);
        assert!(((with_title - without_title) - (theme.legend_title.size + 4.0)).abs() < 1e-9);

        // Matches `continuous_bar_dims`'s own horizontal thickness plus
        // the fixed tick/spacing/label overhead — not just a positive
        // number, the exact formula `draw_continuous_legend_at` relies
        // on to never disagree with what `plot.rs` reserves for it.
        let (_, thickness) = continuous_bar_dims(&theme, true);
        let tick_len = 3.0;
        let label_line = theme.legend_text.size + 4.0;
        assert!(
            (without_title - (thickness + tick_len + theme.legend_spacing + label_line)).abs()
                < 1e-9
        );
    }
}

/// #273: `inside_legend_box` — the pre-draw measurement `renderer.rs` uses
/// to claim an inside legend's box before any corner-furniture geom draws.
#[cfg(test)]
mod inside_legend_box_tests {
    use super::*;
    use crate::scale::color::RGBAColor;
    use crate::scale::manual::ScaleManual;

    fn scales_with_a_legend() -> ScaleSet {
        let mut scales = ScaleSet::new();
        scales.add(Box::new(ScaleManual::new(
            Aesthetic::Color,
            vec![
                ("a", RGBAColor::new(200, 0, 0)),
                ("b", RGBAColor::new(0, 0, 200)),
            ],
        )));
        scales
    }

    fn areas() -> (Rect, Rect) {
        let plot_area = Rect {
            x: 10.0,
            y: 10.0,
            width: 200.0,
            height: 150.0,
        };
        let canvas = Rect {
            x: 0.0,
            y: 0.0,
            width: 220.0,
            height: 170.0,
        };
        (plot_area, canvas)
    }

    #[test]
    fn non_inside_positions_claim_nothing() {
        let scales = scales_with_a_legend();
        let (plot_area, canvas) = areas();
        let guide = GuideLegend::default();
        let suppressed = std::collections::HashSet::new();
        let legend_plans = std::collections::HashMap::new();
        for pos in [
            LegendPosition::Right,
            LegendPosition::Left,
            LegendPosition::Top,
            LegendPosition::Bottom,
            LegendPosition::None,
        ] {
            let theme = Theme {
                legend_position: pos,
                ..Theme::default()
            };
            assert!(inside_legend_box(
                &scales,
                &theme,
                &plot_area,
                &canvas,
                &guide,
                &suppressed,
                &legend_plans
            )
            .is_none());
        }
    }

    #[test]
    fn no_legend_scales_claims_nothing_even_when_inside() {
        let scales = ScaleSet::new(); // nothing mapped to a legend aesthetic
        let (plot_area, canvas) = areas();
        let theme = Theme {
            legend_position: LegendPosition::Inside(0.85, 0.85),
            ..Theme::default()
        };
        let guide = GuideLegend::default();
        let suppressed = std::collections::HashSet::new();
        let legend_plans = std::collections::HashMap::new();
        assert!(inside_legend_box(
            &scales,
            &theme,
            &plot_area,
            &canvas,
            &guide,
            &suppressed,
            &legend_plans
        )
        .is_none());
    }

    #[test]
    fn inside_position_with_a_legend_claims_a_box_within_the_panel() {
        let scales = scales_with_a_legend();
        let (plot_area, canvas) = areas();
        let theme = Theme {
            legend_position: LegendPosition::Inside(0.85, 0.85),
            ..Theme::default()
        };
        let guide = GuideLegend::default();
        let suppressed = std::collections::HashSet::new();
        let legend_plans = std::collections::HashMap::new();
        let rect = inside_legend_box(
            &scales,
            &theme,
            &plot_area,
            &canvas,
            &guide,
            &suppressed,
            &legend_plans,
        )
        .expect("an Inside legend with a mapped scale claims a box");
        assert!(rect.width > 0.0 && rect.height > 0.0);
        assert!(rect.x >= canvas.x - 1e-6 && rect.x + rect.width <= canvas.x + canvas.width + 1e-6);
        assert!(
            rect.y >= canvas.y - 1e-6 && rect.y + rect.height <= canvas.y + canvas.height + 1e-6
        );
    }

    #[test]
    fn matches_the_position_draw_legend_planned_itself_uses() {
        // Mirrors `draw_legend_planned`'s own prefix (`legend_scales_for`
        // → `group_mergeable_legends` → `estimate_legend_box` →
        // `legend_position`) — this must agree with it, or a geom drawn
        // before the legend could stack clear of the wrong spot.
        let scales = scales_with_a_legend();
        let (plot_area, canvas) = areas();
        let theme = Theme {
            legend_position: LegendPosition::Inside(0.85, 0.85),
            ..Theme::default()
        };
        let guide = GuideLegend::default();
        let suppressed = std::collections::HashSet::new();
        let legend_plans = std::collections::HashMap::new();

        let rect = inside_legend_box(
            &scales,
            &theme,
            &plot_area,
            &canvas,
            &guide,
            &suppressed,
            &legend_plans,
        )
        .unwrap();

        let legend_scales = legend_scales_for(&scales, &suppressed);
        let groups = group_mergeable_legends(&scales, &legend_scales, &guide);
        let representatives: Vec<Aesthetic> = groups.iter().map(|g| g[0].clone()).collect();
        let size_hint =
            estimate_legend_box(&scales, &representatives, &theme, &guide, &legend_plans);
        let lift = inside_title_lift(&scales, &representatives, &theme, &guide);
        let (x, y, _) = legend_position(&theme, &plot_area, 0, size_hint, lift, &canvas);

        // The visible box: the title's upper half above the origin (#377).
        assert_eq!(
            (rect.x, rect.y, rect.width, rect.height),
            (x, y - lift, size_hint.0, size_hint.1 + lift)
        );
    }
}
