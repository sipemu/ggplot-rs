//! The pole of inaccessibility of a polygon (Mapbox's `polylabel`): the
//! inside point farthest from the outline. Unlike the centroid it always
//! falls inside the shape, so a C-shaped region's anchor stays in its
//! spine and a ring-shaped one's off its hole.
//!
//! Works in whatever plane the rings are in — lon/lat, projected metres or
//! screen pixels — with `precision` in the same units. `geom_sf_text` /
//! `geom_sf_label` call it on screen rings; a consumer placing symbols at a
//! region's visual centre calls it on data or projected rings.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use super::{Coord, Geometry};

/// Cells the search refines before it stops (a guard against a
/// pathological ring; real polygons converge in far fewer).
const MAX_STEPS: usize = 10_000;

/// The pole of inaccessibility of a polygon, to within `precision` (in the
/// rings' own units). `rings[0]` is the exterior, the rest are holes, as in
/// [`Geometry::Polygon`].
///
/// `None` when there is no exterior with at least three vertices. A
/// degenerate exterior (zero width or height) returns its lowest corner,
/// as mapbox/polylabel does.
///
/// ```
/// use ggplot_rs::spatial::polylabel;
/// // a C open to the right: the centroid falls in the gap, the pole doesn't
/// let c = vec![
///     [0.0, 0.0], [10.0, 0.0], [10.0, 2.0], [2.0, 2.0],
///     [2.0, 8.0], [10.0, 8.0], [10.0, 10.0], [0.0, 10.0],
/// ];
/// let [x, _] = polylabel(&[c], 0.1).unwrap();
/// assert!(x < 2.0);
/// ```
pub fn polylabel(rings: &[Vec<Coord>], precision: f64) -> Option<Coord> {
    polylabel_with_distance(rings, precision).map(|(p, _)| p)
}

/// [`polylabel`], plus the pole's distance to the outline (the radius of
/// the largest circle centred there that fits inside the polygon).
pub fn polylabel_with_distance(rings: &[Vec<Coord>], precision: f64) -> Option<(Coord, f64)> {
    let outer = rings.first().filter(|r| r.len() >= 3)?;
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for &[x, y] in outer {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    let (w, h) = (x1 - x0, y1 - y0);
    let size = w.min(h);
    if size.is_nan() || size <= 0.0 {
        return Some(([x0, y0], 0.0));
    }
    let precision = if precision > 0.0 {
        precision
    } else {
        size * 1e-3
    };
    let mut heap = BinaryHeap::new();
    let mut half = size / 2.0;
    let mut x = x0;
    while x < x1 {
        let mut y = y0;
        while y < y1 {
            heap.push(Cell::new(x + half, y + half, half, rings));
            y += size;
        }
        x += size;
    }
    // the centroid of the outline is a good first guess, the box centre too
    let [cx, cy] = ring_centroid(outer);
    let mut best = Cell::new(cx, cy, 0.0, rings);
    let bbox = Cell::new(x0 + w / 2.0, y0 + h / 2.0, 0.0, rings);
    if bbox.d > best.d {
        best = bbox;
    }
    let mut steps = 0;
    while let Some(c) = heap.pop() {
        let (cx, cy, ch, cmax) = (c.x, c.y, c.h, c.max);
        if c.d > best.d {
            best = c;
        }
        if cmax - best.d <= precision {
            continue;
        }
        steps += 1;
        if steps > MAX_STEPS {
            break;
        }
        half = ch / 2.0;
        for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            heap.push(Cell::new(cx + dx * half, cy + dy * half, half, rings));
        }
    }
    Some(([best.x, best.y], best.d))
}

impl Geometry {
    /// The point a label or symbol for this feature goes at, to within
    /// `precision` (in the geometry's own units): a point is itself (a
    /// `MultiPoint` its first point), a polygon its [`polylabel`] — a
    /// `MultiPolygon` (or a collection) that of its largest part by area.
    /// `None` for lines, empty geometries and collections without either.
    pub fn label_point(&self, precision: f64) -> Option<Coord> {
        if let Some(p) = first_point(self) {
            return Some(p);
        }
        let mut best: Option<(f64, &Vec<Vec<Coord>>)> = None;
        largest_polygon(self, &mut best);
        best.and_then(|(_, rings)| polylabel(rings, precision))
    }
}

