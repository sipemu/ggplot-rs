use crate::aes::Aesthetic;
use crate::annotate::Annotation;
use crate::build::BuiltPlot;
use crate::facet::Facet;
use crate::guide::{axis, legend};
use crate::render::backend::{DrawBackend, LineStyle, Linetype, RectStyle, TextAnchor, TextStyle};
use crate::render::RenderError;

/// Orchestrates rendering of a built plot.
pub struct PlotRenderer;

impl PlotRenderer {
    pub fn render(built: &BuiltPlot, backend: &mut dyn DrawBackend) -> Result<(), RenderError> {
        if !built.facet.is_none() && !built.panels.is_empty() {
            Self::render_faceted(built, backend)
        } else {
            Self::render_single(built, backend)
        }
    }

    fn render_single(built: &BuiltPlot, backend: &mut dyn DrawBackend) -> Result<(), RenderError> {
        let theme = &built.theme;
        let plot_area = backend.plot_area();
        let total_area = backend.total_area();
        // #273: fresh panel-corner claim registry for this panel — see
        // `render::corner_layout`. An inside legend draws *after* every
        // layer (step 9 below), so its box is pre-claimed here, before
        // any corner-furniture geom (annotation_scale/attribution/
        // bivariate_legend) in the layer loop gets a chance to draw.
        backend.clear_corner_claims();
        if let Some(rect) = legend::inside_legend_box(
            &built.scales,
            theme,
            &plot_area,
            &total_area,
            &built.guide_legend,
            &built.suppressed_aes,
            &built.legend_plans,
        ) {
            backend.claim_corner_box(rect);
        }

        // 1. Draw plot background
        if theme.plot_background.visible {
            if let Some(fill) = theme.plot_background.fill {
                backend.draw_rect(
                    (total_area.x, total_area.y),
                    (
                        total_area.x + total_area.width,
                        total_area.y + total_area.height,
                    ),
                    &RectStyle {
                        fill: Some(fill),
                        stroke: None,
                        stroke_width: 0.0,
                        alpha: 1.0,
                        clip: false,
                        corner_radius: 0.0,
                    },
                )?;
            }
        }

        // 2. Draw panel background
        if theme.panel_background.visible {
            if let Some(fill) = theme.panel_background.fill {
                backend.draw_rect(
                    (plot_area.x, plot_area.y),
                    (
                        plot_area.x + plot_area.width,
                        plot_area.y + plot_area.height,
                    ),
                    &RectStyle {
                        fill: Some(fill),
                        stroke: theme.panel_background.color,
                        stroke_width: theme.panel_background.width,
                        alpha: 1.0,
                        clip: false,
                        corner_radius: 0.0,
                    },
                )?;
            }
        }

        // 2b. Draw panel border
        if theme.panel_border.visible {
            let style = LineStyle {
                color: theme.panel_border.color,
                width: theme.panel_border.width,
                alpha: 1.0,
                linetype: Linetype::Solid,
                ..Default::default()
            };
            let x0 = plot_area.x;
            let y0 = plot_area.y;
            let x1 = plot_area.x + plot_area.width;
            let y1 = plot_area.y + plot_area.height;
            backend.draw_line(&[(x0, y0), (x1, y0)], &style)?;
            backend.draw_line(&[(x1, y0), (x1, y1)], &style)?;
            backend.draw_line(&[(x1, y1), (x0, y1)], &style)?;
            backend.draw_line(&[(x0, y1), (x0, y0)], &style)?;
        }

        // 3. Draw gridlines
        let x_scale = built.scales.get(&Aesthetic::X);
        let y_scale = built.scales.get(&Aesthetic::Y);

        let (h_scale, v_scale) = if built.coord.is_flipped() {
            (y_scale, x_scale)
        } else {
            (x_scale, y_scale)
        };

        if let (true, Some(xs), Some(ys)) = (built.coord.is_radar(), x_scale, y_scale) {
            // Radar: rings + spokes + labels replace cartesian grid/axes.
            crate::guide::polar::draw_radar_guides(
                xs,
                ys,
                built.coord.as_ref(),
                theme,
                &plot_area,
                backend,
            )?;
        } else if let (Some(hs), Some(vs)) = (h_scale, v_scale) {
            // Gridlines under the data, unless panel.ontop draws them over it.
            if built.coord.gridlines() && !theme.panel_ontop {
                axis::draw_gridlines(hs, vs, built.coord.as_ref(), theme, &plot_area, backend)?;
            }

            // 4. Draw axes
            axis::draw_x_axis(
                hs,
                built.coord.as_ref(),
                theme,
                &plot_area,
                &total_area,
                backend,
            )?;
            axis::draw_y_axis(vs, built.coord.as_ref(), theme, &plot_area, backend)?;

            // 4b. Draw secondary y axis if present
            if let Some(sec) = built.scales.sec_axis(&Aesthetic::Y) {
                axis::draw_sec_y_axis(vs, sec, built.coord.as_ref(), theme, &plot_area, backend)?;
            }
        }

        // 5. Draw each layer's geometry
        // A layer whose data is empty (no input rows, or a stat that produced
        // nothing) simply draws nothing.
        for layer in built.layers.iter().filter(|l| l.data.nrows() > 0) {
            backend.set_mark_geom(Some(layer.geom.name()));
            let drawn = layer.geom.draw(
                &layer.data,
                built.coord.as_ref(),
                &built.scales,
                theme,
                backend,
            );
            backend.set_mark_geom(None);
            drawn?;
        }

        // 5b. Gridlines on top of the data (R's panel.ontop).
        if theme.panel_ontop && built.coord.gridlines() {
            if let (Some(hs), Some(vs)) = (h_scale, v_scale) {
                axis::draw_gridlines(hs, vs, built.coord.as_ref(), theme, &plot_area, backend)?;
            }
        }

        // 6. Draw annotations
        Self::draw_annotations(
            &built.annotations,
            &built.scales,
            built.coord.as_ref(),
            &plot_area,
            theme,
            backend,
        )?;

        // 7. Draw title
        let title_ref = Self::title_ref_rect(theme, &plot_area, &total_area);
        // #233: a Top legend reserves its own band just above the panel,
        // same idea as a top x-axis's x_axis_lift below — lift title and
        // subtitle above it too, or they'd land inside the legend's band
        // instead of their own (zero when there's no Top legend to draw).
        let legend_lift = legend::top_legend_lift(
            theme,
            &built.scales,
            &built.suppressed_aes,
            &built.legend_plans,
            &built.guide_legend,
        );
        if let Some(ref title) = built.labels.title {
            let (tx, anchor) = Self::hjust_pos(&theme.title, &title_ref);
            // A top x-axis occupies the space just above the panel — lift the
            // title above it so they don't overlap.
            let x_axis_top = built
                .scales
                .get(&Aesthetic::X)
                .map(|s| s.axis_position_opposite())
                .unwrap_or(false);
            let x_axis_lift = if x_axis_top {
                theme.axis_ticks_length + theme.axis_text_x.size + theme.legend_spacing
            } else {
                0.0
            };
            let title_y = plot_area.y - theme.title.size * 0.9 - x_axis_lift - legend_lift;
            let family = if theme.title.family.is_empty() {
                None
            } else {
                Some(theme.title.family.clone())
            };
            backend.draw_text(
                title,
                (tx, title_y.max(theme.title.size)),
                &TextStyle {
                    color: theme.title.color,
                    size: theme.title.size,
                    anchor,
                    angle: 0.0,
                    family,
                    face: theme.title.face,
                    bg_colour: None,
                    bg_r: 0.0,
                },
            )?;
        }

        // 8. Draw subtitle
        if let Some(ref subtitle) = built.labels.subtitle {
            let (sx, anchor) = Self::hjust_pos(&theme.subtitle, &title_ref);
            // #281: apply the Top-legend lift *after* the no-title floor,
            // not before. `theme.title.size + theme.subtitle.size` is a
            // generic "don't draw above the top margin" minimum for the
            // legend-less case; subtracting `legend_lift` first let that
            // floor win over the correctly-lifted value whenever it was
            // bigger (always true once `title_height` drops to zero with
            // no title), silently cancelling the lift and landing the
            // subtitle back in the legend's own band instead of above it.
            let subtitle_y =
                (plot_area.y - 2.0).max(theme.title.size + theme.subtitle.size) - legend_lift;
            let family = if theme.subtitle.family.is_empty() {
                None
            } else {
                Some(theme.subtitle.family.clone())
            };
            backend.draw_text(
                subtitle,
                (sx, subtitle_y),
                &TextStyle {
                    color: theme.subtitle.color,
                    size: theme.subtitle.size,
                    anchor,
                    angle: 0.0,
                    family,
                    face: theme.subtitle.face,
                    bg_colour: None,
                    bg_r: 0.0,
                },
            )?;
        }

        // 9. Draw legend
        legend::draw_legend_planned(
            &built.scales,
            theme,
            &plot_area,
            plot_area.width,
            backend,
            &built.guide_legend,
            &built.suppressed_aes,
            &built.legend_plans,
        )?;

        // 10. Draw caption
        if let Some(ref caption) = built.labels.caption {
            let (cx, anchor) = Self::hjust_pos(&theme.caption, &title_ref);
            let caption_y = total_area.y + total_area.height - theme.caption.size * 0.5;
            let family = if theme.caption.family.is_empty() {
                None
            } else {
                Some(theme.caption.family.clone())
            };
            backend.draw_text(
                caption,
                (cx, caption_y),
                &TextStyle {
                    color: theme.caption.color,
                    size: theme.caption.size,
                    anchor,
                    angle: 0.0,
                    family,
                    face: theme.caption.face,
                    bg_colour: None,
                    bg_r: 0.0,
                },
            )?;
        }

        // 11. Draw corner tag (labs(tag = ...))
        Self::draw_tag(&built.labels, theme, &total_area, backend)?;

        Ok(())
    }

