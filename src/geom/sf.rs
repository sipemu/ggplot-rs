//! `geom_sf` — render simple-features geometry (feature `sf`).
//!
//! Geometry travels through the pipeline as WKT in a `Value::Str` column named
//! `geometry`. [`StatSf`] parses it to derive the x/y extent (so the axes train
//! over the map) while preserving the geometry and data columns; [`GeomSf`]
//! re-parses each feature at draw time and dispatches by geometry type.

use crate::aes::Aesthetic;
use crate::coord::Coord;
use crate::data::{DataFrame, Value};
use crate::position::identity::PositionIdentity;
use crate::position::Position;
use crate::render::backend::{DrawBackend, LineStyle, Linetype, PointShape, PointStyle, RectStyle};
use crate::render::RenderError;
use crate::scale::ScaleSet;
use crate::spatial::{parse_wkt, Geometry, SfProjection};
use crate::stat::Stat;
use crate::theme::Theme;

use super::{Geom, GeomParams};

/// Render a `Value` compactly for a hover tooltip.
fn value_str(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        Value::Float(f) => {
            let r = (f * 1000.0).round() / 1000.0;
            format!("{r}")
        }
        Value::Integer(i) => format!("{i}"),
        Value::Bool(b) => format!("{b}"),
        _ => String::new(),
    }
}

/// Bounding box of a geometry after applying `proj` to every coordinate.
fn projected_bounds(g: &Geometry, proj: SfProjection) -> Option<(f64, f64, f64, f64)> {
    let mut b: Option<(f64, f64, f64, f64)> = None;
    g.for_each_coord(&mut |c| {
        let [x, y] = proj.project(c);
        b = Some(match b {
            None => (x, y, x, y),
            Some((mnx, mny, mxx, mxy)) => (mnx.min(x), mny.min(y), mxx.max(x), mxy.max(y)),
        });
    });
    b
}

/// Stat for `geom_sf`: parse the WKT `geometry` column and append per-feature
/// bounding-box columns (`xmin`/`xmax`/`ymin`/`ymax`) — in projected map units —
/// so the X/Y scales train over the extent, keeping every original column for
/// drawing.
#[derive(Default)]
pub struct StatSf {
    pub projection: SfProjection,
}

impl Stat for StatSf {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let mut out = data.clone();
        let Some(geom) = data.column("geometry") else {
            return out;
        };
        let n = geom.len();
        let (mut xmin, mut xmax) = (Vec::with_capacity(n), Vec::with_capacity(n));
        let (mut ymin, mut ymax) = (Vec::with_capacity(n), Vec::with_capacity(n));
        for v in geom.iter() {
            let bounds = match v {
                Value::Str(s) => parse_wkt(s).and_then(|g| projected_bounds(&g, self.projection)),
                _ => None,
            };
            match bounds {
                Some((mnx, mny, mxx, mxy)) => {
                    xmin.push(Value::Float(mnx));
                    xmax.push(Value::Float(mxx));
                    ymin.push(Value::Float(mny));
                    ymax.push(Value::Float(mxy));
                }
                None => {
                    xmin.push(Value::Na);
                    xmax.push(Value::Na);
                    ymin.push(Value::Na);
                    ymax.push(Value::Na);
                }
            }
        }
        out.add_column("xmin".to_string(), xmin);
        out.add_column("xmax".to_string(), xmax);
        out.add_column("ymin".to_string(), ymin);
        out.add_column("ymax".to_string(), ymax);
        out
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![]
    }

    fn name(&self) -> &str {
        "sf"
    }
}

/// Render simple-features geometry from a `geometry` column of WKT strings.
///
/// The `fill` aesthetic is the primary data channel (colours polygons, points
/// and lines alike — e.g. `aes(fill = region)` for a choropleth); polygons also
/// take a `color` border.
pub struct GeomSf {
    pub fill: (u8, u8, u8),
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub line_width: f64,
    pub point_size: f64,
    /// Map projection applied to lon/lat before scaling (default: none).
    pub projection: SfProjection,
}

impl Default for GeomSf {
    fn default() -> Self {
        GeomSf {
            fill: (97, 156, 255),
            color: (50, 50, 50),
            alpha: 0.85,
            line_width: 0.5,
            point_size: 3.0,
            projection: SfProjection::default(),
        }
    }
}

impl GeomSf {
    /// Set the map projection applied to lon/lat coordinates.
    pub fn project(mut self, projection: SfProjection) -> Self {
        self.projection = projection;
        self
    }

    fn draw_geometry(
        &self,
        g: &Geometry,
        project: &impl Fn([f64; 2]) -> (f64, f64),
        fill: (u8, u8, u8),
        color: (u8, u8, u8),
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        match g {
            Geometry::Empty => {}
            Geometry::Point(c) => self.draw_point(*c, project, fill, backend)?,
            Geometry::MultiPoint(cs) => {
                for c in cs {
                    self.draw_point(*c, project, fill, backend)?;
                }
            }
            Geometry::LineString(cs) => self.draw_path(cs, project, fill, backend)?,
            Geometry::MultiLineString(lines) => {
                for l in lines {
                    self.draw_path(l, project, fill, backend)?;
                }
            }
            Geometry::Polygon(rings) => self.draw_rings(rings, project, fill, color, backend)?,
            Geometry::MultiPolygon(polys) => {
                for p in polys {
                    self.draw_rings(p, project, fill, color, backend)?;
                }
            }
            Geometry::Collection(parts) => {
                for part in parts {
                    self.draw_geometry(part, project, fill, color, backend)?;
                }
            }
        }
        Ok(())
    }

