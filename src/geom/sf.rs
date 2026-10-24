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
use crate::render::backend::{
    DrawBackend, FillPattern, LineStyle, Linetype, PointShape, PointStyle, RectStyle,
};
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
/// take a `color` border. A `size` aesthetic on `Point`/`MultiPoint` rows
/// (`aes(size = "pop")`, graduated symbols — typically paired with
/// `scale_size_area()` so the circle *area* is proportional to the value)
/// scales each point's radius independently and draws largest first, so a
/// big circle never hides a smaller one at the same spot
/// (DataZooDE/anofox-visualization#420, #194; ggplot2/ggspatial convention).
pub struct GeomSf {
    pub fill: (u8, u8, u8),
    pub color: (u8, u8, u8),
    pub alpha: f64,
    pub line_width: f64,
    /// A `Point`/`MultiPoint` row's circle radius when no `size` aesthetic
    /// is mapped, or its mapped value doesn't resolve (no scale trained,
    /// non-numeric). A mapped `size` (e.g. `aes(size = "pop")`, typically
    /// with `scale_size_area()` for graduated symbols) overrides this per
    /// row — see [`Geom::draw`]'s largest-first draw order.
    pub point_size: f64,
    /// Map projection applied to lon/lat before scaling (default: none).
    pub projection: SfProjection,
    /// Render-resolution simplification tolerance, in output pixels
    /// (Ramer-Douglas-Peucker, run in screen space after projecting): a
    /// vertex closer than this to the line between its neighbours is
    /// dropped. `0.0` disables it. Default `0.5` — a boundary made of
    /// 10^5 points draws a fraction of that at a few hundred pixels across,
    /// with no visible difference (#178; ported from anofox-visualization's
    /// `map/write.rs`).
    pub simplify_px: f64,
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
            simplify_px: 0.5,
        }
    }
}

impl GeomSf {
    /// Set the map projection applied to lon/lat coordinates.
    pub fn project(mut self, projection: SfProjection) -> Self {
        self.projection = projection;
        self
    }

    /// Set the render-resolution simplification tolerance (output pixels);
    /// `0.0` draws every vertex. See [`GeomSf::simplify_px`].
    pub fn simplify(mut self, tolerance_px: f64) -> Self {
        self.simplify_px = tolerance_px.max(0.0);
        self
    }

