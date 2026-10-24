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
        // #197: a stacked segment's bottom (`position_stack`/`position_fill`
        // write `ymin`); an overlaid/dodged/plain histogram has none, so
        // every bar still starts at 0, as before.
        let ymin_col = data.column("ymin");
        // #197: `aes(fill = group)` — resolved per row through the fill
        // scale, the same `fill_col`/`map_color` idiom `GeomBar` uses; the
        // outline stays this geom's own single colour, as `GeomBar`'s does.
        let fill_col = data.column("fill");

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

            let fill = fill_col
                .and_then(|c| scales.map_color(&Aesthetic::Fill, &c[i]))
                .unwrap_or(self.fill);

            let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
            let ny_base = ymin_col
                .and_then(|c| c[i].as_f64())
                .and_then(|v| y_scale.map(|s| s.map(&crate::data::Value::Float(v))))
                .unwrap_or_else(|| {
                    y_scale
                        .map(|s| s.map(&crate::data::Value::Float(0.0)))
                        .unwrap_or(0.0)
                });

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
                    fill: Some(fill),
                    stroke: Some(self.color),
                    stroke_width: 0.5,
                    alpha: self.alpha,
                    clip: true,
                    corner_radius: 0.0,
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

    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let (true, Some(ink)) = (self.color == (50, 50, 50), el.ink) {
            self.color = ink;
        }
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

    /// Every `data-geom="histogram"` rect's `fill="#rrggbb"`.
    fn histogram_fills(svg: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut rest = svg;
        while let Some(i) = rest.find("<rect ") {
            let end = i + rest[i..].find('>').unwrap();
            let tag = &rest[i..end];
            if tag.contains("data-geom=\"histogram\"") {
                let k = tag.find(" fill=\"").unwrap() + 7;
                out.push(tag[k..k + tag[k..].find('"').unwrap()].to_string());
            }
            rest = &rest[end + 1..];
        }
        out
    }

    fn grouped_data() -> Vec<(String, Vec<Value>)> {
        let n = 40;
        let v: Vec<Value> = (0..n).map(|i| Value::Float((i % 10) as f64)).collect();
        let g: Vec<Value> = (0..n)
            .map(|i| {
                Value::Str(if i < n / 2 {
                    "ref".into()
                } else {
                    "cur".into()
                })
            })
            .collect();
        vec![("v".into(), v), ("g".into(), g)]
    }

    /// #197: `aes(fill = group)` draws each group's bars in a distinct
    /// colour (overlaid, `position_identity` — the default) with a legend,
    /// instead of one combined series in a single literal colour.
    #[test]
    fn grouped_fill_draws_distinct_colours_with_a_legend() {
        let svg = GGPlot::new(grouped_data())
            .aes(Aes::new().x("v").fill("g"))
            .geom_histogram_with(GeomHistogram {
                alpha: 0.6,
                ..Default::default()
            })
            .render_svg_native_with_size(500, 350)
            .unwrap();
        let fills: std::collections::BTreeSet<String> = histogram_fills(&svg).into_iter().collect();
        assert!(fills.len() >= 2, "expected ≥2 fill colours: {fills:?}");
        assert!(
            svg.contains(">ref<") && svg.contains(">cur<"),
            "legend: {svg}"
        );
    }

    /// #197: bins are computed once over the pooled x range, so every
    /// group's bars share the same edges (needed for overlay/stack/dodge to
    /// line up) — not each group its own range.
    #[test]
    fn grouped_fill_shares_bin_edges() {
        let svg = GGPlot::new(grouped_data())
            .aes(Aes::new().x("v").fill("g"))
            .geom_histogram()
            .render_svg_native_with_size(500, 350)
            .unwrap();
        let mut widths = std::collections::BTreeSet::new();
        let mut rest = svg.as_str();
        while let Some(i) = rest.find("<rect ") {
            let end = i + rest[i..].find('>').unwrap();
            let tag = &rest[i..end];
            if tag.contains("data-geom=\"histogram\"") {
                let k = tag.find(" width=\"").unwrap() + 8;
                let w: f64 = tag[k..k + tag[k..].find('"').unwrap()].parse().unwrap();
                widths.insert((w * 100.0).round() as i64);
            }
            rest = &rest[end + 1..];
        }
        assert_eq!(widths.len(), 1, "every bar the same width: {widths:?}");
    }

    /// #197: `position_stack` — a stacked histogram's upper group starts
    /// where the lower one's bar ends (reads the new `ymin` column), not at
    /// 0 for every group.
    #[test]
    fn grouped_fill_can_be_stacked() {
        let svg = GGPlot::new(grouped_data())
            .aes(Aes::new().x("v").fill("g"))
            .geom_histogram()
            .position(crate::position::stack::PositionStack)
            .render_svg_native_with_size(500, 350)
            .unwrap();
        // Two bars at the same x: their y-ranges (top_px..bottom_px) must
        // not both start at the panel's zero baseline.
        let mut tops = Vec::new();
        let mut rest = svg.as_str();
        while let Some(i) = rest.find("<rect ") {
            let end = i + rest[i..].find('>').unwrap();
            let tag = &rest[i..end];
            if tag.contains("data-geom=\"histogram\"") {
                let k = tag.find(" y=\"").unwrap() + 4;
                let y: f64 = tag[k..k + tag[k..].find('"').unwrap()].parse().unwrap();
                tops.push(y);
            }
            rest = &rest[end + 1..];
        }
        let distinct: std::collections::BTreeSet<i64> =
            tops.iter().map(|t| (t * 10.0).round() as i64).collect();
        assert!(
            distinct.len() > 1,
            "stacked bars should start at different heights: {tops:?}"
        );
    }

    /// #197: `position_dodge2` — ggplot2's own answer for dodging a
    /// variable-width stat (bins can differ in width); groups at the same
    /// bin sit side by side instead of overlapping.
    #[test]
    fn grouped_fill_can_be_dodged() {
        // `position_dodge2`'s width must match the bars' own width to not
        // spill into the next bin — ggplot2's own convention (a caller
        // writes `position_dodge2(width = <bar width>)`); an explicit
        // `binwidth` here makes that width known.
        let svg = GGPlot::new(grouped_data())
            .aes(Aes::new().x("v").fill("g"))
            .geom_histogram_with(GeomHistogram {
                binwidth: Some(1.0),
                ..Default::default()
            })
            .position(crate::position::dodge2::PositionDodge2::default().with_width(1.0))
            .render_svg_native_with_size(500, 350)
            .unwrap();
        let mut spans: Vec<(f64, f64)> = Vec::new();
        let mut rest = svg.as_str();
        while let Some(i) = rest.find("<rect ") {
            let end = i + rest[i..].find('>').unwrap();
            let tag = &rest[i..end];
            if tag.contains("data-geom=\"histogram\"") {
                let attr = |a: &str| -> f64 {
                    let k = tag.find(&format!(" {a}=\"")).unwrap() + a.len() + 3;
                    tag[k..k + tag[k..].find('"').unwrap()].parse().unwrap()
                };
                let x = attr("x");
                spans.push((x, x + attr("width")));
            }
            rest = &rest[end + 1..];
        }
        // No two bars' x-spans should overlap once dodged.
        for i in 0..spans.len() {
            for j in (i + 1)..spans.len() {
                let (a, b) = (spans[i], spans[j]);
                let overlap = a.0 < b.1 - 0.01 && b.0 < a.1 - 0.01;
                assert!(!overlap, "bars {a:?} and {b:?} overlap after dodge2");
            }
        }
    }
}