    /// Horizontal position + anchor for a theme text element, from its `hjust`
    /// (0 = left, 0.5 = center, 1 = right), relative to `area`.
    fn hjust_pos(
        el: &crate::theme::elements::ElementText,
        area: &crate::render::Rect,
    ) -> (f64, TextAnchor) {
        let x = area.x + el.hjust.clamp(0.0, 1.0) * area.width;
        let anchor = if el.hjust <= 0.02 {
            TextAnchor::Start
        } else if el.hjust >= 0.98 {
            TextAnchor::End
        } else {
            TextAnchor::Middle
        };
        (x, anchor)
    }

    /// Horizontal reference rect for titles: the panel or the whole plot width
    /// (R's `plot.title.position`).
    fn title_ref_rect(
        theme: &crate::theme::Theme,
        plot_area: &crate::render::Rect,
        total_area: &crate::render::Rect,
    ) -> crate::render::Rect {
        match theme.title_position {
            crate::theme::TitlePosition::Panel => plot_area.clone(),
            crate::theme::TitlePosition::Plot => crate::render::Rect {
                x: total_area.x + theme.plot_margin.left,
                y: plot_area.y,
                width: (total_area.width - theme.plot_margin.left - theme.plot_margin.right)
                    .max(1.0),
                height: plot_area.height,
            },
        }
    }

