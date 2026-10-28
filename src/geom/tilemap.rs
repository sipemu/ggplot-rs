//! Tile-grid cartograms: `geom_tilemap`. Each row is one equal-size tile
//! (square, hexagon, circle or diamond) at grid position `x` (column) /
//! `y` (row, 0 = top), filled by `fill` and labelled by `label` — e.g. US
//! states or EU countries as a hex map. The grid is fitted into the panel
//! with one uniform scale, so tiles stay regular at any aspect ratio.

use std::f64::consts::PI;

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::DataFrame;
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, FontFace, RectStyle, TextAnchor, TextStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::treemap::{contrast_text, fit_size};
use super::{Geom, GeomParams};

/// Tile shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TileShape {
    #[default]
    Square,
    /// Pointy-top hexagons; odd rows shift half a tile right.
    Hexagon,
    Circle,
    Diamond,
}

/// Tile map. Map `x` (grid column), `y` (grid row, 0 at the top), `fill`
/// and `label`.
pub struct GeomTilemap {
    pub shape: TileShape,
    /// Gap between tiles as a fraction of the tile size.
    pub gap: f64,
    /// Fill when `fill` is unmapped (or a value is missing).
    pub fill: (u8, u8, u8),
    pub label_size: f64,
}

impl Default for GeomTilemap {
    fn default() -> Self {
        GeomTilemap {
            shape: TileShape::Square,
            gap: 0.08,
            fill: (200, 200, 200),
            label_size: 10.0,
        }
    }
}

/// Grid centre of a tile in tile units (x right, y down).
fn centre(shape: TileShape, col: f64, row: f64) -> (f64, f64) {
    match shape {
        TileShape::Hexagon => {
            let odd = (row.round() as i64).rem_euclid(2) == 1;
            (col + if odd { 0.5 } else { 0.0 }, row * 3f64.sqrt() / 2.0)
        }
        _ => (col, row),
    }
}

