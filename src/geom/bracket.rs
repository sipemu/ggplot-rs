use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, FontFace, LineStyle, Linetype, TextAnchor, TextStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Significance bracket — a horizontal bar with two downward end ticks spanning
/// `[xmin, xmax]` at height `y`, captioned with a `label` above it (R's
/// `ggpubr::geom_bracket`). Typically annotates a pairwise-comparison p-value or
/// significance stars over a boxplot.
pub struct GeomBracket {
    pub color: (u8, u8, u8),
    pub line_width: f64,
    /// Length of the downward end ticks, in pixels.
    pub tip_length: f64,
    /// Label font size.
    pub label_size: f64,
}

impl Default for GeomBracket {
    fn default() -> Self {
        GeomBracket {
            color: (0, 0, 0),
            line_width: 1.0,
            tip_length: 8.0,
            label_size: 12.0,
        }
    }
}

impl Geom for GeomBracket {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let xmin_col = data
            .column("xmin")
            .ok_or(RenderError::MissingAesthetic("xmin".into()))?;
        let xmax_col = data
            .column("xmax")
            .ok_or(RenderError::MissingAesthetic("xmax".into()))?;
        let y_col = data
            .column("y")
            .ok_or(RenderError::MissingAesthetic("y".into()))?;
        let label_col = data.column("label");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);

        for i in 0..data.nrows() {
            let nxmin = x_scale.map(|s| s.map(&xmin_col[i])).unwrap_or(0.0);
            let nxmax = x_scale.map(|s| s.map(&xmax_col[i])).unwrap_or(0.0);
            let ny = y_scale.map(|s| s.map(&y_col[i])).unwrap_or(0.0);

            let (px_min, py) = coord.transform((nxmin, ny), &plot_area);
            let (px_max, _) = coord.transform((nxmax, ny), &plot_area);

            // Bar at `py` with end ticks pointing toward the data (+y is down in
            // screen space, so the ticks drop below the bar).
            let tip = self.tip_length;
            backend.draw_line(
                &[
                    (px_min, py + tip),
                    (px_min, py),
                    (px_max, py),
                    (px_max, py + tip),
                ],
                &LineStyle {
                    color: self.color,
                    alpha: 1.0,
                    width: self.line_width,
                    linetype: Linetype::Solid,
                },
            )?;

            // Centered label just above the bar.
            if let Some(lc) = label_col {
                let text = lc[i].to_group_key();
                if !text.is_empty() {
                    let cx = (px_min + px_max) / 2.0;
                    backend.draw_text(
                        &text,
                        (cx, py - self.label_size * 0.3 - 2.0),
                        &TextStyle {
                            color: self.color,
                            size: self.label_size,
                            anchor: TextAnchor::Middle,
                            angle: 0.0,
                            family: None,
                            face: FontFace::Plain,
                        },
                    )?;
                }
            }
        }

        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::Xmin, Aesthetic::Xmax, Aesthetic::Y]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatIdentity)
    }
    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }
    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }
    fn name(&self) -> &str {
        "bracket"
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.color = color;
    }
}