    /// Draw the corner tag label (position from `plot.tag.position`).
    fn draw_tag(
        labels: &crate::plot::Labels,
        theme: &crate::theme::Theme,
        total_area: &crate::render::Rect,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        if let Some(ref tag) = labels.tag {
            let family = if theme.title.family.is_empty() {
                None
            } else {
                Some(theme.title.family.clone())
            };
            use crate::theme::TagPosition::*;
            let pad = theme.title.size;
            let (left, right) = (total_area.x + pad, total_area.x + total_area.width - pad);
            let (top, bottom) = (total_area.y + pad, total_area.y + total_area.height - pad);
            let (x, y, anchor) = match theme.tag_position {
                TopLeft => (left, top, TextAnchor::Start),
                TopRight => (right, top, TextAnchor::End),
                BottomLeft => (left, bottom, TextAnchor::Start),
                BottomRight => (right, bottom, TextAnchor::End),
            };
            backend.draw_text(
                tag,
                (x, y),
                &TextStyle {
                    color: theme.title.color,
                    size: theme.title.size,
                    anchor,
                    angle: 0.0,
                    family,
                    face: theme.title.face,
                    bg_colour: None,
                    bg_r: 0.0,
                },
            )?;
        }
        Ok(())
    }

    fn render_faceted(built: &BuiltPlot, backend: &mut dyn DrawBackend) -> Result<(), RenderError> {
        let theme = &built.theme;
        let plot_area = backend.plot_area();
        let total_area = backend.total_area();

        // Draw plot background
        if theme.plot_background.visible {
            if let Some(fill) = theme.plot_background.fill {
                backend.draw_rect(
                    (total_area.x, total_area.y),
                    (
                        total_area.x + total_area.width,
                        total_area.y + total_area.height,
                    ),
                    &RectStyle {
                        fill: Some(fill),
                        stroke: None,
                        stroke_width: 0.0,
                        alpha: 1.0,
                        clip: false,
                        corner_radius: 0.0,
                    },
                )?;
            }
        }

        // Compute panel grid dimensions
        let ncol = match &built.facet {
            Facet::Wrap { ncol, .. } => {
                ncol.unwrap_or_else(|| (built.panels.len() as f64).sqrt().ceil() as usize)
            }
            Facet::Grid { .. } => built.panels.iter().map(|p| p.col).max().unwrap_or(0) + 1,
            Facet::None => 1,
        };
        let nrow = built.panels.len().div_ceil(ncol);

        let strip_height = theme.strip_text.size + 8.0;
        let gap_x = theme.get_panel_spacing_x();
        let gap_y = theme.get_panel_spacing_y();

        // Proportional panel sizing (R's `space =`): free_x sizes columns to
        // their data x-range, free_y sizes rows to their y-range. Fixed = equal.
        let space = match &built.facet {
            Facet::Grid { space, .. } => space.clone(),
            _ => crate::facet::FacetSpace::Fixed,
        };
        let extent = |pi: usize, col: &str| -> Option<f64> {
            let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
            for df in &built.panels_data[pi] {
                if let Some(c) = df.column(col) {
                    for v in c {
                        if let Some(f) = v.as_f64() {
                            lo = lo.min(f);
                            hi = hi.max(f);
                        }
                    }
                }
            }
            (lo <= hi).then_some(hi - lo)
        };
        let avail_w = plot_area.width - gap_x * (ncol as f64 - 1.0);
        let col_widths: Vec<f64> = if space.free_x() && ncol > 0 {
            let mut ranges = vec![0.0f64; ncol];
            for (pi, panel) in built.panels.iter().enumerate() {
                if let Some(r) = extent(pi, "x") {
                    ranges[panel.col] = ranges[panel.col].max(r);
                }
            }
            let total: f64 = ranges.iter().sum();
            if total > 0.0 {
                ranges.iter().map(|r| avail_w * (r / total)).collect()
            } else {
                vec![avail_w / ncol as f64; ncol]
            }
        } else {
            vec![avail_w / ncol.max(1) as f64; ncol.max(1)]
        };
        let mut col_x = vec![plot_area.x; ncol.max(1)];
        {
            let mut acc = plot_area.x;
            for c in 0..ncol {
                col_x[c] = acc;
                acc += col_widths[c] + gap_x;
            }
        }

        let avail_h = plot_area.height - gap_y * (nrow as f64 - 1.0) - strip_height * nrow as f64;
        let row_heights: Vec<f64> = if space.free_y() && nrow > 0 {
            let mut ranges = vec![0.0f64; nrow];
            for (pi, panel) in built.panels.iter().enumerate() {
                if let Some(r) = extent(pi, "y") {
                    ranges[panel.row] = ranges[panel.row].max(r);
                }
            }
            let total: f64 = ranges.iter().sum();
            if total > 0.0 {
                ranges.iter().map(|r| avail_h * (r / total)).collect()
            } else {
                vec![avail_h / nrow as f64; nrow]
            }
        } else {
            vec![avail_h / nrow.max(1) as f64; nrow.max(1)]
        };
        let mut row_y = vec![plot_area.y; nrow.max(1)];
        {
            let mut acc = plot_area.y;
            for r in 0..nrow {
                row_y[r] = acc;
                acc += row_heights[r] + strip_height + gap_y;
            }
        }

        for (pi, panel) in built.panels.iter().enumerate() {
            let panel_width = col_widths[panel.col];
            let panel_height = row_heights[panel.row];
            let px = col_x[panel.col];
            let py = row_y[panel.row];

            let panel_rect = crate::render::Rect {
                x: px,
                y: py + strip_height,
                width: panel_width,
                height: panel_height,
            };
            // #273: fresh panel-corner claim registry per facet panel —
            // see `render_single`'s identical reset. A faceted plot's
            // inside legend (if any) is drawn once globally, not per
            // panel, so it isn't pre-claimed here the way `render_single`
            // pre-claims it — map furniture geoms under faceting still
            // stack clear of each other within one panel, just not of a
            // faceted inside legend.
            backend.clear_corner_claims();

            // Strip label background
            if theme.strip_background.visible {
                backend.draw_rect(
                    (px, py),
                    (px + panel_width, py + strip_height),
                    &RectStyle {
                        fill: theme.strip_background.fill,
                        stroke: theme.strip_background.color,
                        stroke_width: theme.strip_background.width,
                        alpha: 1.0,
                        clip: false,
                        corner_radius: 0.0,
                    },
                )?;
            }

            // Strip label text
            if theme.strip_text.visible {
                let label = panel.col_label.as_deref().unwrap_or(&panel.label);
                let family = if theme.strip_text.family.is_empty() {
                    None
                } else {
                    Some(theme.strip_text.family.clone())
                };
                backend.draw_text(
                    label,
                    (px + panel_width / 2.0, py + strip_height / 2.0),
                    &TextStyle {
                        color: theme.strip_text.color,
                        size: theme.strip_text.size,
                        anchor: TextAnchor::Middle,
                        angle: 0.0,
                        family,
                        face: theme.strip_text.face,
                        bg_colour: None,
                        bg_r: 0.0,
                    },
                )?;
            }

            // Panel background
            if theme.panel_background.visible {
                if let Some(fill) = theme.panel_background.fill {
                    backend.draw_rect(
                        (panel_rect.x, panel_rect.y),
                        (
                            panel_rect.x + panel_rect.width,
                            panel_rect.y + panel_rect.height,
                        ),
                        &RectStyle {
                            fill: Some(fill),
                            stroke: theme.panel_background.color,
                            stroke_width: theme.panel_background.width,
                            alpha: 1.0,
                            clip: false,
                            corner_radius: 0.0,
                        },
                    )?;
                }
            }

            // Panel border
            if theme.panel_border.visible {
                let style = LineStyle {
                    color: theme.panel_border.color,
                    width: theme.panel_border.width,
                    alpha: 1.0,
                    linetype: Linetype::Solid,
                    ..Default::default()
                };
                let x0 = panel_rect.x;
                let y0 = panel_rect.y;
                let x1 = panel_rect.x + panel_rect.width;
                let y1 = panel_rect.y + panel_rect.height;
                backend.draw_line(&[(x0, y0), (x1, y0)], &style)?;
                backend.draw_line(&[(x1, y0), (x1, y1)], &style)?;
                backend.draw_line(&[(x1, y1), (x0, y1)], &style)?;
                backend.draw_line(&[(x0, y1), (x0, y0)], &style)?;
            }

            // Use per-panel scales if free facets, otherwise global scales
            let panel_scale_set = if pi < built.panel_scales.len() {
                &built.panel_scales[pi]
            } else {
                &built.scales
            };

            // Gridlines + axes for edge panels
            let x_scale = panel_scale_set.get(&Aesthetic::X);
            let y_scale = panel_scale_set.get(&Aesthetic::Y);

            if let (Some(xs), Some(ys)) = (x_scale, y_scale) {
                if built.coord.gridlines() {
                    axis::draw_gridlines(
                        xs,
                        ys,
                        built.coord.as_ref(),
                        theme,
                        &panel_rect,
                        backend,
                    )?;
                }

                // Bottom row gets x axis
                if panel.row == nrow - 1 || pi + ncol >= built.panels.len() {
                    axis::draw_x_axis(
                        xs,
                        built.coord.as_ref(),
                        theme,
                        &panel_rect,
                        &total_area,
                        backend,
                    )?;
                }

                // Left column gets y axis
                if panel.col == 0 {
                    axis::draw_y_axis(ys, built.coord.as_ref(), theme, &panel_rect, backend)?;
                }
            }

            // Draw layers for this panel
            if pi < built.panels_data.len() {
                for (li, layer_data) in built.panels_data[pi].iter().enumerate() {
                    if li < built.layers.len() && layer_data.nrows() > 0 {
                        let mut panel_backend = PanelBackendAdapter {
                            inner: backend,
                            panel_rect: panel_rect.clone(),
                        };
                        let geom = &built.layers[li].geom;
                        panel_backend.set_mark_geom(Some(geom.name()));
                        let drawn = geom.draw(
                            layer_data,
                            built.coord.as_ref(),
                            panel_scale_set,
                            theme,
                            &mut panel_backend,
                        );
                        panel_backend.set_mark_geom(None);
                        drawn?;
                    }
                }
            }
        }

        // Draw title
        let title_ref = Self::title_ref_rect(theme, &plot_area, &total_area);
        // #233: see render_single's identical lift — a Top legend must not
        // overlap the title/subtitle (zero when there's no Top legend).
        let legend_lift = legend::top_legend_lift(
            theme,
            &built.scales,
            &built.suppressed_aes,
            &built.legend_plans,
            &built.guide_legend,
        );
        if let Some(ref title) = built.labels.title {
            let (tx, anchor) = Self::hjust_pos(&theme.title, &title_ref);
            let title_y = plot_area.y - theme.title.size * 0.9 - legend_lift;
            let family = if theme.title.family.is_empty() {
                None
            } else {
                Some(theme.title.family.clone())
            };
            backend.draw_text(
                title,
                (tx, title_y.max(theme.title.size)),
                &TextStyle {
                    color: theme.title.color,
                    size: theme.title.size,
                    anchor,
                    angle: 0.0,
                    family,
                    face: theme.title.face,
                    bg_colour: None,
                    bg_r: 0.0,
                },
            )?;
        }

        // Draw subtitle
        if let Some(ref subtitle) = built.labels.subtitle {
            let (sx, anchor) = Self::hjust_pos(&theme.subtitle, &title_ref);
            // #281: see render_single's identical fix — apply legend_lift
            // after the no-title floor, not before.
            let subtitle_y =
                (plot_area.y - 2.0).max(theme.title.size + theme.subtitle.size) - legend_lift;
            let family = if theme.subtitle.family.is_empty() {
                None
            } else {
                Some(theme.subtitle.family.clone())
            };
            backend.draw_text(
                subtitle,
                (sx, subtitle_y),
                &TextStyle {
                    color: theme.subtitle.color,
                    size: theme.subtitle.size,
                    anchor,
                    angle: 0.0,
                    family,
                    face: theme.subtitle.face,
                    bg_colour: None,
                    bg_r: 0.0,
                },
            )?;
        }

        // Draw caption
        if let Some(ref caption) = built.labels.caption {
            let (cx, anchor) = Self::hjust_pos(&theme.caption, &title_ref);
            let caption_y = total_area.y + total_area.height - theme.caption.size * 0.5;
            let family = if theme.caption.family.is_empty() {
                None
            } else {
                Some(theme.caption.family.clone())
            };
            backend.draw_text(
                caption,
                (cx, caption_y),
                &TextStyle {
                    color: theme.caption.color,
                    size: theme.caption.size,
                    anchor,
                    angle: 0.0,
                    family,
                    face: theme.caption.face,
                    bg_colour: None,
                    bg_r: 0.0,
                },
            )?;
        }

        // Draw annotations
        Self::draw_annotations(
            &built.annotations,
            &built.scales,
            built.coord.as_ref(),
            &plot_area,
            theme,
            backend,
        )?;

        // Draw legend
        legend::draw_legend_planned(
            &built.scales,
            theme,
            &plot_area,
            plot_area.width,
            backend,
            &built.guide_legend,
            &built.suppressed_aes,
            &built.legend_plans,
        )?;

        // Draw corner tag (labs(tag = ...))
        Self::draw_tag(&built.labels, theme, &total_area, backend)?;

        Ok(())
    }

