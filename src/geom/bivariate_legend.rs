//! `bivariate_legend()` (feature `sf`): the square `n × m` guide for
//! [`ScaleFillBivariate`](crate::scale::bivariate::ScaleFillBivariate), with
//! two axis captions — biscale's own legend shape. Drawn as an ordinary
//! furniture geom (no required aesthetic, no data column), the same way
//! [`crate::geom::annotation_scale`]'s scale bar/attribution are, rather
//! than through the standard per-aesthetic discrete-legend machinery (which
//! draws one row per break, not a 2-D grid) — set
//! `.legend_position(LegendPosition::None)` to hide the ordinary Fill
//! legend `ScaleFillBivariate` would otherwise also produce.

use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, RectStyle, TextAnchor, TextStyle};
use crate::render::RenderError;
use crate::scale::bivariate::bivariate_cell_color;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Which panel corner the legend sits in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LegendCorner {
    BottomLeft,
    #[default]
    BottomRight,
    TopLeft,
    TopRight,
}

/// The square bivariate legend: an `nx × ny` grid of swatches plus two axis
/// captions. Mirror a [`ScaleFillBivariate`](crate::scale::bivariate::ScaleFillBivariate)'s
/// `nx`/`ny`/`base`/`color_x`/`color_y` here so the swatches match the data
/// exactly (there's no live link between the scale and this geom — they're
/// independent layers, like `annotation_scale` and `geom_sf`).
pub struct GeomBivariateLegend {
    pub nx: usize,
    pub ny: usize,
    pub base: (u8, u8, u8),
    pub color_x: (u8, u8, u8),
    pub color_y: (u8, u8, u8),
    pub x_name: String,
    pub y_name: String,
    pub corner: LegendCorner,
    pub cell_size: f64,
    pub text_size: f64,
    pub text_color: (u8, u8, u8),
}

impl GeomBivariateLegend {
    /// Build directly from a [`ScaleFillBivariate`](crate::scale::bivariate::ScaleFillBivariate),
    /// so the legend's swatches are guaranteed to match the scale's.
    pub fn from_scale(scale: &crate::scale::bivariate::ScaleFillBivariate) -> Self {
        GeomBivariateLegend {
            nx: scale.nx(),
            ny: scale.ny(),
            base: scale.base(),
            color_x: scale.color_x(),
            color_y: scale.color_y(),
            x_name: scale.x_name().to_string(),
            y_name: scale.y_name().to_string(),
            ..Self::default()
        }
    }

    pub fn corner(mut self, corner: LegendCorner) -> Self {
        self.corner = corner;
        self
    }

    pub fn cell_size(mut self, px: f64) -> Self {
        self.cell_size = px.max(1.0);
        self
    }
}

impl Default for GeomBivariateLegend {
    fn default() -> Self {
        GeomBivariateLegend {
            nx: 3,
            ny: 3,
            base: (255, 255, 255),
            color_x: (220, 90, 60),
            color_y: (60, 90, 220),
            x_name: String::new(),
            y_name: String::new(),
            corner: LegendCorner::default(),
            cell_size: 14.0,
            text_size: 8.0,
            text_color: (80, 80, 80),
        }
    }
}

