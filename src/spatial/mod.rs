//! Minimal simple-features geometry model and a self-contained WKT parser,
//! backing `geom_sf` (feature `sf`). Kept dependency-free so the spatial layer
//! adds no heavy `geo`/`proj`/`gdal` tree to the core.

#[cfg(feature = "geojson")]
pub mod geojson;

mod polylabel;
pub use polylabel::{polylabel, polylabel_with_distance};

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

/// The authalic Earth radius PROJ uses for `+R` spheres (metres) — the
/// sphere [`SfProjection::EqualEarth`] and [`SfProjection::Albers`] project
/// onto, matching `ST_Transform(..., '+R=6371008.8')` in DuckDB spatial.
const EARTH_RADIUS: f64 = 6_371_008.8;

/// A map projection applied to longitude/latitude before scaling.
///
/// [`SfProjection::EqualEarth`], [`SfProjection::Albers`] and
/// [`SfProjection::Utm`] are spherical/ellipsoidal projections good to
/// 1e-6 relative error against PROJ (see the `spatial::tests` module) —
/// ported from anofox-visualization's `map/project.rs`
/// (DataZooDE/anofox-visualization#318).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[non_exhaustive]
pub enum SfProjection {
    /// Longitude/latitude used directly (equirectangular / plate carrée).
    #[default]
    PlateCarree,
    /// Spherical Web Mercator (conformal; latitude clamped to ±85.05°).
    Mercator,
    /// Equal Earth (Šavrič, Jenny & Jenny 2018): an equal-area pseudocylindrical
    /// projection good for whole-world maps, centred on `lon0` (degrees).
    EqualEarth { lon0: f64 },
    /// Spherical Albers equal-area conic, with standard parallels `lat1`/`lat2`
    /// and origin (`lon0`, `lat0`), all in degrees — ggplot2/sf's `+proj=aea`.
    /// Good for a regional extent (a continent, a country); [`SfProjection::albers_usa`]
    /// and [`SfProjection::albers_europe`] give sensible presets.
    Albers {
        lat1: f64,
        lat2: f64,
        lat0: f64,
        lon0: f64,
    },
    /// Ellipsoidal (WGS84) transverse Mercator in a UTM zone (1–60), `south`
    /// adding the 10,000,000 m false northing used south of the equator.
    /// [`SfProjection::utm_for`] picks the zone/hemisphere from a point.
    Utm { zone: u8, south: bool },
}

impl SfProjection {
    /// Equal Earth centred on `lon0` (degrees) — the usual choice for a
    /// whole-world map is `lon0 = 0.0`.
    pub fn equal_earth(lon0: f64) -> Self {
        SfProjection::EqualEarth { lon0 }
    }

    /// Spherical Albers equal-area conic with explicit standard parallels
    /// `lat1`/`lat2` and origin (`lon0`, `lat0`), degrees.
    pub fn albers(lat1: f64, lat2: f64, lat0: f64, lon0: f64) -> Self {
        SfProjection::Albers {
            lat1,
            lat2,
            lat0,
            lon0,
        }
    }

    /// A conterminous-USA Albers preset (the USGS parallels 29.5°/45.5°,
    /// origin 23°N 96°W — the one `ggplot2::coord_sf` / `albersusa` use).
    pub fn albers_usa() -> Self {
        SfProjection::Albers {
            lat1: 29.5,
            lat2: 45.5,
            lat0: 23.0,
            lon0: -96.0,
        }
    }

    /// A Europe Albers preset (parallels 43°/62°, origin 0°N 10°E).
    pub fn albers_europe() -> Self {
        SfProjection::Albers {
            lat1: 43.0,
            lat2: 62.0,
            lat0: 0.0,
            lon0: 10.0,
        }
    }

    /// UTM in an explicit zone (1–60; out-of-range values are clamped),
    /// `south` for the southern-hemisphere false northing.
    pub fn utm(zone: u8, south: bool) -> Self {
        SfProjection::Utm {
            zone: zone.clamp(1, 60),
            south,
        }
    }

    /// The UTM zone containing longitude `lon` (degrees), 1–60.
    pub fn utm_zone(lon: f64) -> u8 {
        (((lon + 180.0) / 6.0).floor() as i64).clamp(0, 59) as u8 + 1
    }

    /// UTM set up for a point: the zone containing `lon`, southern false
    /// northing if `lat < 0`.
    pub fn utm_for(lon: f64, lat: f64) -> Self {
        SfProjection::Utm {
            zone: Self::utm_zone(lon),
            south: lat < 0.0,
        }
    }

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
            SfProjection::EqualEarth { lon0 } => equal_earth(lon - lon0, lat.clamp(-90.0, 90.0)),
            SfProjection::Albers {
                lat1,
                lat2,
                lat0,
                lon0,
            } => albers(*lat1, *lat2, *lat0, *lon0, lon, lat.clamp(-90.0, 90.0)),
            SfProjection::Utm { zone, south } => {
                let lon0 = (*zone as f64) * 6.0 - 183.0;
                let [e, n] = utm(lon, lat.clamp(-89.9, 89.9), lon0);
                [e, if *south { n + 10_000_000.0 } else { n }]
            }
        }
    }
}