    /// `annotate_text`/`annotate_segment` under theme ink/paper (#157,
    /// follow-up to #142): `Annotation`'s fields are all `pub` with no
    /// `#[non_exhaustive]`, so it can't gain an "explicitly styled" flag
    /// without a semver break — the same `value == hard-coded default`
    /// gate `apply_element_geom`'s other callers already use (e.g.
    /// `geom/refline.rs`'s `(50, 50, 50)` → `ink`) is applied here instead.
    /// Default theme (`geom.ink` unset) is byte-identical.
    ///
    /// `annotate_rect`'s grey fill is deliberately excluded: checked
    /// against ggplot2 4.0.3 (`theme(geom = element_geom(ink = "white",
    /// paper = "black"))`), `annotate("text")`/`annotate("segment")`
    /// recolour to `ink` but `annotate("rect")`'s fill stays the same
    /// hard-coded grey (`#A6A6A6`) regardless — ggplot2 itself doesn't
    /// route a rect annotation's fill through ink/paper.
    fn draw_annotations(
        annotations: &[Annotation],
        scales: &crate::scale::ScaleSet,
        coord: &dyn crate::coord::Coord,
        plot_area: &crate::render::Rect,
        theme: &crate::theme::Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        use crate::data::Value;

        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        for ann in annotations {
            match ann {
                Annotation::Text {
                    label,
                    x,
                    y,
                    size,
                    color,
                } => {
                    let nx = x_scale.map(|s| s.map(&Value::Float(*x))).unwrap_or(0.0);
                    let ny = y_scale.map(|s| s.map(&Value::Float(*y))).unwrap_or(0.0);
                    let pos = coord.transform((nx, ny), plot_area);
                    backend.draw_text(
                        label,
                        pos,
                        &TextStyle {
                            color: annotation_ink(*color, theme),
                            size: *size,
                            anchor: TextAnchor::Middle,
                            angle: 0.0,
                            family: crate::geom::resolve_font_family("", theme),
                            face: crate::render::backend::FontFace::Plain,
                            bg_colour: None,
                            bg_r: 0.0,
                        },
                    )?;
                }
                Annotation::Rect {
                    xmin,
                    xmax,
                    ymin,
                    ymax,
                    fill,
                    alpha,
                } => {
                    let nx0 = x_scale.map(|s| s.map(&Value::Float(*xmin))).unwrap_or(0.0);
                    let nx1 = x_scale.map(|s| s.map(&Value::Float(*xmax))).unwrap_or(1.0);
                    let ny0 = y_scale.map(|s| s.map(&Value::Float(*ymin))).unwrap_or(0.0);
                    let ny1 = y_scale.map(|s| s.map(&Value::Float(*ymax))).unwrap_or(1.0);
                    let tl = coord.transform((nx0, ny1), plot_area);
                    let br = coord.transform((nx1, ny0), plot_area);
                    backend.draw_rect(
                        tl,
                        br,
                        &RectStyle {
                            fill: Some(*fill),
                            stroke: None,
                            stroke_width: 0.0,
                            alpha: *alpha,
                            clip: false,
                            corner_radius: 0.0,
                        },
                    )?;
                }
                Annotation::Segment {
                    x,
                    y,
                    xend,
                    yend,
                    color,
                    width,
                } => {
                    let nx0 = x_scale.map(|s| s.map(&Value::Float(*x))).unwrap_or(0.0);
                    let ny0 = y_scale.map(|s| s.map(&Value::Float(*y))).unwrap_or(0.0);
                    let nx1 = x_scale.map(|s| s.map(&Value::Float(*xend))).unwrap_or(1.0);
                    let ny1 = y_scale.map(|s| s.map(&Value::Float(*yend))).unwrap_or(1.0);
                    let p0 = coord.transform((nx0, ny0), plot_area);
                    let p1 = coord.transform((nx1, ny1), plot_area);
                    backend.draw_line(
                        &[p0, p1],
                        &LineStyle {
                            color: annotation_ink(*color, theme),
                            alpha: 1.0,
                            width: *width,
                            linetype: Linetype::Solid,
                            ..Default::default()
                        },
                    )?;
                }
            }
        }
        Ok(())
    }
}

