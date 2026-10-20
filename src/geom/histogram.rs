use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::bin::StatBin;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Histogram geometry (bar chart with StatBin).
pub struct GeomHistogram {
    pub bins: usize,
    pub binwidth: Option<f64>,
    pub fill: (u8, u8, u8),
    pub color: (u8, u8, u8),
    pub alpha: f64,
}

impl GeomHistogram {
    /// Set bin width (overrides bins count).
    pub fn with_binwidth(mut self, width: f64) -> Self {
        self.binwidth = Some(width);
        self
    }

    /// Set number of bins.
    pub fn with_bins(mut self, bins: usize) -> Self {
        self.bins = bins;
        self.binwidth = None;
        self
    }
}

impl Default for GeomHistogram {
    fn default() -> Self {
        GeomHistogram {
            bins: 30,
            binwidth: None,
            fill: (97, 156, 255),
            color: (50, 50, 50),
            alpha: 1.0,
        }
    }
}

impl Geom for GeomHistogram {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let xmin_col = data.column("xmin");
        let xmax_col = data.column("xmax");
        let y_col = data
            .column("y")
            .ok_or(RenderError::MissingAesthetic("y".into()))?;

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        for i in 0..data.nrows() {
            let (left_n, right_n) = match (xmin_col, xmax_col) {
                (Some(xmin), Some(xmax)) => {
                    let ln = x_scale.map(|s| s.map(&xmin[i])).unwrap_or(0.0);
                    let rn = x_scale.map(|s| s.map(&xmax[i])).unwrap_or(1.0);
                    (ln, rn)
                }
                _ => {
                    // Fall back to x column with small width
                    let x_col = data
                        .column("x")
                        .ok_or(RenderError::MissingAesthetic("x".into()))?;
                    let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
                    (nx - 0.02, nx + 0.02)
                }
            };

            let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
            let ny_base = y_scale
                .map(|s| s.map(&crate::data::Value::Float(0.0)))
                .unwrap_or(0.0);

            let (left_px, top_px) = coord.transform((left_n, ny), &plot_area);
            let (right_px, bottom_px) = coord.transform((right_n, ny_base), &plot_area);

            // Hover tooltip: the bin range and its count.
            let cnt = super::tip_value(&y_col[i]);
            let tip = match (xmin_col, xmax_col) {
                (Some(lo), Some(hi)) => format!(
                    "[{}, {}): {cnt}",
                    super::tip_value(&lo[i]),
                    super::tip_value(&hi[i])
                ),
                _ => format!("count: {cnt}"),
            };
            super::set_mark(
                backend,
                Some(tip),
                data.column("x").map(|c| super::tip_value(&c[i])),
                super::series_key(data, i),
                super::measured_value(data, i),
            );
            super::set_mark_formatted(
                backend,
                scales,
                data.column("x").map(|c| &c[i]),
                &Aesthetic::Y,
                super::measured_raw(data, i),
            );
            super::set_row_extras(backend, data, i);

            backend.draw_rect(
                (left_px, top_px.min(bottom_px)),
                (right_px, top_px.max(bottom_px)),
                &RectStyle {
                    fill: Some(self.fill),
                    stroke: Some(self.color),
                    stroke_width: 0.5,
                    alpha: self.alpha,
                    clip: true,
                },
            )?;
        }
        super::clear_mark(backend);
        super::clear_row_extras(backend);

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        let mut stat = StatBin {
            bins: self.bins,
            binwidth: self.binwidth,
        };
        if let Some(bw) = self.binwidth {
            stat = stat.with_binwidth(bw);
        }
        Box::new(stat)
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "histogram"
    }

    fn consumes_tooltips(&self) -> bool {
        true
    }

    fn include_zero_baseline(&self) -> bool {
        true
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }
}

#[cfg(test)]
mod tests {
    use crate::aes::{AesMapping, Aesthetic, MappingStage};
    use crate::data::Value;
    use crate::prelude::*;

    /// Review R-7 (nit): an after_stat `data_attr` on a histogram — there's
    /// no builder for it (`Aes::data_attr` only maps BeforeStat), so this
    /// manually constructs the mapping, the same way a caller reaching for
    /// R's `stage()`-equivalent would have to. It must actually work (the
    /// underlying gap was `GeomHistogram` never calling
    /// `geom::set_row_extras` at all, not anything specific to after_stat).
    #[test]
    fn after_stat_data_attr_works_on_a_histogram() {
        let n = 50;
        let x: Vec<Value> = (0..n).map(|i| Value::Float(i as f64 * 0.2)).collect();
        let mut aes = Aes::new().x("x");
        aes.mappings.push(AesMapping {
            column: "count".to_string(),
            aesthetic: Aesthetic::DataAttr("data-n".to_string()),
            stage: MappingStage::AfterStat,
        });

        let svg = GGPlot::new(vec![("x".into(), x)])
            .aes(aes)
            .geom_histogram()
            .render_svg_native_with_size(400, 300)
            .unwrap();

        assert!(svg.contains("data-n=\""));
    }
}
