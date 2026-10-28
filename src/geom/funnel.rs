//! Funnel and pyramid charts: `geom_funnel` / `geom_pyramid`. Stages are
//! stacked top to bottom as centred polygons; labels show each stage's name,
//! value and share. Layout by [`StatFunnel`].

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, FontFace, RectStyle, TextAnchor, TextStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::funnel::{FunnelShape, StatFunnel};
use crate::stat::Stat;
use crate::theme::Theme;

use super::treemap::contrast_text;
use super::{Geom, GeomParams};

/// What a stage label shows after the stage name and value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FunnelPercent {
    /// No percentage.
    None,
    /// Share of the first stage (conversion from the top).
    OfFirst,
    /// Share of the total.
    OfTotal,
}

/// Funnel / pyramid chart. Map `x` (stage, in data order) and `y` (value).
pub struct GeomFunnel {
    pub shape: FunnelShape,
    /// Vertical gap between stages as a fraction of the chart height.
    pub gap: f64,
    /// Fill when no fill scale applies.
    pub fill: (u8, u8, u8),
    pub alpha: f64,
    /// Draw "name: value (pct)" labels inside the stages.
    pub labels: bool,
    pub percent: FunnelPercent,
    pub label_size: f64,
    /// Colour of labels drawn outside a stage too narrow to hold them.
    pub label_color: (u8, u8, u8),
}

impl Default for GeomFunnel {
    fn default() -> Self {
        GeomFunnel {
            shape: FunnelShape::Proportional,
            gap: 0.012,
            fill: (97, 156, 255),
            alpha: 1.0,
            labels: true,
            percent: FunnelPercent::OfFirst,
            label_size: 10.0,
            label_color: (40, 40, 40),
        }
    }
}

impl GeomFunnel {
    /// Highcharts-style funnel: fixed silhouette narrowing to a neck,
    /// stage heights proportional to the values.
    pub fn silhouette(neck_width: f64, neck_height: f64) -> Self {
        GeomFunnel {
            shape: FunnelShape::Silhouette {
                neck_width,
                neck_height,
            },
            percent: FunnelPercent::OfTotal,
            ..Default::default()
        }
    }

    /// Pyramid: apex on top, stage heights proportional to the values.
    pub fn pyramid() -> Self {
        GeomFunnel {
            shape: FunnelShape::Pyramid,
            percent: FunnelPercent::OfTotal,
            ..Default::default()
        }
    }
}

impl Geom for GeomFunnel {
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
        let (xc, yc, stage, value) = (col("fx")?, col("fy")?, col("stage")?, col("value")?);
        let (ly, pf, pt) = (
            col("label_y")?,
            data.column("pct_first"),
            data.column("pct_total"),
        );
        let fill_col = data.column("fill");
        let area = backend.plot_area();
        // The layout is already normalised (fx ∈ [-0.5, 0.5], fy ∈ [0, 1]):
        // place it in the panel directly, with a small margin.
        let map = |x: f64, y: f64| coord.transform((0.5 + 0.96 * x, 0.02 + 0.96 * y), &area);
        let family = super::resolve_font_family("", theme);

        // Rows come grouped by stage, in drawing order.
        let mut i = 0;
        let mut labels = Vec::new();
        while i < data.nrows() {
            let key = stage[i].key_str();
            let mut j = i;
            while j < data.nrows() && stage[j].key_str() == key {
                j += 1;
            }
            let pts: Vec<(f64, f64)> = (i..j)
                .filter_map(|k| Some(map(xc[k].as_f64()?, yc[k].as_f64()?)))
                .collect();
            let fill = fill_col
                .and_then(|c| scales.map_color(&Aesthetic::Fill, &c[i]))
                .unwrap_or(self.fill);
            let pct = match self.percent {
                FunnelPercent::None => None,
                FunnelPercent::OfFirst => pf.and_then(|c| c[i].as_f64()),
                FunnelPercent::OfTotal => pt.and_then(|c| c[i].as_f64()),
            }
            .map(|p| {
                format!(
                    " ({}%)",
                    super::tip_value(&Value::Float((p * 1000.0).round() / 10.0))
                )
            })
            .unwrap_or_default();
            let name = super::tip_value(&stage[i]);
            let text = format!("{name}: {}{pct}", super::tip_value(&value[i]));
            super::set_mark(
                backend,
                Some(text.clone()),
                Some(name),
                super::series_key(data, i),
                super::raw_value(&value[i]),
            );
            if pts.len() >= 3 {
                backend.draw_polygon(
                    &pts,
                    &RectStyle {
                        fill: Some(fill),
                        stroke: None,
                        stroke_width: 0.0,
                        alpha: self.alpha,
                        clip: true,
                        corner_radius: 0.0,
                    },
                )?;
            }
            if self.labels {
                if let Some(y) = ly[i].as_f64() {
                    // Inside when it fits the stage's mean width, else to the
                    // right of the shape in ink.
                    let half = (i..j)
                        .filter_map(|k| xc[k].as_f64())
                        .map(f64::abs)
                        .sum::<f64>()
                        / (j - i).max(1) as f64;
                    let (l, r) = (map(-half, y), map(half, y));
                    let need = text.chars().count() as f64 * self.label_size * 0.6 + 8.0;
                    if (r.0 - l.0).abs() >= need {
                        labels.push((text, map(0.0, y), contrast_text(fill), TextAnchor::Middle));
                    } else {
                        let edge = (i..j).filter_map(|k| xc[k].as_f64()).fold(0.0, f64::max);
                        let p = map(edge, y);
                        labels.push((text, (p.0 + 6.0, p.1), self.label_color, TextAnchor::Start));
                    }
                }
            }
            i = j;
        }
        super::clear_mark(backend);
        for (text, pos, color, anchor) in labels {
            backend.draw_text(
                &text,
                pos,
                &TextStyle {
                    color,
                    size: self.label_size,
                    anchor,
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
        vec![Aesthetic::X, Aesthetic::Y]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatFunnel {
            shape: self.shape,
            gap: self.gap,
        })
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "funnel"
    }

    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let Some(ink) = el.ink {
            self.label_color = ink;
        }
    }
}