/// `color`, recoloured from `theme.geom.ink` when it's still at
/// `annotate_text`/`annotate_segment`'s hard-coded default (`(50, 50,
/// 50)`) and the theme actually sets `ink` — see the doc comment on
/// [`PlotRenderer::draw_annotations`]. `None`/an explicitly-styled colour
/// (anything else) passes through unchanged.
fn annotation_ink(color: (u8, u8, u8), theme: &crate::theme::Theme) -> (u8, u8, u8) {
    match (color == (50, 50, 50), theme.geom.ink) {
        (true, Some(ink)) => ink,
        _ => color,
    }
}

/// Wrapper that overrides plot_area() to return the panel rect.
struct PanelBackendAdapter<'a> {
    inner: &'a mut dyn DrawBackend,
    panel_rect: crate::render::Rect,
}

impl<'a> DrawBackend for PanelBackendAdapter<'a> {
    fn draw_circle(
        &mut self,
        center: (f64, f64),
        radius: f64,
        style: &crate::render::backend::PointStyle,
    ) -> Result<(), RenderError> {
        self.inner.draw_circle(center, radius, style)
    }
    fn draw_line(
        &mut self,
        points: &[(f64, f64)],
        style: &crate::render::backend::LineStyle,
    ) -> Result<(), RenderError> {
        self.inner.draw_line(points, style)
    }
    fn draw_quadratic(
        &mut self,
        p0: (f64, f64),
        ctrl: (f64, f64),
        p1: (f64, f64),
        style: &crate::render::backend::LineStyle,
    ) -> Result<(), RenderError> {
        self.inner.draw_quadratic(p0, ctrl, p1, style)
    }
    fn draw_rect(
        &mut self,
        top_left: (f64, f64),
        bottom_right: (f64, f64),
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        self.inner.draw_rect(top_left, bottom_right, style)
    }
    fn draw_text(
        &mut self,
        text: &str,
        pos: (f64, f64),
        style: &TextStyle,
    ) -> Result<(), RenderError> {
        self.inner.draw_text(text, pos, style)
    }
    fn draw_polygon(
        &mut self,
        points: &[(f64, f64)],
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        self.inner.draw_polygon(points, style)
    }
    fn draw_polygon_rings(
        &mut self,
        rings: &[Vec<(f64, f64)>],
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        self.inner.draw_polygon_rings(rings, style)
    }
    fn draw_shape(
        &mut self,
        center: (f64, f64),
        radius: f64,
        style: &crate::render::backend::PointStyle,
    ) -> Result<(), RenderError> {
        self.inner.draw_shape(center, radius, style)
    }
    fn plot_area(&self) -> crate::render::Rect {
        self.panel_rect.clone()
    }
    fn set_tooltip(&mut self, tooltip: Option<String>) {
        self.inner.set_tooltip(tooltip)
    }
    fn set_mark_axis(&mut self, key: Option<String>) {
        self.inner.set_mark_axis(key)
    }
    fn set_mark_series(&mut self, series: Option<String>) {
        self.inner.set_mark_series(series)
    }
    fn set_mark_value(&mut self, value: Option<String>) {
        self.inner.set_mark_value(value)
    }
    fn set_mark_geom(&mut self, geom: Option<&str>) {
        self.inner.set_mark_geom(geom)
    }
    fn warn(&mut self, message: String) {
        self.inner.warn(message)
    }
    fn set_mark_part(&mut self, part: Option<&str>) {
        self.inner.set_mark_part(part)
    }
    fn set_mark_attrs(&mut self, attrs: Vec<(String, String)>) {
        self.inner.set_mark_attrs(attrs)
    }