/// Equal Earth on the [`EARTH_RADIUS`] sphere, `lon` relative to the central
/// meridian (both in degrees).
fn equal_earth(lon: f64, lat: f64) -> Coord {
    const A1: f64 = 1.340264;
    const A2: f64 = -0.081106;
    const A3: f64 = 0.000893;
    const A4: f64 = 0.003796;
    let m = 3f64.sqrt() / 2.0;
    let (l, p) = (lon.to_radians(), lat.to_radians());
    let t = (m * p.sin()).asin();
    let (t2, t6) = (t * t, t.powi(6));
    let x = l * t.cos() / (m * (A1 + 3.0 * A2 * t2 + t6 * (7.0 * A3 + 9.0 * A4 * t2)));
    let y = t * (A1 + A2 * t2 + t6 * (A3 + A4 * t2));
    [EARTH_RADIUS * x, EARTH_RADIUS * y]
}

/// Spherical Albers equal-area conic (degrees in, metres out) — standard
/// parallels `lat1`/`lat2`, origin (`lon0`, `lat0`).
fn albers(lat1: f64, lat2: f64, lat0: f64, lon0: f64, lon: f64, lat: f64) -> Coord {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let n = (p1.sin() + p2.sin()) / 2.0;
    if n.abs() < 1e-10 {
        // Parallels straddling the equator: the cone flattens to a cylinder.
        // Equal Earth degrades gracefully there and keeps `project` infallible.
        return equal_earth(lon - lon0, lat);
    }
    let c = p1.cos().powi(2) + 2.0 * n * p1.sin();
    let rho0 = EARTH_RADIUS * (c - 2.0 * n * lat0.to_radians().sin()).max(0.0).sqrt() / n;
    let rho = EARTH_RADIUS * (c - 2.0 * n * lat.to_radians().sin()).max(0.0).sqrt() / n;
    let theta = n * (lon - lon0).to_radians();
    [rho * theta.sin(), rho0 - rho * theta.cos()]
}

