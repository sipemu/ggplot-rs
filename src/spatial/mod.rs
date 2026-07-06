//! Minimal simple-features geometry model and a self-contained WKT parser,
//! backing `geom_sf` (feature `sf`). Kept dependency-free so the spatial layer
//! adds no heavy `geo`/`proj`/`gdal` tree to the core.

/// A 2-D coordinate `[x, y]` (longitude/easting, latitude/northing). Any Z/M
/// ordinates in the WKT are parsed and discarded.
pub type Coord = [f64; 2];

/// A simple-features geometry (the OGC/WKT set), with rings stored explicitly
/// so polygon holes are preserved.
#[derive(Debug, Clone, PartialEq)]
pub enum Geometry {
    Point(Coord),
    LineString(Vec<Coord>),
    /// Rings: index 0 is the exterior, the rest are holes.
    Polygon(Vec<Vec<Coord>>),
    MultiPoint(Vec<Coord>),
    MultiLineString(Vec<Vec<Coord>>),
    MultiPolygon(Vec<Vec<Vec<Coord>>>),
    Collection(Vec<Geometry>),
    Empty,
}

impl Geometry {
    /// Bounding box `(min_x, min_y, max_x, max_y)`, or `None` if empty.
    pub fn bounds(&self) -> Option<(f64, f64, f64, f64)> {
        let mut b: Option<(f64, f64, f64, f64)> = None;
        self.for_each_coord(&mut |[x, y]| {
            b = Some(match b {
                None => (x, y, x, y),
                Some((mnx, mny, mxx, mxy)) => (mnx.min(x), mny.min(y), mxx.max(x), mxy.max(y)),
            });
        });
        b
    }

    /// Visit every coordinate in the geometry (all parts, all rings).
    pub fn for_each_coord(&self, f: &mut impl FnMut(Coord)) {
        match self {
            Geometry::Empty => {}
            Geometry::Point(c) => f(*c),
            Geometry::LineString(cs) | Geometry::MultiPoint(cs) => cs.iter().for_each(|c| f(*c)),
            Geometry::Polygon(rings) | Geometry::MultiLineString(rings) => {
                rings.iter().flatten().for_each(|c| f(*c))
            }
            Geometry::MultiPolygon(polys) => polys.iter().flatten().flatten().for_each(|c| f(*c)),
            Geometry::Collection(gs) => gs.iter().for_each(|g| g.for_each_coord(f)),
        }
    }
}

/// A map projection applied to longitude/latitude before scaling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SfProjection {
    /// Longitude/latitude used directly (equirectangular / plate carrée).
    #[default]
    PlateCarree,
    /// Spherical Web Mercator (conformal; latitude clamped to ±85.05°).
    Mercator,
}

impl SfProjection {
    /// Project a `[lon, lat]` (degrees) coordinate into map units.
    pub fn project(&self, [lon, lat]: Coord) -> Coord {
        match self {
            SfProjection::PlateCarree => [lon, lat],
            SfProjection::Mercator => {
                let lat = lat.clamp(-85.051_129, 85.051_129);
                let y = (std::f64::consts::FRAC_PI_4 + lat.to_radians() / 2.0)
                    .tan()
                    .ln();
                [lon.to_radians(), y]
            }
        }
    }
}