    fn set_mark_class(&mut self, class: Option<String>) {
        self.inner.set_mark_class(class)
    }

    fn set_mark_group(&mut self, group: Option<(String, String)>) {
        self.inner.set_mark_group(group)
    }

    fn set_layer_attrs(&mut self, attrs: Vec<(String, String)>) {
        self.inner.set_layer_attrs(attrs)
    }
    fn wants_formatted_values(&self) -> bool {
        self.inner.wants_formatted_values()
    }
    fn set_mark_fvalue(&mut self, value: Option<String>) {
        self.inner.set_mark_fvalue(value)
    }
    fn set_mark_fx(&mut self, value: Option<String>) {
        self.inner.set_mark_fx(value)
    }
    fn total_area(&self) -> crate::render::Rect {
        self.inner.total_area()
    }
    fn corner_claims(&self) -> Vec<crate::render::Rect> {
        self.inner.corner_claims()
    }
    fn claim_corner_box(&mut self, rect: crate::render::Rect) {
        self.inner.claim_corner_box(rect)
    }
    fn clear_corner_claims(&mut self) {
        self.inner.clear_corner_claims()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::backend::LineStyle;
    use crate::render::svg_backend::SvgBackend;

    /// `PanelBackendAdapter` (every geom draws through one — see the panel
    /// loop in `PlotRenderer::render`) forwards `draw_quadratic`,
    /// `set_mark_part` and `set_mark_attrs` to its inner backend, and
    /// reports the *panel* rect (not the inner backend's full area) as its
    /// `plot_area`.
    #[test]
    fn panel_backend_adapter_forwards_quadratic_and_mark_metadata() {
        let total = crate::render::Rect {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 200.0,
        };
        let panel_rect = crate::render::Rect {
            x: 10.0,
            y: 10.0,
            width: 80.0,
            height: 80.0,
        };
        let mut backend = SvgBackend::new(200, 200, total);
        let mut adapter = PanelBackendAdapter {
            inner: &mut backend,
            panel_rect: panel_rect.clone(),
        };

        assert_eq!(adapter.plot_area().x, panel_rect.x);
        assert_eq!(adapter.plot_area().width, panel_rect.width);

        adapter.set_mark_part(Some("arrow"));
        adapter.set_mark_attrs(vec![("data-x".to_string(), "v".to_string())]);
        adapter
            .draw_quadratic((0.0, 0.0), (5.0, 10.0), (10.0, 0.0), &LineStyle::default())
            .unwrap();
        adapter.set_mark_part(None);
        adapter.set_mark_attrs(Vec::new());

        let svg = backend.finish();
        assert!(svg.contains("<path"));
        assert!(svg.contains("data-part=\"arrow\""));
        assert!(svg.contains("data-x=\"v\""));
    }
}
