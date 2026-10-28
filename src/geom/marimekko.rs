//! Variable-width column charts: `geom_variwide` (bar width ∝ `weight`,
//! height = `y`) and `geom_marimekko` (columns ∝ their total, split into
//! 100 % stacked segments by `fill`). Layout by [`StatMarimekko`]; the
//! builders put the category names under the column centres.

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, FontFace, RectStyle, TextAnchor, TextStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::marimekko::StatMarimekko;
use crate::stat::Stat;
use crate::theme::Theme;

use super::treemap::{contrast_text, fit_size};
use super::{Geom, GeomParams};

/// Variwide / Marimekko chart. Map `x` (category), `weight` (column width)
/// and `y` (variwide height) or `fill` (Marimekko segments).
pub struct GeomMarimekko {
    /// Segments by `fill` (Marimekko) rather than one bar per column.
    pub mosaic: bool,
    /// Gap between columns as a fraction of the full width.
    pub gap: f64,
    /// Fill when `fill` is unmapped.
    pub fill: (u8, u8, u8),
    pub border: (u8, u8, u8),
    pub alpha: f64,
    /// Label Marimekko segments with their share of the column.
    pub labels: bool,
    pub label_size: f64,
}

impl GeomMarimekko {
    /// Variwide: bar widths ∝ `weight`, heights = `y`.
    pub fn variwide() -> Self {
        GeomMarimekko {
            mosaic: false,
            gap: 0.0,
            fill: (97, 156, 255),
            border: (255, 255, 255),
            alpha: 1.0,
            labels: false,
            label_size: 9.0,
        }
    }

    /// Marimekko: column widths ∝ their total `weight`, segments by `fill`.
    pub fn marimekko() -> Self {
        GeomMarimekko {
            mosaic: true,
            gap: 0.006,
            labels: true,
            ..Self::variwide()
        }
    }
}

impl Geom for GeomMarimekko {
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
        let (column, value, share) = (col("column")?, col("value")?, col("share")?);
        let (fill_col, y_col) = (data.column("fill"), data.column("y"));
        let area = backend.plot_area();
        let m = |aes: Aesthetic, v: &Value| scales.get(&aes).map(|s| s.map(v)).unwrap_or(0.0);
        let family = super::resolve_font_family("", theme);
        let pct = |v: &Value| {
            v.as_f64()
                .map(|s| {
                    format!(
                        "{}%",
                        super::tip_value(&Value::Float((s * 1000.0).round() / 10.0))
                    )
                })
                .unwrap_or_default()
        };
        let mut labels = Vec::new();
        for i in 0..data.nrows() {
            let a = coord.transform((m(Aesthetic::X, &x0c[i]), m(Aesthetic::Y, &y1c[i])), &area);
            let b = coord.transform((m(Aesthetic::X, &x1c[i]), m(Aesthetic::Y, &y0c[i])), &area);
            let ((l, t), (r, btm)) = ((a.0.min(b.0), a.1.min(b.1)), (a.0.max(b.0), a.1.max(b.1)));
            if ![l, t, r, btm].iter().all(|v| v.is_finite()) {
                continue;
            }
            let fill = fill_col
                .and_then(|c| scales.map_color(&Aesthetic::Fill, &c[i]))
                .unwrap_or(self.fill);
            let name = super::tip_value(&column[i]);
            let tip = if self.mosaic {
                let seg = fill_col
                    .map(|c| super::tip_value(&c[i]))
                    .unwrap_or_default();
                format!(
                    "{name} / {seg}: {} ({})",
                    super::tip_value(&value[i]),
                    pct(&share[i])
                )
            } else {
                format!(
                    "{name}: {} (width {})",
                    y_col.map(|c| super::tip_value(&c[i])).unwrap_or_default(),
                    super::tip_value(&value[i])
                )
            };
            super::set_mark(
                backend,
                Some(tip),
                Some(name),
                super::series_key(data, i),
                super::raw_value(&value[i]),
            );
            super::set_row_extras(backend, data, i);
            backend.draw_rect(
                (l, t),
                (r, btm),
                &RectStyle {
                    fill: Some(fill),
                    stroke: Some(self.border),
                    stroke_width: 0.8,
                    alpha: self.alpha,
                    clip: true,
                    corner_radius: 0.0,
                },
            )?;
            if self.labels {
                let text = pct(&share[i]);
                if let Some(size) = fit_size(&text, r - l, btm - t, self.label_size, 6.0) {
                    labels.push((
                        text,
                        ((l + r) / 2.0, (t + btm) / 2.0),
                        size,
                        contrast_text(fill),
                    ));
                }
            }
        }
        super::clear_mark(backend);
        super::clear_row_extras(backend);
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
        vec![Aesthetic::X]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatMarimekko {
            mosaic: self.mosaic,
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
        if self.mosaic {
            "marimekko"
        } else {
            "variwide"
        }
    }

    fn consumes_tooltips(&self) -> bool {
        true
    }

    fn include_zero_baseline(&self) -> bool {
        true
    }

    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let (true, Some(paper)) = (self.border == (255, 255, 255), el.paper) {
            self.border = paper;
        }
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }
}