fn first_point(g: &Geometry) -> Option<Coord> {
    match g {
        Geometry::Point(c) => Some(*c),
        Geometry::MultiPoint(cs) => cs.first().copied(),
        Geometry::Collection(gs) => gs.iter().find_map(first_point),
        _ => None,
    }
}

fn largest_polygon<'a>(g: &'a Geometry, best: &mut Option<(f64, &'a Vec<Vec<Coord>>)>) {
    let mut consider = |rings: &'a Vec<Vec<Coord>>| {
        let a = rings.first().map_or(0.0, |r| ring_area(r));
        if best.as_ref().is_none_or(|b| a > b.0) {
            *best = Some((a, rings));
        }
    };
    match g {
        Geometry::Polygon(rings) => consider(rings),
        Geometry::MultiPolygon(ps) => ps.iter().for_each(consider),
        Geometry::Collection(gs) => gs.iter().for_each(|g| largest_polygon(g, best)),
        _ => {}
    }
}

/// The unsigned (shoelace) area of a ring.
pub(crate) fn ring_area(r: &[Coord]) -> f64 {
    let n = r.len();
    if n < 3 {
        return 0.0;
    }
    let mut s = 0.0;
    let mut j = n - 1;
    for i in 0..n {
        s += (r[j][0] + r[i][0]) * (r[j][1] - r[i][1]);
        j = i;
    }
    (s / 2.0).abs()
}

/// The area centroid of a ring (its first vertex when it has no area).
pub(crate) fn ring_centroid(r: &[Coord]) -> Coord {
    let (mut a, mut cx, mut cy) = (0.0, 0.0, 0.0);
    let n = r.len();
    if n == 0 {
        return [0.0, 0.0];
    }
    let mut j = n - 1;
    for i in 0..n {
        let (p, q) = (r[i], r[j]);
        let f = p[0] * q[1] - q[0] * p[1];
        cx += (p[0] + q[0]) * f;
        cy += (p[1] + q[1]) * f;
        a += f * 3.0;
        j = i;
    }
    if a.abs() < 1e-12 {
        return r[0];
    }
    [cx / a, cy / a]
}

/// Distance from `p` to the polygon outline (`rings[0]` exterior, the rest
/// holes), negative outside.
fn signed_dist(p: Coord, rings: &[Vec<Coord>]) -> f64 {
    let mut inside = false;
    let mut best = f64::INFINITY;
    for r in rings {
        let n = r.len();
        if n < 2 {
            continue;
        }
        let mut j = n - 1;
        for i in 0..n {
            let (a, b) = (r[i], r[j]);
            if (a[1] > p[1]) != (b[1] > p[1])
                && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
            {
                inside = !inside;
            }
            best = best.min(seg_dist2(p, a, b));
            j = i;
        }
    }
    let d = best.sqrt();
    if inside {
        d
    } else {
        -d
    }
}