    /// Simplify a screen-space path to within `self.simplify_px` (no-op
    /// when that's `0` or the path is already too short to simplify).
    fn simplified(&self, pts: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
        if self.simplify_px <= 0.0 || pts.len() < 3 {
            return pts;
        }
        simplify_path(&pts, self.simplify_px)
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_geometry(
        &self,
        g: &Geometry,
        project: &impl Fn([f64; 2]) -> (f64, f64),
        fill: (u8, u8, u8),
        color: (u8, u8, u8),
        size: f64,
        pattern: Option<&FillPattern>,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        match g {
            Geometry::Empty => {}
            Geometry::Point(c) => self.draw_point(*c, project, fill, size, backend)?,
            Geometry::MultiPoint(cs) => {
                for c in cs {
                    self.draw_point(*c, project, fill, size, backend)?;
                }
            }
            Geometry::LineString(cs) => self.draw_path(cs, project, fill, backend)?,
            Geometry::MultiLineString(lines) => {
                for l in lines {
                    self.draw_path(l, project, fill, backend)?;
                }
            }
            Geometry::Polygon(rings) => {
                self.draw_rings(rings, project, fill, color, pattern, backend)?
            }
            Geometry::MultiPolygon(polys) => {
                // #213: one mark per data row, not one per part. Every
                // part's rings (exterior + holes) go into a single
                // `draw_rings` call so the backend emits one `<path>` with
                // one `M…Z` subpath per ring/part under the even-odd fill
                // rule, instead of a separate mark (and tooltip/row-attrs)
                // per part. Correct under evenodd even when concatenated:
                // a point inside part A crosses A's rings an odd number of
                // times and B's rings zero times (parts of a valid
                // MultiPolygon never overlap), so the union still fills
                // each part with its own holes cut out.
                let rings: Vec<Vec<[f64; 2]>> =
                    polys.iter().flat_map(|p| p.iter().cloned()).collect();
                self.draw_rings(&rings, project, fill, color, pattern, backend)?;
            }
            Geometry::Collection(parts) => {
                for part in parts {
                    self.draw_geometry(part, project, fill, color, size, pattern, backend)?;
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
        size: f64,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        backend.draw_circle(
            project(c),
            size,
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
        let pts = self.simplified(pts);
        backend.draw_line(
            &pts,
            &LineStyle {
                color,
                alpha: self.alpha,
                width: self.line_width.max(1.0),
                linetype: Linetype::Solid,
                ..Default::default()
            },
        )
    }

    /// Draw a set of rings — for a single `Polygon`, `rings[0]` the
    /// exterior and every other ring a hole; for a `MultiPolygon` (#213),
    /// every part's rings concatenated, part boundaries undistinguished —
    /// as a single [`DrawBackend::draw_polygon_rings`] call, so the
    /// backend cuts the holes out of the fill under the even-odd rule
    /// rather than drawing them as separate, merely-stroked shapes (#178),
    /// and a multi-part row draws as one mark, not one per part (#213).
    /// Each ring is simplified to the render resolution first (screen
    /// space, [`GeomSf::simplify_px`]); a ring that collapses below 3
    /// points keeps its original vertices rather than vanishing (still
    /// simplified downstream by the backend's own rounding).
    fn draw_rings(
        &self,
        rings: &[Vec<[f64; 2]>],
        project: &impl Fn([f64; 2]) -> (f64, f64),
        fill: (u8, u8, u8),
        color: (u8, u8, u8),
        pattern: Option<&FillPattern>,
        backend: &mut dyn DrawBackend,
    ) -> Result<(), RenderError> {
        let mut screen_rings: Vec<Vec<(f64, f64)>> = Vec::with_capacity(rings.len());
        for ring in rings {
            if ring.len() < 3 {
                continue;
            }
            let pts: Vec<(f64, f64)> = ring.iter().map(|c| project(*c)).collect();
            let simplified = self.simplified(pts.clone());
            screen_rings.push(if simplified.len() >= 3 {
                simplified
            } else {
                pts
            });
        }
        if screen_rings.is_empty() {
            return Ok(());
        }
        let style = RectStyle {
            fill: Some(fill),
            stroke: (self.line_width > 0.0).then_some(color),
            stroke_width: self.line_width,
            alpha: self.alpha,
            clip: true,
            corner_radius: 0.0,
        };
        // #214: a compact (relative-command) path encoding, at 1-decimal
        // precision, whenever render-resolution simplification is on at
        // all (the default) — simplified vertices already give up exact
        // position to within `simplify_px`, so rounding to a tenth of a
        // pixel on top loses nothing visible. `.simplify(0.0)` (every
        // vertex, no tolerance) keeps the exact absolute encoding instead.
        let precision = (self.simplify_px > 0.0).then_some(1u8);
        // #206: a no-data row's pattern fill, same ring set and stroke.
        backend.draw_polygon_rings_precise(&screen_rings, pattern, precision, &style)
    }
}

/// Simplify a screen-space path to within `tol` pixels (Ramer-Douglas-Peucker).
/// The first and last points always stay. Ported from anofox-visualization's
/// `map/write.rs` (DataZooDE/anofox-visualization#318), minus its tenth-pixel
/// quantization — `GeomSf`'s own coordinate formatting already rounds that
/// finely at serialization.
fn simplify_path(pts: &[(f64, f64)], tol: f64) -> Vec<(f64, f64)> {
    let finite: Vec<(f64, f64)> = pts
        .iter()
        .copied()
        .filter(|(x, y)| x.is_finite() && y.is_finite())
        .collect();
    if finite.len() < 3 {
        return finite;
    }
    let last = finite.len() - 1;
    let mut keep = vec![false; finite.len()];
    keep[0] = true;
    keep[last] = true;
    let mut stack = vec![(0usize, last)];
    while let Some((a, b)) = stack.pop() {
        if b <= a + 1 {
            continue;
        }
        let (mut far, mut dmax) = (a, -1.0);
        for (i, p) in finite.iter().enumerate().take(b).skip(a + 1) {
            let d = seg_dist(*p, finite[a], finite[b]);
            if d > dmax {
                dmax = d;
                far = i;
            }
        }
        if dmax > tol {
            keep[far] = true;
            stack.push((a, far));
            stack.push((far, b));
        }
    }
    finite
        .into_iter()
        .zip(keep)
        .filter_map(|(p, k)| k.then_some(p))
        .collect()
}

/// Distance from `p` to the segment `a`–`b`.
fn seg_dist(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (ex, ey) = (a.0 + t * dx - p.0, a.1 + t * dy - p.1);
    (ex * ex + ey * ey).sqrt()
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
        let size_col = data.column("size");
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
        // A mapped `size` (graduated symbols, e.g. `aes(size = "pop")` with
        // `scale_size_area()`) resolves to this row's circle radius — see
        // `PointDefaults`/`draw_points` in `geom/point.rs` for the same
        // fallback chain (a trained scale wins, else the raw numeric value,
        // else the unmapped `point_size` default).
        let size_of = |i: usize| -> f64 {
            size_col
                .and_then(|c| scales.map_size(&c[i]))
                .or_else(|| size_col.and_then(|c| c[i].as_f64()))
                .unwrap_or(self.point_size)
        };
        // Largest symbol first, so a big circle never covers a smaller one
        // drawn at the same spot (ggplot2/graduated-symbol-map convention).
        // Stable: rows without a `size` aesthetic (or tied sizes) keep their
        // original relative order.
        let mut order: Vec<usize> = (0..data.nrows()).collect();
        if size_col.is_some() {
            order.sort_by(|&a, &b| size_of(b).total_cmp(&size_of(a)));
        }

        for i in order {
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
            let size = size_of(i);
            // #206: a no-data row (Fill mapped, value missing) draws its
            // scale's na_pattern instead of a flat na colour, when set.
            let pattern = fill_col
                .filter(|fc| matches!(fc[i], Value::Na))
                .and_then(|_| scales.na_fill_pattern(&Aesthetic::Fill));

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
            // `data-fvalue` (B2-4), same formatter-applied-to-the-raw-value
            // writer as `GeomPoint`/`GeomHex` (#238): `fill` is this geom's
            // primary data channel (a choropleth's measure, or a graduated
            // symbol's colour) so it wins when mapped; a `size`-only
            // graduated-symbol map (no fill measure, #238's motivating case)
            // falls back to `size`'s own formatter.
            if let Some(fc) = fill_col {
                super::set_mark_formatted(backend, scales, None, &Aesthetic::Fill, Some(&fc[i]));
            } else if let Some(sc) = size_col {
                super::set_mark_formatted(backend, scales, None, &Aesthetic::Size, Some(&sc[i]));
            } else {
                super::set_mark_formatted(backend, scales, None, &Aesthetic::Fill, None);
            }
            super::set_row_extras(backend, data, i);
            self.draw_geometry(&g, &project, fill, color, size, pattern.as_ref(), backend)?;
        }
        backend.set_tooltip(None);
        super::clear_mark(backend);
        super::clear_row_extras(backend);
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

    fn apply_element_geom(&mut self, el: &crate::theme::elements::ElementGeom) {
        if let (true, Some(ink)) = (self.color == (50, 50, 50), el.ink) {
            self.color = ink;
        }
    }

    fn consumes_tooltips(&self) -> bool {
        true
    }

    fn set_series_color(&mut self, color: (u8, u8, u8)) {
        self.fill = color;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simplify_path_keeps_endpoints_and_drops_collinear_points() {
        let line: Vec<(f64, f64)> = (0..=20)
            .map(|i| (i as f64, if i % 2 == 0 { 0.0 } else { 0.1 }))
            .collect();
        let simplified = simplify_path(&line, 0.5);
        assert_eq!(simplified, vec![(0.0, 0.0), (20.0, 0.0)]);
    }

    #[test]
    fn simplify_path_keeps_a_real_bump() {
        let bump = [(0.0, 0.0), (5.0, 3.0), (10.0, 0.0)];
        assert_eq!(simplify_path(&bump, 0.5).len(), 3);
    }

    #[test]
    fn simplify_path_zero_tolerance_is_a_no_op() {
        let pts = [(0.0, 0.0), (1.0, 0.01), (2.0, 0.0), (3.0, 10.0)];
        // `0.0` tolerance never simplifies (handled at the GeomSf level, not
        // here — this exercises simplify_path itself at a tiny but nonzero
        // tolerance, confirming points genuinely off the line stay).
        let simplified = simplify_path(&pts, 1e-9);
        assert_eq!(simplified.len(), 4);
    }

    #[test]
    fn simplify_path_drops_non_finite_points() {
        let pts = [(0.0, 0.0), (f64::NAN, 1.0), (1.0, 0.0), (2.0, 0.0)];
        let simplified = simplify_path(&pts, 0.5);
        assert!(simplified
            .iter()
            .all(|(x, y)| x.is_finite() && y.is_finite()));
        assert_eq!(simplified.len(), 2, "endpoints of the finite run");
    }

    #[test]
    fn default_simplify_px_is_half_a_pixel() {
        assert_eq!(GeomSf::default().simplify_px, 0.5);
    }

    #[test]
    fn simplify_builder_clamps_negative_to_zero() {
        let g = GeomSf::default().simplify(-1.0);
        assert_eq!(g.simplify_px, 0.0);
    }
}
