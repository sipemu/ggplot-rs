use plotters::prelude::*;

use super::backend::{DrawBackend, LineStyle, PointShape, PointStyle, RectStyle, TextAnchor, TextStyle};
use super::{Rect, RenderError};

/// Adapter from plotters' DrawingArea to our DrawBackend trait.
pub struct PlottersAdapter<'a, DB: DrawingBackend> {
    area: &'a DrawingArea<DB, plotters::coord::Shift>,
    plot_area: Rect,
    total_area: Rect,
}

impl<'a, DB: DrawingBackend> PlottersAdapter<'a, DB> {
    pub fn new(area: &'a DrawingArea<DB, plotters::coord::Shift>, plot_area: Rect) -> Self {
        let (w, h) = area.dim_in_pixel();
        PlottersAdapter {
            area,
            plot_area,
            total_area: Rect {
                x: 0.0,
                y: 0.0,
                width: w as f64,
                height: h as f64,
            },
        }
    }
}

fn to_rgba(color: (u8, u8, u8), alpha: f64) -> RGBAColor {
    RGBAColor(color.0, color.1, color.2, alpha)
}

fn map_err<E: std::fmt::Debug>(e: E) -> RenderError {
    RenderError::BackendError(format!("{:?}", e))
}

/// Segment a polyline according to a dash pattern, returning visible sub-paths.
fn segment_dashed(points: &[(f64, f64)], pattern: &[(f64, f64)]) -> Vec<Vec<(f64, f64)>> {
    if pattern.is_empty() || points.len() < 2 {
        return vec![points.to_vec()];
    }

    let mut segments: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut current_seg: Vec<(f64, f64)> = Vec::new();
    let mut drawing = true;
    let mut pat_idx = 0;
    let mut remaining_in_pat = pattern[0].0; // start with draw phase

    for window in points.windows(2) {
        let (x0, y0) = window[0];
        let (x1, y1) = window[1];
        let dx = x1 - x0;
        let dy = y1 - y0;
        let seg_len = (dx * dx + dy * dy).sqrt();
        if seg_len < 0.001 {
            continue;
        }
        let ux = dx / seg_len;
        let uy = dy / seg_len;
        let mut consumed = 0.0;

        while consumed < seg_len - 0.001 {
            let available = seg_len - consumed;
            let step = remaining_in_pat.min(available);
            let px = x0 + ux * (consumed + step);
            let py = y0 + uy * (consumed + step);

            if drawing {
                if current_seg.is_empty() {
                    current_seg.push((x0 + ux * consumed, y0 + uy * consumed));
                }
                current_seg.push((px, py));
            }

            consumed += step;
            remaining_in_pat -= step;

            if remaining_in_pat < 0.001 {
                if drawing {
                    if current_seg.len() >= 2 {
                        segments.push(std::mem::take(&mut current_seg));
                    } else {
                        current_seg.clear();
                    }
                    drawing = false;
                    remaining_in_pat = pattern[pat_idx].1; // gap
                } else {
                    drawing = true;
                    pat_idx = (pat_idx + 1) % pattern.len();
                    remaining_in_pat = pattern[pat_idx].0; // draw
                }
            }
        }
    }

    if drawing && current_seg.len() >= 2 {
        segments.push(current_seg);
    }

    segments
}

impl<'a, DB: DrawingBackend> DrawBackend for PlottersAdapter<'a, DB> {
    fn draw_circle(
        &mut self,
        center: (f64, f64),
        radius: f64,
        style: &PointStyle,
    ) -> Result<(), RenderError> {
        let color = to_rgba(style.color, style.alpha);
        if style.filled {
            self.area
                .draw(&Circle::new(
                    (center.0 as i32, center.1 as i32),
                    radius as i32,
                    color.filled(),
                ))
                .map_err(map_err)?;
        } else {
            self.area
                .draw(&Circle::new(
                    (center.0 as i32, center.1 as i32),
                    radius as i32,
                    color.stroke_width(1),
                ))
                .map_err(map_err)?;
        }
        Ok(())
    }