fn seg_dist2(p: Coord, a: Coord, b: Coord) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 {
        (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (ex, ey) = (a[0] + t * dx - p[0], a[1] + t * dy - p[1]);
    ex * ex + ey * ey
}

struct Cell {
    x: f64,
    y: f64,
    h: f64,
    /// Signed distance to the outline (negative outside).
    d: f64,
    /// The best distance any point in the cell could have.
    max: f64,
}

impl Cell {
    fn new(x: f64, y: f64, h: f64, rings: &[Vec<Coord>]) -> Cell {
        let d = signed_dist([x, y], rings);
        Cell {
            x,
            y,
            h,
            d,
            max: d + h * std::f64::consts::SQRT_2,
        }
    }
}

impl PartialEq for Cell {
    fn eq(&self, o: &Self) -> bool {
        self.max == o.max
    }
}
impl Eq for Cell {}
impl PartialOrd for Cell {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Cell {
    fn cmp(&self, o: &Self) -> Ordering {
        self.max.total_cmp(&o.max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spatial::parse_wkt;

    fn c_shape() -> Vec<Coord> {
        vec![
            [0.0, 0.0],
            [10.0, 0.0],
            [10.0, 2.0],
            [2.0, 2.0],
            [2.0, 8.0],
            [10.0, 8.0],
            [10.0, 10.0],
            [0.0, 10.0],
        ]
    }

    #[test]
    fn the_pole_of_a_c_shape_is_inside_it() {
        let c = c_shape();
        let ([x, y], d) = polylabel_with_distance(std::slice::from_ref(&c), 0.1).unwrap();
        assert!(signed_dist([x, y], std::slice::from_ref(&c)) > 0.0);
        assert!(x < 2.0 && d > 0.8, "({x},{y}) in the spine, d = {d}");
        // the centroid falls in the gap
        assert!(signed_dist(ring_centroid(&c), &[c]) < 0.0);
    }

    #[test]
    fn a_squares_pole_is_its_centre() {
        let sq = vec![[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0]];
        let ([x, y], d) = polylabel_with_distance(&[sq], 0.01).unwrap();
        assert!((x - 2.0).abs() < 0.05 && (y - 2.0).abs() < 0.05);
        assert!((d - 2.0).abs() < 0.05);
    }

    #[test]
    fn a_hole_is_avoided() {
        // an 8×8 square with a 4×4 hole dead centre
        let outer = vec![[0.0, 0.0], [8.0, 0.0], [8.0, 8.0], [0.0, 8.0]];
        let hole = vec![[2.0, 2.0], [6.0, 2.0], [6.0, 6.0], [2.0, 6.0]];
        let [x, y] = polylabel(&[outer.clone(), hole.clone()], 0.1).unwrap();
        assert!(signed_dist([x, y], &[outer, hole]) > 0.0, "({x},{y})");
        assert!(!((2.0..=6.0).contains(&x) && (2.0..=6.0).contains(&y)));
    }

    // mapbox/polylabel's own test cases (test/test.js)
    #[test]
    fn degenerate_polygons_return_a_corner() {
        let line = vec![[0.0, 0.0], [1.0, 0.0], [2.0, 0.0], [0.0, 0.0]];
        assert_eq!(polylabel(&[line], 1.0), Some([0.0, 0.0]));
        let spike = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]];
        assert_eq!(polylabel(&[spike], 1.0), Some([0.0, 0.0]));
    }

    #[test]
    fn it_works_in_any_plane() {
        // the same C shape scaled to projected metres keeps its pole in the
        // spine: precision is in the rings' own units
        let m: Vec<Coord> = c_shape()
            .iter()
            .map(|[x, y]| [x * 1e5 + 4e5, y * 1e5 + 5.8e6])
            .collect();
        let [x, _] = polylabel(&[m], 100.0).unwrap();
        assert!(x < 2e5 + 4e5, "{x}");
    }

    #[test]
    fn no_exterior_no_pole() {
        assert_eq!(polylabel(&[], 1.0), None);
        assert_eq!(polylabel(&[vec![[0.0, 0.0], [1.0, 1.0]]], 1.0), None);
    }

    #[test]
    fn label_point_of_each_geometry_type() {
        let p = |s: &str| parse_wkt(s).unwrap();
        assert_eq!(p("POINT (3 4)").label_point(0.1), Some([3.0, 4.0]));
        assert_eq!(
            p("MULTIPOINT ((1 2), (5 6))").label_point(0.1),
            Some([1.0, 2.0])
        );
        assert_eq!(p("LINESTRING (0 0, 1 1)").label_point(0.1), None);
        // a multipolygon: the largest part's pole
        let [x, y] =
            p("MULTIPOLYGON (((0 0, 1 0, 1 1, 0 1, 0 0)), ((10 10, 20 10, 20 20, 10 20, 10 10)))")
                .label_point(0.01)
                .unwrap();
        assert!(
            (x - 15.0).abs() < 0.1 && (y - 15.0).abs() < 0.1,
            "({x},{y})"
        );
        assert_eq!(Geometry::Empty.label_point(0.1), None);
    }
}
