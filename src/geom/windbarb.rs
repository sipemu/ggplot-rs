//! Wind barbs: `geom_windbarb`. The standard meteorological glyph — a shaft
//! pointing to where the wind blows *from*, with half barbs (5 kt), full
//! barbs (10 kt) and pennants (50 kt) at its outer end; calm (< 2.5 kt) is a
//! circle. Speeds are rounded to the nearest 5 kt.

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineStyle, Linetype, PointStyle, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::stat::identity::StatIdentity;
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Speed unit of the speed column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WindUnit {
    #[default]
    Knots,
    MetersPerSecond,
}

/// Wind barbs. Map `x` (and optionally `y`; without it the barbs sit in a
/// row across the panel); name the speed and direction columns.
pub struct GeomWindbarb {
    /// Speed column.
    pub speed: String,
    /// Direction column: degrees the wind blows *from*, clockwise from north.
    pub direction: String,
    pub unit: WindUnit,
    /// Shaft length in pixels.
    pub length: f64,
    pub color: (u8, u8, u8),
    pub line_width: f64,
}

impl GeomWindbarb {
    pub fn new(speed: impl Into<String>, direction: impl Into<String>) -> Self {
        GeomWindbarb {
            speed: speed.into(),
            direction: direction.into(),
            unit: WindUnit::Knots,
            length: 28.0,
            color: (40, 40, 40),
            line_width: 1.3,
        }
    }

    /// Speeds are in metres per second (converted to knots for the glyph).
    pub fn meters_per_second(mut self) -> Self {
        self.unit = WindUnit::MetersPerSecond;
        self
    }
}

/// `(pennants, full barbs, half barbs)` for a speed in knots (rounded to 5).
pub(crate) fn barb_parts(knots: f64) -> (usize, usize, usize) {
    let s = (knots.max(0.0) / 5.0).round() as usize * 5;
    (s / 50, (s % 50) / 10, (s % 10) / 5)
}

/// A pixel position.
type Px = (f64, f64);

/// Line segments and pennant triangles (pixel coordinates) of one barb at
/// `at`, wind from `dir` degrees, `knots` speed, shaft length `len`.
pub(crate) fn barb_glyph(
    at: (f64, f64),
    dir: f64,
    knots: f64,
    len: f64,
) -> (Vec<[Px; 2]>, Vec<[Px; 3]>) {
    let rad = dir.to_radians();
    // Screen y grows downward: north is −y.
    let u = (rad.sin(), -rad.cos());
    // Barbs on the clockwise side of the shaft (northern-hemisphere style).
    let side = (-u.1, u.0);
    let tip = (at.0 + u.0 * len, at.1 + u.1 * len);
    let barb = 0.42 * len;
    let gap = 0.13 * len;
    let along = |d: f64| (tip.0 - u.0 * d, tip.1 - u.1 * d);
    // Barbs lean toward the tip.
    let out = |p: (f64, f64), l: f64| {
        (
            p.0 + side.0 * l + u.0 * l * 0.35,
            p.1 + side.1 * l + u.1 * l * 0.35,
        )
    };
    let (pennants, full, half) = barb_parts(knots);
    let mut lines = vec![[at, tip]];
    let mut tris = Vec::new();
    let mut d = 0.0;
    for _ in 0..pennants {
        let (a, b) = (along(d), along(d + gap));
        tris.push([a, out(a, barb), b]);
        d += gap * 1.15;
    }
    for _ in 0..full {
        let p = along(d);
        lines.push([p, out(p, barb)]);
        d += gap;
    }
    for _ in 0..half {
        // A lone half barb sits one step in from the tip, as on charts.
        if pennants == 0 && full == 0 {
            d += gap;
        }
        let p = along(d);
        lines.push([p, out(p, barb / 2.0)]);
    }
    (lines, tris)
}

