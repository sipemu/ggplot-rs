//! Hierarchical partition charts: `geom_sunburst` (under `coord_polar`) and
//! `geom_icicle` (Cartesian). One band per hierarchy level; each node's span
//! is proportional to its value and nested inside its parent's. Layout by
//! [`StatPartition`].

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, FontFace, RectStyle, TextAnchor, TextStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::partition::StatPartition;
use crate::stat::Stat;
use crate::theme::Theme;

use super::treemap::{contrast_text, fit_size};
use super::{Geom, GeomParams};

/// Partition chart over the `levels` columns (outermost first), sized by
/// `weight` (1 per row when unmapped). Drawn as sectors under a polar coord
/// (sunburst) and as rectangles otherwise (icicle).
pub struct GeomPartition {
    /// Hierarchy columns, outermost level first.
    pub levels: Vec<String>,
    /// Centre hole of a sunburst, in level widths (`geom_sunburst` turns it
    /// into the polar coord's inner radius; unused in Cartesian coords).
    pub hole: f64,
    /// Order siblings by descending value.
    pub sort: bool,
    /// Grow downward with the root on top (icicle).
    pub downward: bool,
    /// Fill when no fill scale applies.
    pub fill: (u8, u8, u8),
    /// Blend each level this much further toward white (0 = flat colours).
    pub lighten_per_level: f64,
    pub border: (u8, u8, u8),
    pub border_width: f64,
    pub alpha: f64,
    /// Label size (shrunk to fit, hidden below `min_label_size`; 0 = none).
    pub label_size: f64,
    pub min_label_size: f64,
}

impl GeomPartition {
    /// A sunburst over `levels` (pair with `coord_polar`; `geom_sunburst`
    /// does that for you).
    pub fn sunburst<S: Into<String>>(levels: impl IntoIterator<Item = S>) -> Self {
        GeomPartition {
            levels: levels.into_iter().map(Into::into).collect(),
            hole: 0.6,
            sort: true,
            downward: false,
            fill: (97, 156, 255),
            lighten_per_level: 0.18,
            border: (255, 255, 255),
            border_width: 1.0,
            alpha: 1.0,
            label_size: 10.0,
            min_label_size: 6.5,
        }
    }

    /// An icicle over `levels`: the root level on top, children below.
    pub fn icicle<S: Into<String>>(levels: impl IntoIterator<Item = S>) -> Self {
        GeomPartition {
            hole: 0.0,
            downward: true,
            ..Self::sunburst(levels)
        }
    }
}

fn lighten(c: (u8, u8, u8), t: f64) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    let m = |v: u8| (v as f64 + (255.0 - v as f64) * t).round() as u8;
    (m(c.0), m(c.1), m(c.2))
}

impl Geom for GeomPartition {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let col = |n: &str| {
            data.column(n)
                .ok_or_else(|| RenderError::MissingAesthetic(n.into()))
        };
        let (x0c, x1c, y0c, y1c) = (col("xmin")?, col("xmax")?, col("ymin")?, col("ymax")?);
        let (node, path, value) = (col("node")?, col("path")?, col("value")?);
        let (depth, share) = (data.column("depth"), data.column("share"));
        let fill_col = data.column("fill");
        let area = backend.plot_area();
        let nx = |v: &Value| scales.get(&Aesthetic::X).map(|s| s.map(v)).unwrap_or(0.0);
        let ny = |v: &Value| scales.get(&Aesthetic::Y).map(|s| s.map(v)).unwrap_or(0.0);
        let dist = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).hypot(a.1 - b.1);
        let family = super::resolve_font_family("", theme);

        let mut labels = Vec::new();
        for i in 0..data.nrows() {
            let (x0, x1, y0, y1) = (nx(&x0c[i]), nx(&x1c[i]), ny(&y0c[i]), ny(&y1c[i]));
            if ![x0, x1, y0, y1].iter().all(|v| v.is_finite()) || x1 <= x0 {
                continue;
            }
            let base = fill_col
                .and_then(|c| scales.map_color(&Aesthetic::Fill, &c[i]))
                .unwrap_or(self.fill);
            let d = depth.and_then(|c| c[i].as_f64()).unwrap_or(0.0);
            let fill = lighten(base, d * self.lighten_per_level);
            let pct = share
                .and_then(|c| c[i].as_f64())
                .map(|s| {
                    format!(
                        " ({}%)",
                        super::tip_value(&Value::Float((s * 1000.0).round() / 10.0))
                    )
                })
                .unwrap_or_default();
            super::set_mark(
                backend,
                Some(format!(
                    "{}: {}{pct}",
                    super::tip_value(&path[i]),
                    super::tip_value(&value[i])
                )),
                Some(super::tip_value(&path[i])),
                super::series_key(data, i),
                super::raw_value(&value[i]),
            );
            let style = RectStyle {
                fill: Some(fill),
                stroke: Some(self.border),
                stroke_width: self.border_width,
                alpha: self.alpha,
                clip: !coord.is_polar(),
                corner_radius: 0.0,
            };
            if coord.is_polar() {
                let pts = super::col::polar_sector(coord, &area, x0, x1, y0, y1);
                backend.draw_polygon(&pts, &style)?;
            } else {
                let (a, b) = (
                    coord.transform((x0, y1), &area),
                    coord.transform((x1, y0), &area),
                );
                backend.draw_rect(
                    (a.0.min(b.0), a.1.min(b.1)),
                    (a.0.max(b.0), a.1.max(b.1)),
                    &style,
                )?;
            }
            if self.label_size > 0.0 {
                let (xm, ym) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
                let centre = coord.transform((xm, ym), &area);
                // Room along the band (chord at mid-radius) and across it.
                let along = dist(
                    coord.transform((x0, ym), &area),
                    coord.transform((x1, ym), &area),
                );
                let across = dist(
                    coord.transform((xm, y0), &area),
                    coord.transform((xm, y1), &area),
                );
                let name = super::tip_value(&node[i]);
                if let Some(size) =
                    fit_size(&name, along, across, self.label_size, self.min_label_size)
                {
                    labels.push((name, centre, size, contrast_text(fill)));
                }
            }
        }
        super::clear_mark(backend);
        for (text, pos, size, color) in labels {
            backend.draw_text(
                &text,
                pos,
                &TextStyle {
                    color,
                    size,
                    anchor: TextAnchor::Middle,
                    angle: 0.0,
                    family: family.clone(),
                    face: FontFace::Plain,
                    bg_colour: None,
                    bg_r: 0.0,
                },
            )?;
        }
        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        Vec::new()
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatPartition {
            levels: self.levels.clone(),
            sort: self.sort,
            downward: self.downward,
        })
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "partition"
    }

    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let (true, Some(paper)) = (self.border == (255, 255, 255), el.paper) {
            self.border = paper;
        }
    }
}
