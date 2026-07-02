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

        if let (Some(hs), Some(vs)) = (h_scale, v_scale) {
            if built.coord.gridlines() {
                axis::draw_gridlines(hs, vs, built.coord.as_ref(), theme, &plot_area, backend)?;
            }

            // 4. Draw axes
            axis::draw_x_axis(hs, built.coord.as_ref(), theme, &plot_area, backend)?;
            axis::draw_y_axis(vs, built.coord.as_ref(), theme, &plot_area, backend)?;

            // 4b. Draw secondary y axis if present
            if let Some(sec) = built.scales.sec_axis(&Aesthetic::Y) {
                axis::draw_sec_y_axis(vs, sec, built.coord.as_ref(), theme, &plot_area, backend)?;
            }
        }

        // 5. Draw each layer's geometry
        for layer in &built.layers {
            layer.geom.draw(
                &layer.data,
                built.coord.as_ref(),
                &built.scales,
                theme,
                backend,
            )?;
        }

        // 6. Draw annotations
        Self::draw_annotations(
            &built.annotations,
            &built.scales,
            built.coord.as_ref(),
            &plot_area,
            backend,
        )?;

        // 7. Draw title
        if let Some(ref title) = built.labels.title {
            let center_x = plot_area.x + plot_area.width / 2.0;
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
            let title_y = plot_area.y - theme.title.size * 0.9 - x_axis_lift;
            let family = if theme.title.family.is_empty() {
                None
            } else {
                Some(theme.title.family.clone())
            };
            backend.draw_text(
                title,
                (center_x, title_y.max(theme.title.size)),
                &TextStyle {
                    color: theme.title.color,
                    size: theme.title.size,
                    anchor: TextAnchor::Middle,
                    angle: 0.0,
                    family,
                },
            )?;
        }

        // 8. Draw subtitle
        if let Some(ref subtitle) = built.labels.subtitle {
            let center_x = plot_area.x + plot_area.width / 2.0;
            let subtitle_y = plot_area.y - 2.0;
            let family = if theme.subtitle.family.is_empty() {
                None
            } else {
                Some(theme.subtitle.family.clone())
            };
            backend.draw_text(
                subtitle,
                (
                    center_x,
                    subtitle_y.max(theme.title.size + theme.subtitle.size),
                ),
                &TextStyle {
                    color: theme.subtitle.color,
                    size: theme.subtitle.size,
                    anchor: TextAnchor::Middle,
                    angle: 0.0,
                    family,
                },
            )?;
        }

        // 9. Draw legend
        legend::draw_legend(
            &built.scales,
            theme,
            &plot_area,
            backend,
            &built.guide_legend,
            &built.suppressed_aes,
        )?;

        // 10. Draw caption
        if let Some(ref caption) = built.labels.caption {
            let right_x = plot_area.x + plot_area.width;
            let caption_y = total_area.y + total_area.height - theme.caption.size * 0.5;
            let family = if theme.caption.family.is_empty() {
                None
            } else {
                Some(theme.caption.family.clone())
            };
            backend.draw_text(
                caption,
                (right_x, caption_y),
                &TextStyle {
                    color: theme.caption.color,
                    size: theme.caption.size,
                    anchor: TextAnchor::End,
                    angle: 0.0,
                    family,
                },
            )?;
        }

        // 11. Draw corner tag (labs(tag = ...))
        Self::draw_tag(&built.labels, theme, &total_area, backend)?;

        Ok(())
    }

    /// Draw the corner tag label at the top-left of the plot area.
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
            backend.draw_text(
                tag,
                (
                    total_area.x + theme.title.size,
                    total_area.y + theme.title.size,
                ),
                &TextStyle {
                    color: theme.title.color,
                    size: theme.title.size,
                    anchor: TextAnchor::Start,
                    angle: 0.0,
                    family,
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
                    axis::draw_x_axis(xs, built.coord.as_ref(), theme, &panel_rect, backend)?;
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
                        built.layers[li].geom.draw(
                            layer_data,
                            built.coord.as_ref(),
                            panel_scale_set,
                            theme,
                            &mut panel_backend,
                        )?;
                    }
                }
            }
        }

        // Draw title
        if let Some(ref title) = built.labels.title {
            let center_x = plot_area.x + plot_area.width / 2.0;
            let title_y = plot_area.y - theme.title.size * 0.9;
            let family = if theme.title.family.is_empty() {
                None
            } else {
                Some(theme.title.family.clone())
            };
            backend.draw_text(
                title,
                (center_x, title_y.max(theme.title.size)),
                &TextStyle {
                    color: theme.title.color,
                    size: theme.title.size,
                    anchor: TextAnchor::Middle,
                    angle: 0.0,
                    family,
                },
            )?;
        }

        // Draw subtitle
        if let Some(ref subtitle) = built.labels.subtitle {
            let center_x = plot_area.x + plot_area.width / 2.0;
            let subtitle_y = plot_area.y - 2.0;
            let family = if theme.subtitle.family.is_empty() {
                None
            } else {
                Some(theme.subtitle.family.clone())
            };
            backend.draw_text(
                subtitle,
                (
                    center_x,
                    subtitle_y.max(theme.title.size + theme.subtitle.size),
                ),
                &TextStyle {
                    color: theme.subtitle.color,
                    size: theme.subtitle.size,
                    anchor: TextAnchor::Middle,
                    angle: 0.0,
                    family,
                },
            )?;
        }

        // Draw caption
        if let Some(ref caption) = built.labels.caption {
            let right_x = plot_area.x + plot_area.width;
            let caption_y = total_area.y + total_area.height - theme.caption.size * 0.5;
            let family = if theme.caption.family.is_empty() {
                None
            } else {
                Some(theme.caption.family.clone())
            };
            backend.draw_text(
                caption,
                (right_x, caption_y),
                &TextStyle {
                    color: theme.caption.color,
                    size: theme.caption.size,
                    anchor: TextAnchor::End,
                    angle: 0.0,
                    family,
                },
            )?;
        }

        // Draw annotations
        Self::draw_annotations(
            &built.annotations,
            &built.scales,
            built.coord.as_ref(),
            &plot_area,
            backend,
        )?;

        // Draw legend
        legend::draw_legend(
            &built.scales,
            theme,
            &plot_area,
            backend,
            &built.guide_legend,
            &built.suppressed_aes,
        )?;

        // Draw corner tag (labs(tag = ...))
        Self::draw_tag(&built.labels, theme, &total_area, backend)?;

        Ok(())
    }

    fn draw_annotations(
        annotations: &[Annotation],
        scales: &crate::scale::ScaleSet,
        coord: &dyn crate::coord::Coord,
        plot_area: &crate::render::Rect,
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
                            color: *color,
                            size: *size,
                            anchor: TextAnchor::Middle,
                            angle: 0.0,
                            family: None,
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
                            color: *color,
                            alpha: 1.0,
                            width: *width,
                            linetype: Linetype::Solid,
                        },
                    )?;
                }
            }
        }
        Ok(())
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
    fn total_area(&self) -> crate::render::Rect {
        self.inner.total_area()
    }
}