/// Parse a WKT string (case-insensitive, optional `Z`/`M`, `EMPTY`) into a
/// [`Geometry`]. Returns `None` on malformed input.
pub fn parse_wkt(input: &str) -> Option<Geometry> {
    let mut p = Parser {
        b: input.as_bytes(),
        i: 0,
    };
    let g = p.geometry()?;
    p.skip_ws();
    Some(g)
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn skip_ws(&mut self) {
        while self.i < self.b.len() && (self.b[self.i] as char).is_whitespace() {
            self.i += 1;
        }
    }
    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }
    fn eat(&mut self, c: u8) -> bool {
        self.skip_ws();
        if self.peek() == Some(c) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn keyword(&mut self) -> String {
        self.skip_ws();
        let start = self.i;
        while self.i < self.b.len() && (self.b[self.i] as char).is_ascii_alphabetic() {
            self.i += 1;
        }
        String::from_utf8_lossy(&self.b[start..self.i]).to_ascii_uppercase()
    }
    fn number(&mut self) -> Option<f64> {
        self.skip_ws();
        let start = self.i;
        if matches!(self.peek(), Some(b'+') | Some(b'-')) {
            self.i += 1;
        }
        while matches!(self.peek(), Some(c) if (c as char).is_ascii_digit() || c == b'.' || c == b'e' || c == b'E' || c == b'+' || c == b'-')
        {
            self.i += 1;
        }
        std::str::from_utf8(&self.b[start..self.i])
            .ok()?
            .parse()
            .ok()
    }
    /// `x y [z] [m]` — keep the first two ordinates, discard any extra.
    fn coord(&mut self) -> Option<Coord> {
        let x = self.number()?;
        let y = self.number()?;
        loop {
            let save = self.i;
            if self.number().is_none() {
                self.i = save;
                break;
            }
        }
        Some([x, y])
    }
    /// `EMPTY` token if present at the cursor.
    fn is_empty_kw(&mut self) -> bool {
        let save = self.i;
        if self.keyword() == "EMPTY" {
            true
        } else {
            self.i = save;
            false
        }
    }
    /// `( coord , coord , ... )`
    fn coord_list(&mut self) -> Option<Vec<Coord>> {
        if !self.eat(b'(') {
            return None;
        }
        let mut v = vec![self.coord()?];
        while self.eat(b',') {
            v.push(self.coord()?);
        }
        if !self.eat(b')') {
            return None;
        }
        Some(v)
    }
    /// `( coord_list , coord_list , ... )` — polygon rings / multilinestring.
    fn ring_list(&mut self) -> Option<Vec<Vec<Coord>>> {
        if !self.eat(b'(') {
            return None;
        }
        let mut v = vec![self.coord_list()?];
        while self.eat(b',') {
            v.push(self.coord_list()?);
        }
        if !self.eat(b')') {
            return None;
        }
        Some(v)
    }
    /// MULTIPOINT accepts both `(1 2, 3 4)` and `((1 2), (3 4))`.
    fn multipoint(&mut self) -> Option<Vec<Coord>> {
        if !self.eat(b'(') {
            return None;
        }
        let mut v = Vec::new();
        loop {
            self.skip_ws();
            if self.peek() == Some(b'(') {
                v.push(*self.coord_list()?.first()?);
            } else {
                v.push(self.coord()?);
            }
            if !self.eat(b',') {
                break;
            }
        }
        if !self.eat(b')') {
            return None;
        }
        Some(v)
    }
    /// `( polygon , polygon , ... )` where each polygon is a ring list.
    fn polygon_list(&mut self) -> Option<Vec<Vec<Vec<Coord>>>> {
        if !self.eat(b'(') {
            return None;
        }
        let mut v = vec![self.ring_list()?];
        while self.eat(b',') {
            v.push(self.ring_list()?);
        }
        if !self.eat(b')') {
            return None;
        }
        Some(v)
    }
    fn geometry(&mut self) -> Option<Geometry> {
        let kw = self.keyword();
        // optional Z / M / ZM dimensionality tag
        self.skip_ws();
        while matches!(
            self.peek(),
            Some(b'Z') | Some(b'z') | Some(b'M') | Some(b'm')
        ) {
            self.i += 1;
            self.skip_ws();
        }
        if self.is_empty_kw() {
            return Some(Geometry::Empty);
        }
        match kw.as_str() {
            "POINT" => Some(Geometry::Point(*self.coord_list()?.first()?)),
            "LINESTRING" => Some(Geometry::LineString(self.coord_list()?)),
            "POLYGON" => Some(Geometry::Polygon(self.ring_list()?)),
            "MULTIPOINT" => Some(Geometry::MultiPoint(self.multipoint()?)),
            "MULTILINESTRING" => Some(Geometry::MultiLineString(self.ring_list()?)),
            "MULTIPOLYGON" => Some(Geometry::MultiPolygon(self.polygon_list()?)),
            "GEOMETRYCOLLECTION" => {
                if !self.eat(b'(') {
                    return None;
                }
                let mut v = vec![self.geometry()?];
                while self.eat(b',') {
                    v.push(self.geometry()?);
                }
                if !self.eat(b')') {
                    return None;
                }
                Some(Geometry::Collection(v))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point() {
        assert_eq!(
            parse_wkt("POINT (30 10)"),
            Some(Geometry::Point([30.0, 10.0]))
        );
        assert_eq!(
            parse_wkt("POINT(30 10)"),
            Some(Geometry::Point([30.0, 10.0]))
        );
    }

    #[test]
    fn point_z_is_dropped() {
        assert_eq!(
            parse_wkt("POINT Z (30 10 5)"),
            Some(Geometry::Point([30.0, 10.0]))
        );
    }

    #[test]
    fn linestring() {
        assert_eq!(
            parse_wkt("LINESTRING (30 10, 10 30, 40 40)"),
            Some(Geometry::LineString(vec![
                [30.0, 10.0],
                [10.0, 30.0],
                [40.0, 40.0]
            ]))
        );
    }

    #[test]
    fn polygon_with_hole() {
        let g = parse_wkt(
            "POLYGON ((35 10, 45 45, 15 40, 10 20, 35 10), (20 30, 35 35, 30 20, 20 30))",
        )
        .unwrap();
        match g {
            Geometry::Polygon(rings) => {
                assert_eq!(rings.len(), 2);
                assert_eq!(rings[0].len(), 5);
                assert_eq!(rings[1].len(), 4);
            }
            _ => panic!("expected polygon"),
        }
    }

    #[test]
    fn multipoint_both_forms() {
        let a = parse_wkt("MULTIPOINT (10 40, 40 30)").unwrap();
        let b = parse_wkt("MULTIPOINT ((10 40), (40 30))").unwrap();
        assert_eq!(a, b);
        assert_eq!(a, Geometry::MultiPoint(vec![[10.0, 40.0], [40.0, 30.0]]));
    }

    #[test]
    fn multipolygon() {
        let g = parse_wkt(
            "MULTIPOLYGON (((30 20, 45 40, 10 40, 30 20)), ((15 5, 40 10, 10 20, 5 10, 15 5)))",
        )
        .unwrap();
        match g {
            Geometry::MultiPolygon(polys) => assert_eq!(polys.len(), 2),
            _ => panic!("expected multipolygon"),
        }
    }

    #[test]
    fn bounds_over_polygon() {
        let g = parse_wkt("POLYGON ((0 0, 10 0, 10 5, 0 5, 0 0))").unwrap();
        assert_eq!(g.bounds(), Some((0.0, 0.0, 10.0, 5.0)));
    }

    #[test]
    fn projection_math() {
        // PlateCarree is the identity.
        assert_eq!(
            SfProjection::PlateCarree.project([10.0, 45.0]),
            [10.0, 45.0]
        );
        // Mercator: x = lon in radians; y = ln(tan(pi/4 + lat/2)).
        let [x, y] = SfProjection::Mercator.project([0.0, 45.0]);
        assert!((x - 0.0).abs() < 1e-12);
        assert!((y - 0.881_373).abs() < 1e-5, "mercator y at 45° = {y}");
        // The equator maps to ~0; higher latitudes stretch more than lower ones.
        assert!(SfProjection::Mercator.project([5.0, 0.0])[1].abs() < 1e-12);
        let y20 = SfProjection::Mercator.project([0.0, 20.0])[1];
        let y60_80 = SfProjection::Mercator.project([0.0, 80.0])[1]
            - SfProjection::Mercator.project([0.0, 60.0])[1];
        assert!(y60_80 > y20, "poleward bands stretch more");
    }

    #[test]
    fn empty_and_malformed() {
        assert_eq!(parse_wkt("POLYGON EMPTY"), Some(Geometry::Empty));
        assert_eq!(parse_wkt("GARBAGE"), None);
        assert_eq!(parse_wkt("POINT (30)"), None);
    }
}