    fn draw_line(
        &mut self,
        points: &[(f64, f64)],
        style: &LineStyle,
    ) -> Result<(), RenderError> {
        if points.len() < 2 {
            return Ok(());
        }
        // Simulate sub-pixel line widths: render as 1px with reduced opacity
        let (pixel_width, alpha) = if style.width >= 1.0 {
            (style.width as u32, style.alpha)
        } else if style.width > 0.0 {
            (1, style.alpha * style.width)
        } else {
            (0, style.alpha)
        };
        let color = to_rgba(style.color, alpha);
        let stroke = color.stroke_width(pixel_width);

        let pattern = style.linetype.pattern();
        let sub_paths = segment_dashed(points, pattern);

        for path in &sub_paths {
            for window in path.windows(2) {
                let p1 = (window[0].0 as i32, window[0].1 as i32);
                let p2 = (window[1].0 as i32, window[1].1 as i32);
                self.area
                    .draw(&PathElement::new(vec![p1, p2], stroke))
                    .map_err(map_err)?;
            }
        }
        Ok(())
    }

    fn draw_rect(
        &mut self,
        top_left: (f64, f64),
        bottom_right: (f64, f64),
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        let tl = (top_left.0 as i32, top_left.1 as i32);
        let br = (bottom_right.0 as i32, bottom_right.1 as i32);

        if let Some(fill) = style.fill {
            let fill_color = to_rgba(fill, style.alpha);
            self.area
                .draw(&plotters::prelude::Rectangle::new(
                    [tl, br],
                    fill_color.filled(),
                ))
                .map_err(map_err)?;
        }

        if let Some(stroke) = style.stroke {
            let stroke_color = to_rgba(stroke, style.alpha);
            self.area
                .draw(&plotters::prelude::Rectangle::new(
                    [tl, br],
                    stroke_color.stroke_width(if style.stroke_width > 0.0 { (style.stroke_width as u32).max(1) } else { 0 }),
                ))
                .map_err(map_err)?;
        }

        Ok(())
    }

    fn draw_text(
        &mut self,
        text: &str,
        pos: (f64, f64),
        style: &TextStyle,
    ) -> Result<(), RenderError> {
        let color = to_rgba(style.color, 1.0);
        let font = ("sans-serif", style.size).into_font();

        let pos_adj = match style.anchor {
            TextAnchor::Start => plotters::style::text_anchor::Pos::new(
                plotters::style::text_anchor::HPos::Left,
                plotters::style::text_anchor::VPos::Center,
            ),
            TextAnchor::Middle => plotters::style::text_anchor::Pos::new(
                plotters::style::text_anchor::HPos::Center,
                plotters::style::text_anchor::VPos::Center,
            ),
            TextAnchor::End => plotters::style::text_anchor::Pos::new(
                plotters::style::text_anchor::HPos::Right,
                plotters::style::text_anchor::VPos::Center,
            ),
        };

        let text_style = plotters::prelude::TextStyle::from(font)
            .color(&color)
            .pos(pos_adj);

        if style.angle != 0.0 {
            let transform = match style.angle as i32 {
                270 | -90 => plotters::style::text_anchor::Pos::new(
                    plotters::style::text_anchor::HPos::Center,
                    plotters::style::text_anchor::VPos::Center,
                ),
                _ => pos_adj,
            };

            let text_style = plotters::prelude::TextStyle::from(("sans-serif", style.size).into_font())
                .color(&color)
                .transform(FontTransform::Rotate270)
                .pos(transform);

            self.area
                .draw_text(text, &text_style, (pos.0 as i32, pos.1 as i32))
                .map_err(map_err)?;
        } else {
            self.area
                .draw_text(text, &text_style, (pos.0 as i32, pos.1 as i32))
                .map_err(map_err)?;
        }

        Ok(())
    }