impl Geom for GeomTilemap {
    fn draw(
        &self,
        data: &DataFrame,
        _coord: &dyn Coord,
        scales: &ScaleSet,
        theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let col = |n: &str| {
            data.column(n)
                .ok_or_else(|| RenderError::MissingAesthetic(n.into()))
        };
        let (xc, yc) = (col("x")?, col("y")?);
        let (fill_col, label_col) = (data.column("fill"), data.column("label"));
        let tiles: Vec<(usize, (f64, f64))> = (0..data.nrows())
            .filter_map(|i| Some((i, centre(self.shape, xc[i].as_f64()?, yc[i].as_f64()?))))
            .collect();
        if tiles.is_empty() {
            return Ok(());
        }
        let area = backend.plot_area();
        let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for (_, (x, y)) in &tiles {
            x0 = x0.min(*x);
            x1 = x1.max(*x);
            y0 = y0.min(*y);
            y1 = y1.max(*y);
        }
        // One tile of margin around the grid; uniform px per tile unit.
        let (gw, gh) = (x1 - x0 + 1.0, y1 - y0 + 1.0);
        let unit = (area.width / gw).min(area.height / gh);
        let ox = area.x + (area.width - unit * gw) / 2.0 + unit / 2.0 - x0 * unit;
        let oy = area.y + (area.height - unit * gh) / 2.0 + unit / 2.0 - y0 * unit;
        let r = unit * (1.0 - self.gap) / 2.0;
        let family = super::resolve_font_family("", theme);
        let mut labels = Vec::new();
        for (i, (x, y)) in tiles {
            let c = (ox + x * unit, oy + y * unit);
            let fill = fill_col
                .and_then(|f| scales.map_color(&Aesthetic::Fill, &f[i]))
                .unwrap_or(self.fill);
            let name = label_col
                .map(|l| super::tip_value(&l[i]))
                .unwrap_or_default();
            let value = fill_col
                .map(|f| super::tip_value(&f[i]))
                .unwrap_or_default();
            let tip = match (name.is_empty(), value.is_empty()) {
                (false, false) => format!("{name}: {value}"),
                (false, true) => name.clone(),
                _ => value.clone(),
            };
            super::set_mark(
                backend,
                (!tip.is_empty()).then_some(tip),
                (!name.is_empty()).then(|| name.clone()),
                super::series_key(data, i),
                fill_col.and_then(|f| super::raw_value(&f[i])),
            );
            super::set_row_extras(backend, data, i);
            let style = RectStyle {
                fill: Some(fill),
                stroke: None,
                stroke_width: 0.0,
                alpha: 1.0,
                clip: true,
                corner_radius: 0.0,
            };
            let poly = |k: usize, rad: f64, phase: f64| -> Vec<(f64, f64)> {
                (0..k)
                    .map(|j| {
                        let a = phase + 2.0 * PI * j as f64 / k as f64;
                        (c.0 + rad * a.cos(), c.1 + rad * a.sin())
                    })
                    .collect()
            };
            match self.shape {
                TileShape::Square => {
                    backend.draw_rect((c.0 - r, c.1 - r), (c.0 + r, c.1 + r), &style)?
                }
                // Pointy-top: vertices at ±30°, ±90°, ±150°; circumradius so
                // neighbours (one unit apart) just touch.
                TileShape::Hexagon => {
                    backend.draw_polygon(&poly(6, r * 2.0 / 3f64.sqrt(), PI / 6.0), &style)?
                }
                TileShape::Circle => backend.draw_polygon(&poly(48, r, 0.0), &style)?,
                TileShape::Diamond => backend.draw_polygon(&poly(4, r, 0.0), &style)?,
            }
            if !name.is_empty() {
                if let Some(size) = fit_size(&name, 1.6 * r, 1.6 * r, self.label_size, 5.0) {
                    labels.push((name, c, size, contrast_text(fill)));
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
                    face: FontFace::Bold,
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
        Box::new(StatIdentity)
    }
    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }
    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }
    fn name(&self) -> &str {
        "tilemap"
    }
    fn consumes_tooltips(&self) -> bool {
        true
    }
}

/// A common square tile layout of the 50 US states + DC: `(abbreviation,
/// column, row)` with row 0 at the top — join it to your data by state and
/// map `x = column`, `y = row`.
pub fn us_state_grid() -> Vec<(&'static str, i64, i64)> {
    vec![
        ("AK", 0, 0),
        ("ME", 11, 0),
        ("VT", 10, 1),
        ("NH", 11, 1),
        ("WA", 1, 2),
        ("ID", 2, 2),
        ("MT", 3, 2),
        ("ND", 4, 2),
        ("MN", 5, 2),
        ("IL", 6, 2),
        ("WI", 7, 2),
        ("MI", 8, 2),
        ("NY", 9, 2),
        ("RI", 10, 2),
        ("MA", 11, 2),
        ("OR", 1, 3),
        ("NV", 2, 3),
        ("WY", 3, 3),
        ("SD", 4, 3),
        ("IA", 5, 3),
        ("IN", 6, 3),
        ("OH", 7, 3),
        ("PA", 8, 3),
        ("NJ", 9, 3),
        ("CT", 10, 3),
        ("CA", 1, 4),
        ("UT", 2, 4),
        ("CO", 3, 4),
        ("NE", 4, 4),
        ("MO", 5, 4),
        ("KY", 6, 4),
        ("WV", 7, 4),
        ("VA", 8, 4),
        ("MD", 9, 4),
        ("DE", 10, 4),
        ("AZ", 2, 5),
        ("NM", 3, 5),
        ("KS", 4, 5),
        ("AR", 5, 5),
        ("TN", 6, 5),
        ("NC", 7, 5),
        ("SC", 8, 5),
        ("DC", 9, 5),
        ("OK", 4, 6),
        ("LA", 5, 6),
        ("MS", 6, 6),
        ("AL", 7, 6),
        ("GA", 8, 6),
        ("HI", 0, 7),
        ("TX", 4, 7),
        ("FL", 9, 7),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_is_complete_and_unique() {
        let g = us_state_grid();
        assert_eq!(g.len(), 51);
        let mut cells: Vec<(i64, i64)> = g.iter().map(|s| (s.1, s.2)).collect();
        cells.sort();
        cells.dedup();
        assert_eq!(cells.len(), 51, "no two states share a cell");
        assert_eq!(
            centre(TileShape::Hexagon, 2.0, 1.0),
            (2.5, 3f64.sqrt() / 2.0)
        );
        assert_eq!(centre(TileShape::Square, 2.0, 1.0), (2.0, 1.0));
    }
}