/// Ellipsoidal (WGS84) transverse Mercator (Krüger series to n⁶, as in
/// Karney 2011), UTM scale and false easting; northing from the equator (no
/// south false northing — [`SfProjection::project`] adds that).
fn utm(lon: f64, lat: f64, lon0: f64) -> Coord {
    const A: f64 = 6_378_137.0;
    const F: f64 = 1.0 / 298.257_223_563;
    const K0: f64 = 0.9996;
    let n = F / (2.0 - F);
    let (n2, n3) = (n * n, n * n * n);
    let (n4, n5, n6) = (n2 * n2, n2 * n3, n3 * n3);
    let a_r = A / (1.0 + n) * (1.0 + n2 / 4.0 + n4 / 64.0 + n6 / 256.0);
    let alpha = [
        n / 2.0 - 2.0 * n2 / 3.0 + 5.0 * n3 / 16.0 + 41.0 * n4 / 180.0 - 127.0 * n5 / 288.0
            + 7891.0 * n6 / 37800.0,
        13.0 * n2 / 48.0 - 3.0 * n3 / 5.0 + 557.0 * n4 / 1440.0 + 281.0 * n5 / 630.0
            - 1983433.0 * n6 / 1935360.0,
        61.0 * n3 / 240.0 - 103.0 * n4 / 140.0 + 15061.0 * n5 / 26880.0 + 167603.0 * n6 / 181440.0,
        49561.0 * n4 / 161280.0 - 179.0 * n5 / 168.0 + 6601661.0 * n6 / 7257600.0,
        34729.0 * n5 / 80640.0 - 3418889.0 * n6 / 1995840.0,
        212378941.0 * n6 / 319334400.0,
    ];
    let p = lat.to_radians();
    let dl = (lon - lon0).to_radians();
    let e = 2.0 * n.sqrt() / (1.0 + n);
    let t = (p.sin().atanh() - e * (e * p.sin()).atanh()).sinh();
    let xi1 = t.atan2(dl.cos());
    let eta1 = (dl.sin() / (1.0 + t * t).sqrt()).atanh();
    let (mut xi, mut eta) = (xi1, eta1);
    for (j, a) in alpha.iter().enumerate() {
        let k = 2.0 * (j + 1) as f64;
        xi += a * (k * xi1).sin() * (k * eta1).cosh();
        eta += a * (k * xi1).cos() * (k * eta1).sinh();
    }
    [500_000.0 + K0 * a_r * eta, K0 * a_r * xi]
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

    fn close(a: Coord, b: Coord, tol: f64) {
        assert!(
            (a[0] - b[0]).abs() < tol && (a[1] - b[1]).abs() < tol,
            "{a:?} vs {b:?}"
        );
    }

    // Reference values: DuckDB spatial (PROJ 9) ST_Transform from EPSG:4326
    // with always_xy, e.g.
    //   ST_Transform(ST_Point(13.4, 52.5), 'EPSG:4326', '+proj=eqearth +R=6371008.8', always_xy := true)
    // (matching anofox-visualization's map/project.rs, which these are
    // ported from — DataZooDE/anofox-visualization#318).
    #[test]
    fn equal_earth_matches_proj() {
        let p = SfProjection::equal_earth(0.0);
        close(
            p.project([13.4, 52.5]),
            [1_036_974.059_041_676_3, 6_245_911.951_883_592],
            0.01,
        );
        close(
            p.project([-120.0, -33.0]),
            [-10_599_506.921_602_746, -4_132_993.094_803_968_4],
            0.01,
        );
    }

    #[test]
    fn albers_europe_matches_proj() {
        // +proj=aea +lat_1=43 +lat_2=62 +lat_0=0 +lon_0=10 +R=6371008.8
        let p = SfProjection::albers_europe();
        close(
            p.project([13.4, 52.5]),
            [226_949.661_171_036_07, 5_421_888.063_620_956],
            0.01,
        );
        close(
            p.project([-9.1, 38.7]),
            [-1_658_326.311_163_812_6, 4_093_362.143_505_720_4],
            0.01,
        );
    }

    #[test]
    fn utm_matches_proj() {
        // EPSG:32633 (zone 33N, central meridian 15°)
        let p = SfProjection::utm_for(13.4, 52.5);
        assert_eq!(
            p,
            SfProjection::Utm {
                zone: 33,
                south: false
            }
        );
        close(
            p.project([13.4, 52.5]),
            [391_390.731_339_952_04, 5_817_855.240_817_33],
            0.01,
        );
        // EPSG:32632 (zone 32N): Munich
        close(
            SfProjection::utm(32, false).project([11.575, 48.137]),
            [691_567.326_384_146_5, 5_334_734.330_481_76],
            0.01,
        );
        // EPSG:32733 (zone 33S): a southern-hemisphere point gets the
        // 10,000,000 m false northing.
        // ST_Transform(ST_Point(27.0, -29.0), 'EPSG:4326', '+proj=utm +zone=33 +south +ellps=WGS84', always_xy := true)
        close(
            SfProjection::utm(33, true).project([27.0, -29.0]),
            [1_673_355.340_918_114_6, 6_731_880.836_950_015],
            0.01,
        );
    }

    #[test]
    fn utm_zone_picks_the_6_degree_band() {
        assert_eq!(SfProjection::utm_zone(-180.0), 1);
        assert_eq!(SfProjection::utm_zone(15.0), 33);
        assert_eq!(SfProjection::utm_zone(179.999), 60);
        assert!(SfProjection::utm_for(15.0, -1.0).project([15.0, -1.0])[1] > 9_000_000.0);
    }

    #[test]
    fn utm_zone_out_of_range_clamps_instead_of_panicking() {
        assert_eq!(SfProjection::utm(0, false), SfProjection::utm(1, false));
        assert_eq!(SfProjection::utm(200, false), SfProjection::utm(60, false));
    }

    #[test]
    fn albers_usa_matches_proj() {
        // +proj=aea +lat_1=29.5 +lat_2=45.5 +lat_0=23 +lon_0=-96 +R=6371008.8
        let p = SfProjection::albers_usa();
        close(
            p.project([-98.5, 39.5]),
            [-212_471.293_229_921_78, 1_838_982.778_857_062_8],
            0.01,
        );
        close(
            p.project([-122.4, 37.8]),
            [-2_267_548.949_559_569_4, 1_962_287.268_585_857_5],
            0.01,
        );
    }

    #[test]
    fn albers_zero_n_falls_back_to_equal_earth() {
        // Standard parallels straddling the equator collapse the cone (n≈0);
        // `project` stays total rather than dividing by ~0.
        let p = SfProjection::albers(-1.0, 1.0, 0.0, 0.0);
        let out = p.project([13.4, 52.5]);
        assert!(out[0].is_finite() && out[1].is_finite());
        assert_eq!(out, SfProjection::equal_earth(0.0).project([13.4, 52.5]));
    }

    #[test]
    fn default_is_plate_carree() {
        assert_eq!(SfProjection::default(), SfProjection::PlateCarree);
        assert_eq!(SfProjection::default().project([13.4, 52.5]), [13.4, 52.5]);
    }
}