    fn draw_polygon(
        &mut self,
        points: &[(f64, f64)],
        style: &RectStyle,
    ) -> Result<(), RenderError> {
        if points.len() < 3 {
            return Ok(());
        }
        let int_points: Vec<(i32, i32)> = points.iter().map(|(x, y)| (*x as i32, *y as i32)).collect();

        if let Some(fill) = style.fill {
            let fill_color = to_rgba(fill, style.alpha);
            self.area
                .draw(&Polygon::new(int_points.clone(), fill_color.filled()))
                .map_err(map_err)?;
        }

        Ok(())
    }

    fn draw_shape(
        &mut self,
        center: (f64, f64),
        radius: f64,
        style: &PointStyle,
    ) -> Result<(), RenderError> {
        let color = to_rgba(style.color, style.alpha);
        let (cx, cy) = (center.0 as i32, center.1 as i32);
        let r = radius as i32;

        match style.shape {
            PointShape::Circle => self.draw_circle(center, radius, style),
            PointShape::Square => {
                let tl = (cx - r, cy - r);
                let br = (cx + r, cy + r);
                if style.filled {
                    self.area
                        .draw(&plotters::prelude::Rectangle::new([tl, br], color.filled()))
                        .map_err(map_err)?;
                } else {
                    self.area
                        .draw(&plotters::prelude::Rectangle::new([tl, br], color.stroke_width(1)))
                        .map_err(map_err)?;
                }
                Ok(())
            }
            PointShape::Triangle => {
                let pts = vec![
                    (cx, cy - r),
                    (cx - r, cy + r),
                    (cx + r, cy + r),
                ];
                if style.filled {
                    self.area
                        .draw(&Polygon::new(pts, color.filled()))
                        .map_err(map_err)?;
                } else {
                    let outline = vec![
                        (cx, cy - r),
                        (cx - r, cy + r),
                        (cx + r, cy + r),
                        (cx, cy - r),
                    ];
                    self.area
                        .draw(&PathElement::new(outline, color.stroke_width(1)))
                        .map_err(map_err)?;
                }
                Ok(())
            }
            PointShape::Diamond => {
                let pts = vec![
                    (cx, cy - r),
                    (cx + r, cy),
                    (cx, cy + r),
                    (cx - r, cy),
                ];
                if style.filled {
                    self.area
                        .draw(&Polygon::new(pts, color.filled()))
                        .map_err(map_err)?;
                } else {
                    let outline = vec![
                        (cx, cy - r),
                        (cx + r, cy),
                        (cx, cy + r),
                        (cx - r, cy),
                        (cx, cy - r),
                    ];
                    self.area
                        .draw(&PathElement::new(outline, color.stroke_width(1)))
                        .map_err(map_err)?;
                }
                Ok(())
            }
            PointShape::Cross => {
                // X shape
                self.area
                    .draw(&PathElement::new(
                        vec![(cx - r, cy - r), (cx + r, cy + r)],
                        color.stroke_width(1),
                    ))
                    .map_err(map_err)?;
                self.area
                    .draw(&PathElement::new(
                        vec![(cx - r, cy + r), (cx + r, cy - r)],
                        color.stroke_width(1),
                    ))
                    .map_err(map_err)?;
                Ok(())
            }
            PointShape::Plus => {
                // + shape
                self.area
                    .draw(&PathElement::new(
                        vec![(cx - r, cy), (cx + r, cy)],
                        color.stroke_width(1),
                    ))
                    .map_err(map_err)?;
                self.area
                    .draw(&PathElement::new(
                        vec![(cx, cy - r), (cx, cy + r)],
                        color.stroke_width(1),
                    ))
                    .map_err(map_err)?;
                Ok(())
            }
        }
    }

    fn plot_area(&self) -> Rect {
        self.plot_area.clone()
    }

    fn total_area(&self) -> Rect {
        self.total_area.clone()
    }
}
