//! Read GeoJSON into a plot-ready frame (feature `geojson`).
//!
//! Each feature becomes a row: its geometry is converted to WKT in a `geometry`
//! column (consumed by `geom_sf`), and its `properties` become data columns.

use serde_json::Value as J;

use crate::data::Value;

/// Parse a GeoJSON `FeatureCollection` (or a single `Feature`) into columns
/// ready for `GGPlot::new` + `geom_sf`: a `geometry` column of WKT plus one
/// column per property key (missing values become `Value::Na`).
pub fn read_geojson_str(input: &str) -> Result<Vec<(String, Vec<Value>)>, String> {
    let root: J = serde_json::from_str(input).map_err(|e| format!("invalid JSON: {e}"))?;
    let features: Vec<&J> = match root.get("type").and_then(|t| t.as_str()) {
        Some("FeatureCollection") => root
            .get("features")
            .and_then(|f| f.as_array())
            .map(|a| a.iter().collect())
            .ok_or("FeatureCollection missing `features` array")?,
        Some("Feature") => vec![&root],
        _ => return Err("expected a GeoJSON FeatureCollection or Feature".into()),
    };

    let mut geom = Vec::with_capacity(features.len());
    let mut keys: Vec<String> = Vec::new();
    let mut props: Vec<serde_json::Map<String, J>> = Vec::with_capacity(features.len());
    for f in features {
        let wkt = f.get("geometry").and_then(geom_to_wkt).unwrap_or_default();
        geom.push(Value::Str(wkt));
        let p = f
            .get("properties")
            .and_then(|p| p.as_object())
            .cloned()
            .unwrap_or_default();
        for k in p.keys() {
            if !keys.contains(k) {
                keys.push(k.clone());
            }
        }
        props.push(p);
    }

    let mut cols: Vec<(String, Vec<Value>)> = vec![("geometry".to_string(), geom)];
    for k in &keys {
        let col = props
            .iter()
            .map(|p| p.get(k).map(json_to_value).unwrap_or(Value::Na))
            .collect();
        cols.push((k.clone(), col));
    }
    Ok(cols)
}

/// [`read_geojson_str`] from a file path.
pub fn read_geojson_file(path: &str) -> Result<Vec<(String, Vec<Value>)>, String> {
    let s = std::fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))?;
    read_geojson_str(&s)
}

fn json_to_value(j: &J) -> Value {
    match j {
        J::String(s) => Value::Str(s.clone()),
        J::Number(n) => n.as_f64().map(Value::Float).unwrap_or(Value::Na),
        J::Bool(b) => Value::Bool(*b),
        _ => Value::Na,
    }
}

// ── GeoJSON geometry → WKT ────────────────────────────────────────────────

fn pt(a: &J) -> Option<String> {
    let a = a.as_array()?;
    Some(format!("{} {}", a.first()?.as_f64()?, a.get(1)?.as_f64()?))
}

/// `[[x,y], ...]` → `x y, x y`
fn line(a: &J) -> Option<String> {
    let parts: Vec<String> = a.as_array()?.iter().filter_map(pt).collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

/// `[[[x,y], ...], ...]` → `(x y, ...),(x y, ...)`
fn rings(a: &J) -> Option<String> {
    let parts: Vec<String> = a
        .as_array()?
        .iter()
        .filter_map(|r| Some(format!("({})", line(r)?)))
        .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

/// `[[[[x,y]...]...]...]` → `((...)),((...),(...))`
fn polys(a: &J) -> Option<String> {
    let parts: Vec<String> = a
        .as_array()?
        .iter()
        .filter_map(|p| Some(format!("({})", rings(p)?)))
        .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

fn geom_to_wkt(g: &J) -> Option<String> {
    let t = g.get("type")?.as_str()?;
    let c = g.get("coordinates")?;
    Some(match t {
        "Point" => format!("POINT ({})", pt(c)?),
        "LineString" => format!("LINESTRING ({})", line(c)?),
        "Polygon" => format!("POLYGON ({})", rings(c)?),
        "MultiPoint" => format!("MULTIPOINT ({})", line(c)?),
        "MultiLineString" => format!("MULTILINESTRING ({})", rings(c)?),
        "MultiPolygon" => format!("MULTIPOLYGON ({})", polys(c)?),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spatial::parse_wkt;

    const FC: &str = r#"{
      "type": "FeatureCollection",
      "features": [
        { "type": "Feature",
          "properties": { "name": "A", "pop": 12.5 },
          "geometry": { "type": "Polygon", "coordinates": [[[0,0],[2,0],[2,2],[0,2],[0,0]]] } },
        { "type": "Feature",
          "properties": { "name": "B" },
          "geometry": { "type": "Point", "coordinates": [5, 5] } }
      ]
    }"#;

    #[test]
    fn reads_features_and_properties() {
        let cols = read_geojson_str(FC).unwrap();
        let names: Vec<&str> = cols.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"geometry") && names.contains(&"name") && names.contains(&"pop"));

        let geom = &cols.iter().find(|(n, _)| n == "geometry").unwrap().1;
        assert_eq!(geom.len(), 2);
        // Emitted WKT round-trips through the parser.
        match &geom[0] {
            Value::Str(s) => {
                assert!(s.starts_with("POLYGON"), "{s}");
                assert!(parse_wkt(s).is_some());
            }
            _ => panic!("geometry should be WKT string"),
        }
        // Missing property becomes Na.
        let pop = &cols.iter().find(|(n, _)| n == "pop").unwrap().1;
        assert_eq!(pop[0].as_f64(), Some(12.5));
        assert!(matches!(pop[1], Value::Na));
    }

    #[test]
    fn geojson_renders_through_geom_sf() {
        use crate::prelude::*;
        let cols = read_geojson_str(FC).unwrap();
        let svg = GGPlot::new(cols)
            .aes(Aes::new().fill("pop"))
            .geom_sf()
            .render_svg()
            .expect("render geojson");
        assert!(svg.contains("<polygon") || svg.contains("<circle") || svg.contains("<path"));
    }

    #[test]
    fn multipolygon_and_errors() {
        let mp = r#"{"type":"Feature","properties":{},"geometry":
            {"type":"MultiPolygon","coordinates":[[[[0,0],[1,0],[1,1],[0,0]]],[[[2,2],[3,2],[3,3],[2,2]]]]}}"#;
        let cols = read_geojson_str(mp).unwrap();
        if let Value::Str(s) = &cols[0].1[0] {
            assert!(s.starts_with("MULTIPOLYGON"), "{s}");
            assert!(parse_wkt(s).is_some());
        }
        assert!(read_geojson_str("{ not json").is_err());
        assert!(read_geojson_str(r#"{"type":"Point"}"#).is_err());
    }
}