impl Geom for GeomBivariateLegend {
    fn draw(
        &self,
        _data: &DataFrame,
        _coord: &dyn Coord,
        _scales: &ScaleSet,
        theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let plot_area = backend.plot_area();
        let (nx, ny) = (self.nx.max(1), self.ny.max(1));
        let grid_w = nx as f64 * self.cell_size;
        let grid_h = ny as f64 * self.cell_size;
        // Room for the y caption (rotated, to the left) and the x caption
        // (below), plus a little padding all round.
        let y_caption_w = if self.y_name.is_empty() {
            0.0
        } else {
            self.text_size + 4.0
        };
        let x_caption_h = if self.x_name.is_empty() {
            0.0
        } else {
            self.text_size + 4.0
        };
        let pad = 6.0;
        let total_w = y_caption_w + grid_w + pad * 2.0;
        let total_h = grid_h + x_caption_h + pad * 2.0;

        let (box_x, box_y) = match self.corner {
            LegendCorner::BottomLeft => (
                plot_area.x + pad,
                plot_area.y + plot_area.height - total_h - pad,
            ),
            LegendCorner::BottomRight => (
                plot_area.x + plot_area.width - total_w - pad,
                plot_area.y + plot_area.height - total_h - pad,
            ),
            LegendCorner::TopLeft => (plot_area.x + pad, plot_area.y + pad),
            LegendCorner::TopRight => (
                plot_area.x + plot_area.width - total_w - pad,
                plot_area.y + pad,
            ),
        };

        if theme.legend_background.visible {
            if let Some(fill) = theme.legend_background.fill {
                backend.draw_rect(
                    (box_x, box_y),
                    (box_x + total_w, box_y + total_h),
                    &RectStyle {
                        fill: Some(fill),
                        stroke: theme.legend_background.color,
                        stroke_width: theme.legend_background.width,
                        alpha: 1.0,
                        clip: false,
                        corner_radius: 0.0,
                    },
                )?;
            }
        }

        let grid_x = box_x + y_caption_w;
        let grid_y = box_y;
        // Row 0 (j = 0, the low-y end) is drawn at the BOTTOM, matching
        // biscale's own legend orientation (y increases upward).
        for j in 0..ny {
            for i in 0..nx {
                let color =
                    bivariate_cell_color(self.base, self.color_x, self.color_y, nx, ny, i, j);
                let cx = grid_x + i as f64 * self.cell_size;
                let cy = grid_y + (ny - 1 - j) as f64 * self.cell_size;
                backend.draw_rect(
                    (cx, cy),
                    (cx + self.cell_size, cy + self.cell_size),
                    &RectStyle {
                        fill: Some(color),
                        stroke: None,
                        stroke_width: 0.0,
                        alpha: 1.0,
                        clip: false,
                        corner_radius: 0.0,
                    },
                )?;
            }
        }

        let font = super::resolve_font_family("", theme);
        if !self.x_name.is_empty() {
            backend.draw_text(
                &self.x_name,
                (grid_x + grid_w / 2.0, grid_y + grid_h + x_caption_h / 2.0),
                &TextStyle {
                    color: self.text_color,
                    size: self.text_size,
                    anchor: TextAnchor::Middle,
                    angle: 0.0,
                    family: font.clone(),
                    face: crate::render::backend::FontFace::Plain,
                },
            )?;
        }
        if !self.y_name.is_empty() {
            backend.draw_text(
                &self.y_name,
                (box_x + y_caption_w / 2.0, grid_y + grid_h / 2.0),
                &TextStyle {
                    color: self.text_color,
                    size: self.text_size,
                    anchor: TextAnchor::Middle,
                    angle: 90.0,
                    family: font,
                    face: crate::render::backend::FontFace::Plain,
                },
            )?;
        }
        Ok(())
    }

    fn required_aes(&self) -> Vec<crate::aes::Aesthetic> {
        vec![]
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
        "bivariate_legend"
    }
    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let (true, Some(ink)) = (self.text_color == (80, 80, 80), el.ink) {
            self.text_color = ink;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aes::Aesthetic;
    use crate::scale::bivariate::ScaleFillBivariate;

    #[test]
    fn from_scale_copies_grid_shape_and_colors() {
        let s = ScaleFillBivariate::tertile(
            Aesthetic::Fill,
            (217, 95, 2),
            (117, 112, 179),
            "Income",
            "Education",
        );
        let g = GeomBivariateLegend::from_scale(&s);
        assert_eq!(g.nx, 3);
        assert_eq!(g.ny, 3);
        assert_eq!(g.color_x, (217, 95, 2));
        assert_eq!(g.color_y, (117, 112, 179));
        assert_eq!(g.x_name, "Income");
        assert_eq!(g.y_name, "Education");
    }

    #[test]
    fn default_corner_is_bottom_right() {
        assert_eq!(LegendCorner::default(), LegendCorner::BottomRight);
    }
}
