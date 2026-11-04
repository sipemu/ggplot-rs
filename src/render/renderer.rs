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
    /// How far the title must rise to clear an axis above the panel: a
    /// top primary x axis, or a secondary x axis (#409).
    fn top_axis_lift(built: &BuiltPlot, theme: &crate::theme::Theme) -> f64 {
        let Some(xs) = built.scales.get(&Aesthetic::X) else {
            return 0.0;
        };
        if xs.axis_position_opposite() {
            theme.axis_ticks_length + theme.axis_text_x.size + theme.legend_spacing
        } else if let Some(sec) = xs.sec_axis().filter(|_| built.facet.is_none()) {
            axis::sec_x_axis_height(theme, sec)
        } else {
            0.0
        }
    }

    pub fn render(built: &BuiltPlot, backend: &mut dyn DrawBackend) -> Result<(), RenderError> {
        // #465: semantic classes on the non-data elements, when asked for.
        backend.enable_element_classes(built.svg_classes);
        let drawn = if !built.facet.is_none() && !built.panels.is_empty() {
            Self::render_faceted(built, backend)
        } else {
            Self::render_single(built, backend)
        };
        backend.set_element_class(None);
        backend.enable_element_classes(false);
        drawn
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
        if let Some(r) = built.coord.data_rect(&plot_area) {
            backend.set_viewport(&r);
        }
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
                backend.set_element_class(Some("gg-plot-background"));
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
                backend.set_element_class(Some("gg-panel-background"));
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
            backend.set_element_class(Some("gg-panel-border"));
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
        } else if let (true, Some(xs), Some(ys)) = (built.coord.is_polar(), x_scale, y_scale) {
            // Polar / radial (#411): rings, spokes and labels around the
            // circle; the axis titles stay at the bottom / left.
            crate::guide::polar::draw_polar_guides(
                xs,
                ys,
                built.coord.as_ref(),
                theme,
                &plot_area,
                backend,
            )?;
            axis::draw_x_axis_title(xs, theme, &plot_area, backend)?;
            let reserve = axis::y_label_reserve(theme, &ys.breaks());
            axis::draw_y_axis_title(ys, theme, &plot_area, reserve, backend)?;
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
            // 4c. A secondary x axis along the top (#409) — unless the
            // primary axis is already up there.
            if let Some(sec) = hs.sec_axis().filter(|_| !hs.axis_position_opposite()) {
                axis::draw_sec_x_axis(hs, sec, built.coord.as_ref(), theme, &plot_area, backend)?;
            }
        }

        // 5. Draw each layer's geometry
        // A layer whose data is empty (no input rows, or a stat that produced
        // nothing) simply draws nothing. #411: layers reaching past the
        // panel (a zoomed coord) are clipped to it (`clip = "on"`), or may
        // draw past it everywhere (`clip = "off"`).
        backend.set_element_class(None);
        let clip_group = built.clip
            && built
                .layers
                .iter()
                .any(|l| layer_spills(&l.data, &built.scales, built.coord.as_ref(), &plot_area));
        if clip_group {
            backend.begin_clip(&plot_area);
        }
        if !built.clip {
            backend.set_clipping(false);
        }
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
        if !built.clip {
            backend.set_clipping(true);
        }
        if clip_group {
            backend.end_clip();
        }

        // 5b. Gridlines on top of the data (R's panel.ontop).
        if theme.panel_ontop && built.coord.gridlines() {
            if let (Some(hs), Some(vs)) = (h_scale, v_scale) {
                axis::draw_gridlines(hs, vs, built.coord.as_ref(), theme, &plot_area, backend)?;
            }
        }

        backend.set_element_class(None);
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
        let caption_ref = Self::ref_rect(
            theme.caption_position.unwrap_or(theme.title_position),
            theme,
            &plot_area,
            &total_area,
        );
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
            let x_axis_lift = Self::top_axis_lift(built, theme);
            let (title_y, _) = Self::title_subtitle_y(
                theme,
                plot_area.y,
                x_axis_lift + legend_lift,
                built.labels.subtitle.is_some(),
            );
            let family = if theme.title.family.is_empty() {
                None
            } else {
                Some(theme.title.family.clone())
            };
            backend.set_element_class(Some("gg-title"));
            backend.draw_text(
                title,
                (tx, title_y),
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
            // #281: lifted above a Top legend's band; a top x-axis too.
            let x_axis_lift = Self::top_axis_lift(built, theme);
            let (_, subtitle_y) =
                Self::title_subtitle_y(theme, plot_area.y, x_axis_lift + legend_lift, true);
            let family = if theme.subtitle.family.is_empty() {
                None
            } else {
                Some(theme.subtitle.family.clone())
            };
            backend.set_element_class(Some("gg-subtitle"));
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
            let (cx, anchor) = Self::hjust_pos(&theme.caption, &caption_ref);
            // Inside the layout's `caption_area`: the bottom of the
            // canvas less `plot_margin.bottom` (#286).
            let caption_y = total_area.y + total_area.height
                - theme.plot_margin.bottom.max(0.0)
                - theme.caption.size * 0.5;
            let family = if theme.caption.family.is_empty() {
                None
            } else {
                Some(theme.caption.family.clone())
            };
            backend.set_element_class(Some("gg-caption"));
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
        // #413: `plot.tag.location = "panel"` puts the tag in the panel's
        // corner instead of the plot's.
        let tag_area = if theme.tag_location_panel {
            &plot_area
        } else {
            &total_area
        };
        Self::draw_tag(&built.labels, theme, tag_area, backend)?;

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

    /// Vertical centres `(title_y, subtitle_y)` of the title and subtitle
    /// (#384). Text is drawn `dominant-baseline="middle"`, so these are the
    /// line centres, stacked bottom-up from the panel top inside the bands
    /// `PlotLayout` reserves: the subtitle in its `1.5 × size` band directly
    /// above the panel (less `lift` — a top x-axis and/or a Top legend,
    /// #233/#281), the title above that. Without a subtitle the title keeps
    /// its old place, `0.9 × size` above the panel. The title is floored at
    /// `title.size` so it never leaves the canvas.
    fn title_subtitle_y(
        theme: &crate::theme::Theme,
        panel_top: f64,
        lift: f64,
        has_subtitle: bool,
    ) -> (f64, f64) {
        let base = panel_top - lift;
        let subtitle_band = if has_subtitle {
            theme.subtitle.size * 1.5
        } else {
            0.0
        };
        let subtitle_y = base - subtitle_band / 2.0;
        let title_y = (base - subtitle_band - theme.title.size * 0.9).max(theme.title.size);
        (title_y, subtitle_y)
    }

    /// Horizontal reference rect for titles: the panel or the whole plot width
    /// (R's `plot.title.position`).
    fn title_ref_rect(
        theme: &crate::theme::Theme,
        plot_area: &crate::render::Rect,
        total_area: &crate::render::Rect,
    ) -> crate::render::Rect {
        Self::ref_rect(theme.title_position, theme, plot_area, total_area)
    }

    /// The rectangle a title / caption aligns to under `pos` (#413: the
    /// caption can differ, `plot.caption.position`).
    fn ref_rect(
        pos: crate::theme::TitlePosition,
        theme: &crate::theme::Theme,
        plot_area: &crate::render::Rect,
        total_area: &crate::render::Rect,
    ) -> crate::render::Rect {
        match pos {
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
    /// [`draw_tag`](Self::draw_tag) for a non-plot grid cell (#378).
    pub(crate) fn draw_cell_tag(
        labels: &crate::plot::Labels,
        theme: &crate::theme::Theme,
        area: &crate::render::Rect,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        Self::draw_tag(labels, theme, area, backend)
    }

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
            backend.set_element_class(Some("gg-tag"));
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
                backend.set_element_class(Some("gg-plot-background"));
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
        let mut nrow = built.panels.len().div_ceil(ncol);

        // #412: a wrapped label (`FacetLabeller::Wrap`) takes several lines;
        // the strip grows to the tallest one.
        let strip_lines = |label: &dyn Fn(&crate::facet::Panel) -> Option<String>| {
            built
                .panels
                .iter()
                .filter_map(|p| label(p).map(|l| l.lines().count()))
                .max()
                .unwrap_or(1)
                .max(1)
        };
        // #413: horizontal strips size to `strip_text_x`, turned ones to
        // `strip_text_y` (each falling back to `strip_text`).
        let size_x = theme
            .strip_text_x
            .as_ref()
            .unwrap_or(&theme.strip_text)
            .size;
        let size_y = theme
            .strip_text_y
            .as_ref()
            .unwrap_or(&theme.strip_text)
            .size;
        let thickness = |lines: usize, size: f64| size * (1.0 + 1.2 * (lines - 1) as f64) + 8.0;
        // #412: `facet_grid` draws its column strips once (top, or bottom
        // when switched) and its row strips once (right, or left); a
        // `facet_wrap` panel carries its own strip on `strip_position`.
        let is_grid = matches!(built.facet, Facet::Grid { .. });
        let col_strip = |p: &crate::facet::Panel| {
            if is_grid {
                p.col_label.clone()
            } else {
                Some(p.label.clone())
            }
        };
        let row_strip = |p: &crate::facet::Panel| {
            if is_grid {
                p.row_label.clone()
            } else {
                None
            }
        };
        let row_strip_width = thickness(strip_lines(&row_strip), size_y);
        let (wrap_side, switch) = match &built.facet {
            Facet::Wrap { strip_position, .. } => {
                (*strip_position, crate::facet::FacetSwitch::None)
            }
            Facet::Grid { switch, .. } => (crate::facet::StripPosition::Top, *switch),
            Facet::None => (
                crate::facet::StripPosition::Top,
                crate::facet::FacetSwitch::None,
            ),
        };
        // A wrap strip on the left / right is as thick as a top one is tall.
        let wrap_vertical = matches!(
            wrap_side,
            crate::facet::StripPosition::Left | crate::facet::StripPosition::Right
        );
        // #412: axes on inner panels too (`facet_axes`, or a free scale
        // under `facet_wrap`, as in ggplot2) widen the gaps to hold them.
        let (free_x, free_y) = match &built.facet {
            Facet::Wrap { scales, .. } => (
                matches!(
                    scales,
                    crate::facet::FacetScales::FreeX | crate::facet::FacetScales::Free
                ),
                matches!(
                    scales,
                    crate::facet::FacetScales::FreeY | crate::facet::FacetScales::Free
                ),
            ),
            _ => (false, false),
        };
        let all_x = built.facet_axes.all_x || free_x;
        let all_y = built.facet_axes.all_y || free_y;
        // A free scale's inner axes are always labelled (they differ).
        let inner_labels_x = !built.facet_axes.margin_labels_x || free_x;
        let inner_labels_y = !built.facet_axes.margin_labels_y || free_y;
        let unlabelled = |x: bool| {
            let mut t = theme.clone();
            if x {
                t.axis_text_x.visible = false;
            } else {
                t.axis_text_y.visible = false;
            }
            t
        };
        let theme_no_x_labels = unlabelled(true);
        let theme_no_y_labels = unlabelled(false);
        let gap_x = theme.get_panel_spacing_x()
            + if all_y && built.panels.len() > 1 {
                theme.axis_ticks_length
                    + if inner_labels_y && theme.axis_text_y.visible {
                        built
                            .panel_scales
                            .iter()
                            .chain(std::iter::once(&built.scales))
                            .filter_map(|s| s.get(&Aesthetic::Y))
                            .map(|ys| axis::y_label_reserve(theme, &ys.breaks()))
                            .fold(0.0_f64, f64::max)
                    } else {
                        0.0
                    }
            } else {
                0.0
            };
        let gap_y = theme.get_panel_spacing_y()
            + if all_x && built.panels.len() > 1 {
                if inner_labels_x {
                    super::layout::PlotLayout::x_axis_band(theme, None)
                } else {
                    theme.axis_ticks_length
                }
            } else {
                0.0
            };
        let strip_height = thickness(
            strip_lines(&col_strip),
            if wrap_vertical { size_y } else { size_x },
        );

        // Figure-level layers (`GeomAttribution::figure`): one band under
        // the whole grid, which shrinks by its height.
        let is_figure = |li: usize| {
            built.layers[li]
                .geom
                .figure_band_height(plot_area.width, theme)
                .is_some()
        };
        let band_h: f64 = built
            .layers
            .iter()
            .filter_map(|l| l.geom.figure_band_height(plot_area.width, theme))
            .sum::<f64>()
            .min(plot_area.height * 0.5);
        let band_rect = crate::render::Rect {
            x: plot_area.x,
            y: plot_area.y + plot_area.height - band_h,
            width: plot_area.width,
            height: band_h,
        };
        let plot_area = crate::render::Rect {
            height: plot_area.height - band_h,
            ..plot_area
        };

        // `facet_wrap` under a fixed-aspect coord (`coord_sf` maps): equal
        // panels shaped like the data, in the column count that makes them
        // largest (or the given `ncol`), centred in the plot area — instead
        // of `sqrt(n)` arbitrary cells each letterboxing its map.
        let fixed_grid = match (&built.facet, built.coord.fixed_aspect()) {
            (Facet::Wrap { ncol: want, .. }, Some(aspect)) => Some(crate::facet::wrap::fit_grid(
                built.panels.len(),
                *want,
                aspect,
                plot_area.width
                    - if wrap_vertical {
                        strip_height * want.unwrap_or(1) as f64
                    } else {
                        0.0
                    },
                plot_area.height,
                if wrap_vertical { 0.0 } else { strip_height },
                gap_x,
                gap_y,
            )),
            _ => None,
        };
        let ncol = if let Some(g) = fixed_grid {
            nrow = g.nrow;
            g.ncol
        } else {
            ncol
        };
        // Each panel's (row, col): from its index under the fixed grid (the
        // build-time row/col assumed the `sqrt(n)` default), else as built.
        let cell: Vec<(usize, usize)> = built
            .panels
            .iter()
            .enumerate()
            .map(|(i, p)| {
                if fixed_grid.is_some() {
                    match &built.facet {
                        Facet::Wrap {
                            vertical, as_table, ..
                        } => crate::facet::wrap::wrap_cell(
                            i,
                            built.panels.len(),
                            ncol,
                            *vertical,
                            *as_table,
                        ),
                        _ => (i / ncol, i % ncol),
                    }
                } else {
                    (p.row, p.col)
                }
            })
            .collect();

        // Each panel's strips (#412): (side, label, thickness). The bands a
        // row / column reserves for them are the thickest strip it carries
        // on each side.
        use crate::facet::StripPosition as Side;
        // #412: `strip.placement = "outside"` — a bottom / left strip of a
        // panel that draws that axis sits beyond the axis (+ the pad).
        let outside = theme.strip_placement_outside;
        let x_axis_off =
            super::layout::PlotLayout::x_axis_band(theme, None) + theme.strip_switch_pad;
        let y_axis_off = theme.axis_ticks_length
            + built
                .panel_scales
                .iter()
                .chain(std::iter::once(&built.scales))
                .filter_map(|s| s.get(&Aesthetic::Y))
                .map(|ys| axis::y_label_reserve(theme, &ys.breaks()))
                .fold(0.0_f64, f64::max)
            + theme.strip_switch_pad;
        let n_panels = built.panels.len();
        let draws_x = |pi: usize, r: usize| r == nrow - 1 || pi + ncol >= n_panels || all_x;
        let draws_y = |c: usize| c == 0 || all_y;
        let off = |side: Side, pi: usize, r: usize, c: usize| match side {
            Side::Bottom if outside && draws_x(pi, r) => x_axis_off,
            Side::Left if outside && draws_y(c) => y_axis_off,
            _ => 0.0,
        };
        let strips: Vec<Vec<(Side, String, f64)>> = built
            .panels
            .iter()
            .enumerate()
            .map(|(pi, p)| {
                let (r, c) = cell[pi];
                let mut v = Vec::new();
                if is_grid {
                    if let Some(l) = &p.col_label {
                        if switch.x() && r == nrow - 1 {
                            v.push((Side::Bottom, l.clone(), strip_height));
                        } else if !switch.x() && r == 0 {
                            v.push((Side::Top, l.clone(), strip_height));
                        }
                    }
                    if let Some(l) = &p.row_label {
                        if switch.y() && c == 0 {
                            v.push((Side::Left, l.clone(), row_strip_width));
                        } else if !switch.y() && c == ncol - 1 {
                            v.push((Side::Right, l.clone(), row_strip_width));
                        }
                    }
                } else {
                    v.push((wrap_side, p.label.clone(), strip_height));
                }
                v
            })
            .collect();
        let mut band_top = vec![0.0_f64; nrow.max(1)];
        let mut band_bottom = vec![0.0_f64; nrow.max(1)];
        let mut band_left = vec![0.0_f64; ncol.max(1)];
        let mut band_right = vec![0.0_f64; ncol.max(1)];
        for (pi, list) in strips.iter().enumerate() {
            let (r, c) = cell[pi];
            for (side, _, t) in list {
                let o = off(*side, pi, r, c);
                let b = match side {
                    Side::Top => &mut band_top[r],
                    Side::Bottom => &mut band_bottom[r],
                    Side::Left => &mut band_left[c],
                    Side::Right => &mut band_right[c],
                };
                *b = b.max(*t + o);
            }
        }
        // Every row / column of a wrap keeps its strip band, even one whose
        // cells are all empty slots (a top band per row, as before #412).
        if !is_grid {
            match wrap_side {
                Side::Top => band_top.fill(strip_height),
                Side::Bottom => band_bottom
                    .iter_mut()
                    .for_each(|b| *b = b.max(strip_height)),
                Side::Left => band_left.iter_mut().for_each(|b| *b = b.max(strip_height)),
                Side::Right => band_right.fill(strip_height),
            }
        }

        // Proportional panel sizing (R's `space =`): free_x sizes columns to
        // their data x-range, free_y sizes rows to their y-range. Fixed = equal.
        let space = match &built.facet {
            Facet::Grid { space, .. } => space.clone(),
            // #412: a wrap sizes only along its single row / column.
            Facet::Wrap { space, .. } if fixed_grid.is_none() => {
                use crate::facet::FacetSpace as S;
                match (space.free_x() && nrow == 1, space.free_y() && ncol == 1) {
                    (true, true) => S::Free,
                    (true, false) => S::FreeX,
                    (false, true) => S::FreeY,
                    _ => S::Fixed,
                }
            }
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
        let avail_w = plot_area.width
            - gap_x * (ncol as f64 - 1.0)
            - band_left.iter().sum::<f64>()
            - band_right.iter().sum::<f64>();
        let col_widths: Vec<f64> = if let Some(g) = fixed_grid {
            vec![g.panel_w; ncol]
        } else if space.free_x() && ncol > 0 {
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
            let used_w: f64 = col_widths.iter().sum::<f64>()
                + gap_x * (ncol as f64 - 1.0)
                + band_left.iter().sum::<f64>()
                + band_right.iter().sum::<f64>();
            let mut acc = if fixed_grid.is_some() {
                plot_area.x + ((plot_area.width - used_w) / 2.0).max(0.0)
            } else {
                plot_area.x
            };
            for c in 0..ncol {
                col_x[c] = acc + band_left[c];
                acc += band_left[c] + col_widths[c] + band_right[c] + gap_x;
            }
        }

        let avail_h = plot_area.height
            - gap_y * (nrow as f64 - 1.0)
            - band_top.iter().sum::<f64>()
            - band_bottom.iter().sum::<f64>();
        let row_heights: Vec<f64> = if let Some(g) = fixed_grid {
            vec![g.panel_h; nrow]
        } else if space.free_y() && nrow > 0 {
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
            let used_h: f64 = row_heights.iter().sum::<f64>()
                + gap_y * (nrow as f64 - 1.0)
                + band_top.iter().sum::<f64>()
                + band_bottom.iter().sum::<f64>();
            let mut acc = if fixed_grid.is_some() {
                plot_area.y + ((plot_area.height - used_h) / 2.0).max(0.0)
            } else {
                plot_area.y
            };
            for r in 0..nrow {
                // `row_y` is the panel's own top edge.
                row_y[r] = acc + band_top[r];
                acc += band_top[r] + row_heights[r] + band_bottom[r] + gap_y;
            }
        }

        // #348: one x / y axis title for the whole grid (ggplot2), drawn
        // after the panels over the union of the panels carrying that axis.
        // The y title also needs the widest of those panels' tick labels.
        let mut x_title_area: Option<super::Rect> = None;
        let mut y_title_area: Option<(super::Rect, f64)> = None;
        let union = |a: &Option<super::Rect>, b: &super::Rect| match a {
            None => b.clone(),
            Some(a) => {
                let (x0, y0) = (a.x.min(b.x), a.y.min(b.y));
                let x1 = (a.x + a.width).max(b.x + b.width);
                let y1 = (a.y + a.height).max(b.y + b.height);
                super::Rect {
                    x: x0,
                    y: y0,
                    width: x1 - x0,
                    height: y1 - y0,
                }
            }
        };

        for (pi, panel) in built.panels.iter().enumerate() {
            let (prow, pcol) = cell[pi];
            let panel_width = col_widths[pcol];
            let panel_height = row_heights[prow];
            let px = col_x[pcol];
            let py = row_y[prow];

            let panel_rect = crate::render::Rect {
                x: px,
                y: py,
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

            // Strips (#412): each on its side of the panel, the side ones
            // with their text turned to read along them.
            // An inside strip pushes its axis out; an outside one sits
            // beyond the axis and only moves the axis title.
            let mut bottom_strip = 0.0_f64;
            let mut left_strip = 0.0_f64;
            let mut bottom_title = 0.0_f64;
            let mut left_title = 0.0_f64;
            for (side, label, t) in &strips[pi] {
                let o = off(*side, pi, prow, pcol);
                let (rect, angle) = match side {
                    Side::Top => ((px, py - t, panel_width, *t), 0.0),
                    Side::Bottom => {
                        if o > 0.0 {
                            bottom_title = *t + o;
                        } else {
                            bottom_strip = *t;
                        }
                        ((px, py + panel_height + o, panel_width, *t), 0.0)
                    }
                    Side::Left => {
                        if o > 0.0 {
                            left_title = *t + o;
                        } else {
                            left_strip = *t;
                        }
                        ((px - t - o, py, *t, panel_height), 270.0)
                    }
                    Side::Right => ((px + panel_width, py, *t, panel_height), 90.0),
                };
                draw_strip(backend, theme, rect, label, angle)?;
            }

            // Panel background
            if theme.panel_background.visible {
                if let Some(fill) = theme.panel_background.fill {
                    backend.set_element_class(Some("gg-panel-background"));
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
                backend.set_element_class(Some("gg-panel-border"));
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

            if let (true, Some(xs), Some(ys)) = (
                built.coord.is_polar() && !built.coord.is_radar(),
                x_scale,
                y_scale,
            ) {
                // Polar facets (#411): each panel its own rings / spokes.
                crate::guide::polar::draw_polar_guides(
                    xs,
                    ys,
                    built.coord.as_ref(),
                    theme,
                    &panel_rect,
                    backend,
                )?;
            } else if let (Some(xs), Some(ys)) = (x_scale, y_scale) {
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
                // A strip below / left of the panel sits between it and its
                // axis (ggplot2's `strip.placement = "inside"`, #412).
                let bottom_edge = prow == nrow - 1 || pi + ncol >= built.panels.len();
                if bottom_edge || all_x {
                    let axis_rect = crate::render::Rect {
                        height: panel_rect.height + bottom_strip,
                        ..panel_rect.clone()
                    };
                    let t = if bottom_edge || inner_labels_x {
                        theme
                    } else {
                        &theme_no_x_labels
                    };
                    axis::draw_x_axis_with(
                        xs,
                        built.coord.as_ref(),
                        t,
                        &axis_rect,
                        &total_area,
                        false,
                        backend,
                    )?;
                    if bottom_edge {
                        let title_rect = crate::render::Rect {
                            height: axis_rect.height + bottom_title,
                            ..axis_rect.clone()
                        };
                        x_title_area = Some(union(&x_title_area, &title_rect));
                    }
                }

                // Left column gets y axis (every column with `all_y`).
                if pcol == 0 || all_y {
                    let axis_rect = crate::render::Rect {
                        x: panel_rect.x - left_strip,
                        width: panel_rect.width + left_strip,
                        ..panel_rect.clone()
                    };
                    let t = if pcol == 0 || inner_labels_y {
                        theme
                    } else {
                        &theme_no_y_labels
                    };
                    axis::draw_y_axis_with(
                        ys,
                        built.coord.as_ref(),
                        t,
                        &axis_rect,
                        false,
                        backend,
                    )?;
                    if pcol == 0 {
                        let reserve = axis::y_label_reserve(theme, &ys.breaks());
                        let prev = y_title_area.as_ref().map(|(r, _)| r.clone());
                        let widest = y_title_area.as_ref().map_or(0.0, |(_, w)| *w);
                        let title_rect = crate::render::Rect {
                            x: axis_rect.x - left_title,
                            width: axis_rect.width + left_title,
                            ..axis_rect.clone()
                        };
                        y_title_area = Some((union(&prev, &title_rect), widest.max(reserve)));
                    }
                }
            }

            // Draw layers for this panel — under a fixed-aspect wrap (maps)
            // in their own clipped group, so one panel's features never
            // spill into the next.
            if fixed_grid.is_some() {
                let data = built
                    .coord
                    .data_rect(&panel_rect)
                    .unwrap_or_else(|| panel_rect.clone());
                backend.begin_panel(pi, &panel.label, &panel_rect, &data);
            }
            // #411: as for a single panel — clip this panel's layers when
            // they reach past it (a fixed-aspect grid's panel group already
            // clips), or let them draw past it with `clip = "off"`.
            backend.set_element_class(None);
            let clip_group = built.clip
                && fixed_grid.is_none()
                && built.panels_data.get(pi).is_some_and(|layers| {
                    layers.iter().any(|d| {
                        layer_spills(d, panel_scale_set, built.coord.as_ref(), &panel_rect)
                    })
                });
            if clip_group {
                backend.begin_clip(&panel_rect);
            }
            if !built.clip {
                backend.set_clipping(false);
            }
            if pi < built.panels_data.len() {
                for (li, layer_data) in built.panels_data[pi].iter().enumerate() {
                    if li < built.layers.len() && layer_data.nrows() > 0 && !is_figure(li) {
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
            if !built.clip {
                backend.set_clipping(true);
            }
            if clip_group {
                backend.end_clip();
            }
            if fixed_grid.is_some() {
                backend.end_panel();
            }
        }

        // The grid's single axis titles (#348), against the first panel's
        // scales (the titles are the same on every panel).
        let title_scales = built.panel_scales.first().unwrap_or(&built.scales);
        if let (Some(area), Some(xs)) = (&x_title_area, title_scales.get(&Aesthetic::X)) {
            axis::draw_x_axis_title(xs, theme, area, backend)?;
        }
        if let (Some((area, reserve)), Some(ys)) = (&y_title_area, title_scales.get(&Aesthetic::Y))
        {
            axis::draw_y_axis_title(ys, theme, area, *reserve, backend)?;
        }

        // Figure-level layers, once, in the band under the grid.
        if band_h > 0.0 {
            backend.clear_corner_claims();
            for li in (0..built.layers.len()).filter(|&li| is_figure(li)) {
                let empty = crate::data::DataFrame::new();
                let data = built
                    .panels_data
                    .iter()
                    .filter_map(|p| p.get(li))
                    .find(|d| d.nrows() > 0)
                    .unwrap_or(&empty);
                let mut band_backend = PanelBackendAdapter {
                    inner: backend,
                    panel_rect: band_rect.clone(),
                };
                let geom = &built.layers[li].geom;
                band_backend.set_mark_geom(Some(geom.name()));
                let drawn = geom.draw(
                    data,
                    built.coord.as_ref(),
                    &built.scales,
                    theme,
                    &mut band_backend,
                );
                band_backend.set_mark_geom(None);
                drawn?;
            }
        }

        // Draw title
        let title_ref = Self::title_ref_rect(theme, &plot_area, &total_area);
        let caption_ref = Self::ref_rect(
            theme.caption_position.unwrap_or(theme.title_position),
            theme,
            &plot_area,
            &total_area,
        );
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
            let (title_y, _) = Self::title_subtitle_y(
                theme,
                plot_area.y,
                legend_lift,
                built.labels.subtitle.is_some(),
            );
            let family = if theme.title.family.is_empty() {
                None
            } else {
                Some(theme.title.family.clone())
            };
            backend.set_element_class(Some("gg-title"));
            backend.draw_text(
                title,
                (tx, title_y),
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
            let (_, subtitle_y) = Self::title_subtitle_y(theme, plot_area.y, legend_lift, true);
            let family = if theme.subtitle.family.is_empty() {
                None
            } else {
                Some(theme.subtitle.family.clone())
            };
            backend.set_element_class(Some("gg-subtitle"));
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
            let (cx, anchor) = Self::hjust_pos(&theme.caption, &caption_ref);
            // Inside the layout's `caption_area`: the bottom of the
            // canvas less `plot_margin.bottom` (#286).
            let caption_y = total_area.y + total_area.height
                - theme.plot_margin.bottom.max(0.0)
                - theme.caption.size * 0.5;
            let family = if theme.caption.family.is_empty() {
                None
            } else {
                Some(theme.caption.family.clone())
            };
            backend.set_element_class(Some("gg-caption"));
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

        backend.set_element_class(None);
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
        // #413: `plot.tag.location = "panel"` puts the tag in the panel's
        // corner instead of the plot's.
        let tag_area = if theme.tag_location_panel {
            &plot_area
        } else {
            &total_area
        };
        Self::draw_tag(&built.labels, theme, tag_area, backend)?;

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
                Annotation::LogTicks {
                    sides,
                    outside,
                    lengths,
                    color,
                    width,
                } => {
                    let style = LineStyle {
                        color: annotation_ink(*color, theme),
                        alpha: 1.0,
                        width: *width,
                        linetype: Linetype::Solid,
                        ..Default::default()
                    };
                    let dir = if *outside { -1.0 } else { 1.0 };
                    for side in sides.chars() {
                        let horizontal = matches!(side, 'b' | 't');
                        let scale = if horizontal { x_scale } else { y_scale };
                        let Some((lo, hi)) = scale.and_then(|s| s.expanded_domain()) else {
                            continue;
                        };
                        for (t, kind) in log_ticks(lo, hi) {
                            let n = scale.map(|s| s.map(&Value::Float(t))).unwrap_or(0.0);
                            let len = dir
                                * match kind {
                                    0 => lengths.2,
                                    1 => lengths.1,
                                    _ => lengths.0,
                                };
                            // The tick's anchor on the panel edge, then inward.
                            let (edge, step) = match side {
                                'b' => ((n, 0.0), (0.0, -len)),
                                't' => ((n, 1.0), (0.0, len)),
                                'l' => ((0.0, n), (len, 0.0)),
                                'r' => ((1.0, n), (-len, 0.0)),
                                _ => continue,
                            };
                            let p0 = coord.transform(edge, plot_area);
                            let p1 = (p0.0 + step.0, p0.1 + step.1);
                            backend.draw_line(&[p0, p1], &style)?;
                        }
                    }
                }
                Annotation::Raster {
                    image,
                    xmin,
                    xmax,
                    ymin,
                    ymax,
                    alpha,
                } => {
                    let rect = data_rect(
                        x_scale,
                        y_scale,
                        coord,
                        plot_area,
                        [*xmin, *xmax, *ymin, *ymax],
                    );
                    backend.draw_image(image, &rect, plot_area, *alpha)?;
                }
                Annotation::Inset {
                    plot,
                    xmin,
                    xmax,
                    ymin,
                    ymax,
                } => {
                    let rect = data_rect(
                        x_scale,
                        y_scale,
                        coord,
                        plot_area,
                        [*xmin, *xmax, *ymin, *ymax],
                    );
                    let (w, h) = (rect.width.round().max(1.0), rect.height.round().max(1.0));
                    let mut guard = plot.0.lock().unwrap_or_else(|e| e.into_inner());
                    match &mut *guard {
                        Ok((built, meta)) => {
                            let layout =
                                crate::GGPlot::layout_built(built, meta, w as u32, h as u32);
                            let mut off = crate::render::offset::OffsetBackend::new(
                                backend,
                                rect.x,
                                rect.y,
                                layout.plot_area.clone(),
                                layout.total.clone(),
                            );
                            Self::render(built, &mut off)?;
                        }
                        Err(e) => {
                            backend.warn(format!("annotation_custom: inset plot failed: {e}"))
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

/// Log-10 tick positions inside `[lo, hi]` (log10 units): `(position, kind)`
/// with kind 0 = a power of ten (long), 1 = 5× (mid), 2 = the rest (short).
/// One facet strip: its background over `(x, y, w, h)` and its label lines
/// centred in it, turned by `angle` (90 = reads top to bottom, a right-hand
/// row strip; 270 = bottom to top, a left one) (#412).
fn draw_strip(
    backend: &mut dyn DrawBackend,
    theme: &crate::theme::Theme,
    (x, y, w, h): (f64, f64, f64, f64),
    label: &str,
    angle: f64,
) -> Result<(), RenderError> {
    // #413: a horizontal strip uses `strip_*_x`, a turned one `strip_*_y`.
    let horizontal = angle == 0.0;
    let (bg, text) = if horizontal {
        (&theme.strip_background_x, &theme.strip_text_x)
    } else {
        (&theme.strip_background_y, &theme.strip_text_y)
    };
    let bg = bg.as_ref().unwrap_or(&theme.strip_background);
    let text = text.as_ref().unwrap_or(&theme.strip_text);
    if bg.visible {
        backend.set_element_class(Some("gg-strip gg-strip-background"));
        backend.draw_rect(
            (x, y),
            (x + w, y + h),
            &RectStyle {
                fill: bg.fill,
                stroke: bg.color,
                stroke_width: bg.width,
                alpha: 1.0,
                clip: false,
                corner_radius: 0.0,
            },
        )?;
    }
    if !text.visible {
        return Ok(());
    }
    let family = if text.family.is_empty() {
        None
    } else {
        Some(text.family.clone())
    };
    let lines: Vec<&str> = label.lines().collect();
    let n = lines.len().max(1) as f64;
    for (k, line) in lines.iter().enumerate() {
        let d = (k as f64 - (n - 1.0) / 2.0) * text.size * 1.2;
        // Successive lines stack downwards when horizontal, and away from
        // the panel's reading direction when turned.
        let (dx, dy) = if angle == 90.0 {
            (-d, 0.0)
        } else if angle == 270.0 {
            (d, 0.0)
        } else {
            (0.0, d)
        };
        backend.set_element_class(Some("gg-strip gg-strip-text"));
        backend.draw_text(
            line,
            (x + w / 2.0 + dx, y + h / 2.0 + dy),
            &TextStyle {
                color: text.color,
                size: text.size,
                anchor: TextAnchor::Middle,
                angle,
                family: family.clone(),
                face: text.face,
                bg_colour: None,
                bg_r: 0.0,
            },
        )?;
    }
    Ok(())
}

fn log_ticks(lo: f64, hi: f64) -> Vec<(f64, u8)> {
    let mut out = Vec::new();
    if !(lo.is_finite() && hi.is_finite()) || hi - lo > 300.0 {
        return out;
    }
    for k in (lo.floor() as i64)..=(hi.ceil() as i64) {
        for m in 1..=9 {
            let t = k as f64 + (m as f64).log10();
            if t >= lo - 1e-9 && t <= hi + 1e-9 {
                out.push((
                    t,
                    if m == 1 {
                        0
                    } else if m == 5 {
                        1
                    } else {
                        2
                    },
                ));
            }
        }
    }
    out
}

/// The pixel rectangle of a data-space `[xmin, xmax, ymin, ymax]`
/// (non-finite bounds = the panel edges, as in ggplot2).
fn data_rect(
    x_scale: Option<&dyn crate::scale::Scale>,
    y_scale: Option<&dyn crate::scale::Scale>,
    coord: &dyn crate::coord::Coord,
    plot_area: &crate::render::Rect,
    [xmin, xmax, ymin, ymax]: [f64; 4],
) -> crate::render::Rect {
    use crate::data::Value;
    let m = |s: Option<&dyn crate::scale::Scale>, v: f64, edge: f64| {
        if v.is_finite() {
            s.map(|s| s.map(&Value::Float(v))).unwrap_or(edge)
        } else if v > 0.0 {
            1.0
        } else {
            0.0
        }
    };
    let a = coord.transform((m(x_scale, xmin, 0.0), m(y_scale, ymax, 1.0)), plot_area);
    let b = coord.transform((m(x_scale, xmax, 1.0), m(y_scale, ymin, 0.0)), plot_area);
    crate::render::Rect {
        x: a.0.min(b.0),
        y: a.1.min(b.1),
        width: (a.0 - b.0).abs(),
        height: (a.1 - b.1).abs(),
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

/// Does a layer's data reach past `area` (#411)? Its position columns
/// (`x`, `xmin`, …, `yend`), mapped and transformed like the geoms do,
/// tested against the panel with half a pixel of slack. A polar coord
/// never spills (its scales fill the circle).
fn layer_spills(
    data: &crate::data::DataFrame,
    scales: &crate::scale::ScaleSet,
    coord: &dyn crate::coord::Coord,
    area: &crate::render::Rect,
) -> bool {
    if coord.is_polar() || data.nrows() == 0 {
        return false;
    }
    const SLACK: f64 = 0.5;
    let outside = |(px, py): (f64, f64)| {
        px < area.x - SLACK
            || px > area.x + area.width + SLACK
            || py < area.y - SLACK
            || py > area.y + area.height + SLACK
    };
    let axes = [
        (Aesthetic::X, ["x", "xmin", "xmax", "xend"], true),
        (Aesthetic::Y, ["y", "ymin", "ymax", "yend"], false),
    ];
    for (aes, cols, is_x) in axes {
        let Some(scale) = scales.get(&aes) else {
            continue;
        };
        for col in cols {
            let Some(values) = data.column(col) else {
                continue;
            };
            for v in values {
                if !v.as_f64().is_some_and(f64::is_finite) {
                    continue;
                }
                let n = scale.map(v);
                if !n.is_finite() {
                    continue;
                }
                let p = if is_x { (n, 0.5) } else { (0.5, n) };
                if outside(coord.transform(p, area)) {
                    return true;
                }
            }
        }
    }
    false
}

/// Wrapper that overrides plot_area() to return the panel rect.
struct PanelBackendAdapter<'a> {
    inner: &'a mut dyn DrawBackend,
    panel_rect: crate::render::Rect,
}

impl<'a> DrawBackend for PanelBackendAdapter<'a> {
    fn set_clipping(&mut self, on: bool) {
        self.inner.set_clipping(on)
    }
    fn begin_clip(&mut self, rect: &crate::render::Rect) {
        self.inner.begin_clip(rect)
    }
    fn enable_element_classes(&mut self, on: bool) {
        self.inner.enable_element_classes(on)
    }
    fn set_element_class(&mut self, class: Option<&'static str>) {
        self.inner.set_element_class(class)
    }
    fn end_clip(&mut self) {
        self.inner.end_clip()
    }
    fn draw_image(
        &mut self,
        bytes: &[u8],
        rect: &crate::render::Rect,
        clip: &crate::render::Rect,
        alpha: f64,
    ) -> Result<(), RenderError> {
        self.inner.draw_image(bytes, rect, clip, alpha)
    }
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
