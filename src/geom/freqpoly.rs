use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineStyle, Linetype};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::bin::StatBin;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Frequency polygon — line through bin centers (StatBin output).
pub struct GeomFreqpoly {
    pub color: (u8, u8, u8),
    pub width: f64,
    pub alpha: f64,
}

impl Default for GeomFreqpoly {
    fn default() -> Self {
        GeomFreqpoly {
            color: (0, 0, 0),
            width: 1.5,
            alpha: 1.0,
        }
    }
}

impl Geom for GeomFreqpoly {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let x_col = data
            .column("x")
            .ok_or(RenderError::MissingAesthetic("x".into()))?;
        let y_col = data
            .column("y")
            .ok_or(RenderError::MissingAesthetic("y".into()))?;
        let color_col = data.column("color");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        // #197: `StatBin` is `panelwise` (bins shared across every
        // `aes(color=)` group) and emits every group's bins in one call, so
        // draw one line per group — the same `row_groups` idiom
        // `GeomQQLine` uses — instead of connecting every group's points
        // into one zigzag with a single colour.
        for rows in super::support::row_groups(data) {
            let points: Vec<(f64, f64)> = rows
                .iter()
                .map(|&i| {
                    let nx = x_scale.map(|s| s.map(&x_col[i])).unwrap_or(0.0);
                    let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);
                    coord.transform((nx, ny), &plot_area)
                })
                .collect();
            let first = rows[0];
            let line_color = color_col
                .and_then(|cc| scales.map_color(&Aesthetic::Color, &cc[first]))
                .unwrap_or(self.color);
            if points.len() >= 2 {
                backend.draw_line(
                    &points,
                    &LineStyle {
                        color: line_color,
                        alpha: self.alpha,
                        width: self.width,
                        linetype: Linetype::Solid,
                        ..Default::default()
                    },
                )?;
            }
        }

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatBin::default())
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "freqpoly"
    }

    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let (true, Some(ink)) = (self.color == (0, 0, 0), el.ink) {
            self.color = ink;
        }
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.color = color;
    }
}

#[cfg(test)]
mod tests {
    use crate::data::Value;
    use crate::prelude::*;

    /// #197: `StatBin` became `panelwise` (bins shared across groups) and
    /// now hands every group's rows to the geom in one call — this must
    /// still draw one connected line per group, each in its own colour, not
    /// one zigzag line through every group's points in a single colour.
    #[test]
    fn grouped_freqpoly_draws_one_line_per_group() {
        let n = 30;
        let v: Vec<Value> = (0..n).map(|i| Value::Float((i % 10) as f64)).collect();
        let g: Vec<Value> = (0..n)
            .map(|i| Value::Str(if i < n / 2 { "a".into() } else { "b".into() }))
            .collect();
        let svg = GGPlot::new(vec![("v".into(), v), ("g".into(), g)])
            .aes(Aes::new().x("v").color("g"))
            .geom_freqpoly()
            .render_svg_native_with_size(500, 350)
            .unwrap();
        // Two `data-geom="freqpoly"` lines (one per group, not counting
        // grid lines/ticks), not a single big one connecting both groups'
        // bins.
        let mut colors = std::collections::BTreeSet::new();
        let mut rest = svg.as_str();
        let mut freqpoly_lines = 0;
        while let Some(i) = rest.find("<polyline ") {
            let end = i + rest[i..].find('>').unwrap();
            let tag = &rest[i..end];
            if tag.contains("data-geom=\"freqpoly\"") {
                freqpoly_lines += 1;
                let k = tag.find(" stroke=\"").unwrap() + 9;
                colors.insert(tag[k..k + tag[k..].find('"').unwrap()].to_string());
            }
            rest = &rest[end + 1..];
        }
        assert_eq!(freqpoly_lines, 2, "{svg}");
        assert_eq!(colors.len(), 2, "{colors:?}");
    }
}