    fn draw_point(
        &self,
        c: [f64; 2],
        project: &impl Fn([f64; 2]) -> (f64, f64),
        color: (u8, u8, u8),
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        backend.draw_circle(
            project(c),
            self.point_size,
            &PointStyle {
                color,
                alpha: self.alpha,
                filled: true,
                shape: PointShape::Circle,
            },
        )
    }

    fn draw_path(
        &self,
        cs: &[[f64; 2]],
        project: &impl Fn([f64; 2]) -> (f64, f64),
        color: (u8, u8, u8),
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        if cs.len() < 2 {
            return Ok(());
        }
        let pts: Vec<(f64, f64)> = cs.iter().map(|c| project(*c)).collect();
        backend.draw_line(
            &pts,
            &LineStyle {
                color,
                alpha: self.alpha,
                width: self.line_width.max(1.0),
                linetype: Linetype::Solid,
            },
        )
    }

    fn draw_rings(
        &self,
        rings: &[Vec<[f64; 2]>],
        project: &impl Fn([f64; 2]) -> (f64, f64),
        fill: (u8, u8, u8),
        color: (u8, u8, u8),
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        for (ri, ring) in rings.iter().enumerate() {
            if ring.len() < 3 {
                continue;
            }
            let pts: Vec<(f64, f64)> = ring.iter().map(|c| project(*c)).collect();
            // Exterior ring is filled + stroked; interior rings (holes) are
            // stroked only — draw_polygon can't cut holes out of the fill.
            let style = if ri == 0 {
                RectStyle {
                    fill: Some(fill),
                    stroke: (self.line_width > 0.0).then_some(color),
                    stroke_width: self.line_width,
                    alpha: self.alpha,
                    clip: true,
                }
            } else {
                RectStyle {
                    fill: None,
                    stroke: Some(color),
                    stroke_width: self.line_width.max(0.5),
                    alpha: self.alpha,
                    clip: true,
                }
            };
            backend.draw_polygon(&pts, &style)?;
        }
        Ok(())
    }
}

impl Geom for GeomSf {
    fn draw(
        &self,
        data: &DataFrame,
        coord: &dyn Coord,
        scales: &ScaleSet,
        _theme: &Theme,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let geom_col = data
            .column("geometry")
            .ok_or(RenderError::MissingAesthetic("geometry".into()))?;
        let fill_col = data.column("fill");
        let color_col = data.column("color");
        // A `label` mapping (and/or the fill value) becomes a hover tooltip.
        let label_col = data.column("label");

        let plot_area = backend.plot_area();
        let x_scale = scales.get(&Aesthetic::X);
        let y_scale = scales.get(&Aesthetic::Y);
        let project = |c: [f64; 2]| {
            let [x, y] = self.projection.project(c);
            let vx = Value::Float(x);
            let vy = Value::Float(y);
            let nx = x_scale.map(|s| s.map(&vx)).unwrap_or(0.0);
            let ny = y_scale.map(|s| s.map(&vy)).unwrap_or(0.0);
            coord.transform((nx, ny), &plot_area)
        };

        for i in 0..data.nrows() {
            let g = match &geom_col[i] {
                Value::Str(s) => parse_wkt(s),
                _ => None,
            };
            let Some(g) = g else { continue };
            let fill = fill_col
                .and_then(|fc| scales.map_color(&Aesthetic::Fill, &fc[i]))
                .unwrap_or(self.fill);
            let color = color_col
                .and_then(|cc| scales.map_color(&Aesthetic::Color, &cc[i]))
                .unwrap_or(self.color);

            let label = label_col
                .map(|c| value_str(&c[i]))
                .filter(|s| !s.is_empty());
            let fill_val = fill_col.map(|c| value_str(&c[i])).filter(|s| !s.is_empty());
            backend.set_tooltip(match (label, fill_val) {
                (Some(l), Some(f)) => Some(format!("{l}: {f}")),
                (Some(l), None) => Some(l),
                (None, Some(f)) => Some(f),
                (None, None) => None,
            });
            self.draw_geometry(&g, &project, fill, color, backend)?;
        }
        backend.set_tooltip(None);
        Ok(())
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        vec![]
    }

    fn default_stat(&self) -> Box<dyn Stat> {
        Box::new(StatSf {
            projection: self.projection,
        })
    }

    fn default_position(&self) -> Box<dyn Position> {
        Box::new(PositionIdentity)
    }

    fn default_params(&self) -> GeomParams {
        GeomParams::default()
    }

    fn name(&self) -> &str {
        "sf"
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }
}
