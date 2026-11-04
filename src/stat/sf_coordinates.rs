use crate::aes::Aesthetic;
use crate::data::{DataFrame, Value};
use crate::scale::ScaleSet;
use crate::spatial::{parse_wkt, Geometry, SfProjection};

use super::Stat;

/// One point per simple-features geometry (ggplot2's `stat_sf_coordinates`):
/// reads the WKT `geometry` column and emits its representative point as
/// `x` / `y` — the pole of inaccessibility of the largest polygon (always
/// inside, like `st_point_on_surface`), a line's middle vertex, a point
/// itself — keeping every other column, so any geom (`geom_text`,
/// `geom_point`, `geom_label_repel` …) can be placed on a map.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StatSfCoordinates {
    /// Project the point like the `geom_sf` layer it accompanies.
    pub projection: SfProjection,
}

impl StatSfCoordinates {
    pub fn project(mut self, projection: SfProjection) -> Self {
        self.projection = projection;
        self
    }
}

fn largest_polygon(g: &Geometry) -> Option<&Vec<Vec<[f64; 2]>>> {
    let area = |r: &Vec<[f64; 2]>| {
        let n = r.len();
        (0..n)
            .map(|i| {
                let (a, b) = (r[i], r[(i + 1) % n]);
                a[0] * b[1] - b[0] * a[1]
            })
            .sum::<f64>()
            .abs()
    };
    match g {
        Geometry::Polygon(p) => Some(p),
        Geometry::MultiPolygon(ps) => ps
            .iter()
            .filter(|p| !p.is_empty())
            .max_by(|a, b| area(&a[0]).total_cmp(&area(&b[0]))),
        Geometry::Collection(gs) => gs.iter().find_map(largest_polygon),
        _ => None,
    }
}

/// The representative point of a geometry (in its own coordinates).
pub(crate) fn representative_point(g: &Geometry) -> Option<[f64; 2]> {
    if let Some(rings) = largest_polygon(g) {
        let (x0, y0, x1, y1) = g.bounds()?;
        let precision = ((x1 - x0).max(y1 - y0) / 1000.0).max(1e-9);
        return crate::spatial::polylabel(rings, precision);
    }
    match g {
        Geometry::Point(p) => Some(*p),
        Geometry::LineString(l) | Geometry::MultiPoint(l) => l.get(l.len() / 2).copied(),
        Geometry::MultiLineString(ls) => ls
            .iter()
            .max_by_key(|l| l.len())
            .and_then(|l| l.get(l.len() / 2).copied()),
        Geometry::Collection(gs) => gs.iter().find_map(representative_point),
        _ => None,
    }
}

impl Stat for StatSfCoordinates {
    fn compute_group(&self, data: &DataFrame, _scales: &ScaleSet) -> DataFrame {
        let Some(geom) = data.column("geometry") else {
            return DataFrame::new();
        };
        let pts: Vec<Option<[f64; 2]>> = geom
            .iter()
            .map(|v| {
                let p = representative_point(&parse_wkt(v.as_str()?)?)?;
                Some(self.projection.project(p))
            })
            .collect();
        let keep: Vec<usize> = (0..pts.len()).filter(|&i| pts[i].is_some()).collect();
        let mut out = DataFrame::new();
        for name in data.column_names() {
            if name == "x" || name == "y" {
                continue;
            }
            let c = data.column(name).unwrap();
            out.add_column(
                name.to_string(),
                keep.iter().map(|&i| c[i].clone()).collect(),
            );
        }
        let coord = |k: usize| {
            keep.iter()
                .map(|&i| Value::Float(pts[i].unwrap()[k]))
                .collect()
        };
        out.add_column("x".into(), coord(0));
        out.add_column("y".into(), coord(1));
        out
    }

    fn required_aes(&self) -> Vec<Aesthetic> {
        Vec::new()
    }

    fn name(&self) -> &str {
        "sf_coordinates"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_inside_each_geometry() {
        let mut df = DataFrame::new();
        df.add_column(
            "geometry".into(),
            [
                "POLYGON ((0 0, 10 0, 10 10, 0 10, 0 0))",
                "POINT (3 4)",
                "LINESTRING (0 0, 1 1, 2 2)",
                "not wkt",
            ]
            .map(|s| Value::Str(s.into()))
            .to_vec(),
        );
        df.add_column(
            "name".into(),
            ["a", "b", "c", "d"].map(|s| Value::Str(s.into())).to_vec(),
        );
        let out = StatSfCoordinates::default().compute_group(&df, &ScaleSet::new());
        assert_eq!(out.nrows(), 3);
        let x: Vec<f64> = out
            .column("x")
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        let y: Vec<f64> = out
            .column("y")
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        assert!((x[0] - 5.0).abs() < 0.1 && (y[0] - 5.0).abs() < 0.1);
        assert_eq!((x[1], y[1]), (3.0, 4.0));
        assert_eq!((x[2], y[2]), (1.0, 1.0));
        assert_eq!(out.column("name").unwrap()[2].as_str(), Some("c"));
    }
}