impl Geom for GeomWindbarb {
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
        let y_col = data.column("y");
        let speed = data
            .column(&self.speed)
            .ok_or_else(|| RenderError::MissingAesthetic(self.speed.clone()))?;
        let dir = data
            .column(&self.direction)
            .ok_or_else(|| RenderError::MissingAesthetic(self.direction.clone()))?;
        let color_col = data.column("color");
        let area = backend.plot_area();
        let mx = |v: &Value| scales.get(&Aesthetic::X).map(|s| s.map(v)).unwrap_or(0.5);
        let my = |v: &Value| scales.get(&Aesthetic::Y).map(|s| s.map(v)).unwrap_or(0.5);
        let factor = match self.unit {
            WindUnit::Knots => 1.0,
            WindUnit::MetersPerSecond => 1.943_844,
        };
        for i in 0..data.nrows() {
            let (Some(s), Some(d)) = (speed[i].as_f64(), dir[i].as_f64()) else {
                continue;
            };
            let ny = y_col.map(|c| my(&c[i])).unwrap_or(0.5);
            let at = coord.transform((mx(&x_col[i]), ny), &area);
            if !(at.0.is_finite() && at.1.is_finite() && s.is_finite() && d.is_finite()) {
                continue;
            }
            let color = color_col
                .and_then(|c| scales.map_color(&Aesthetic::Color, &c[i]))
                .unwrap_or(self.color);
            let knots = s * factor;
            super::set_mark(
                backend,
                Some(format!(
                    "{}: {} from {}°",
                    super::tip_value(&x_col[i]),
                    super::tip_value(&speed[i]),
                    super::tip_value(&dir[i])
                )),
                Some(super::tip_value(&x_col[i])),
                super::series_key(data, i),
                super::raw_value(&speed[i]),
            );
            super::set_row_extras(backend, data, i);
            if knots < 2.5 {
                backend.draw_circle(
                    at,
                    4.0,
                    &PointStyle {
                        color,
                        alpha: 1.0,
                        filled: false,
                        ..Default::default()
                    },
                )?;
                continue;
            }
            let (lines, tris) = barb_glyph(at, d, knots, self.length);
            let style = LineStyle {
                color,
                alpha: 1.0,
                width: self.line_width,
                linetype: Linetype::Solid,
                ..Default::default()
            };
            for l in &lines {
                backend.draw_line(l, &style)?;
            }
            for t in &tris {
                backend.draw_polygon(
                    t,
                    &RectStyle {
                        fill: Some(color),
                        stroke: Some(color),
                        stroke_width: 0.5,
                        alpha: 1.0,
                        clip: true,
                        corner_radius: 0.0,
                    },
                )?;
            }
        }
        super::clear_mark(backend);
        super::clear_row_extras(backend);
        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![Aesthetic::X]
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
        "windbarb"
    }
    fn consumes_tooltips(&self) -> bool {
        true
    }
    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let (true, Some(ink)) = (self.color == (40, 40, 40), el.ink) {
            self.color = ink;
        }
    }
    /// Without a `y` mapping a hidden y extent keeps the panel's axes.
    fn setup_data(&self, data: &mut DataFrame, _warnings: &mut Vec<String>) {
        if !data.has_column("y") {
            let n = data.nrows();
            for (c, v) in [("ymin", 0.0), ("ymax", 1.0)] {
                if !data.has_column(c) {
                    data.add_column(c.into(), vec![Value::Float(v); n]);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn barb_counts_follow_the_wmo_convention() {
        assert_eq!(barb_parts(0.0), (0, 0, 0));
        assert_eq!(barb_parts(5.0), (0, 0, 1));
        assert_eq!(barb_parts(15.0), (0, 1, 1));
        assert_eq!(barb_parts(65.0), (1, 1, 1));
        assert_eq!(barb_parts(102.0), (2, 0, 0), "rounded to 100");
        assert_eq!(barb_parts(147.0), (2, 4, 1));
        assert_eq!(barb_parts(148.0), (3, 0, 0), "rounded to 150");
    }

    #[test]
    fn shaft_points_into_the_wind() {
        // Wind from the north: the shaft runs straight up from the station.
        let (lines, _) = barb_glyph((0.0, 0.0), 0.0, 10.0, 30.0);
        let [a, b] = lines[0];
        assert!((b.0 - a.0).abs() < 1e-9 && (b.1 - a.1 + 30.0).abs() < 1e-9);
        // From the east: to the right.
        let (lines, tris) = barb_glyph((0.0, 0.0), 90.0, 50.0, 30.0);
        assert!((lines[0][1].0 - 30.0).abs() < 1e-9);
        assert_eq!(tris.len(), 1, "one pennant");
    }
}
