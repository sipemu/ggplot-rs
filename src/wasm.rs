//! Browser (WebAssembly) bindings — feature `wasm`.
//!
//! Exposes a plotters-free renderer to JavaScript: pass a JSON spec (geometry +
//! optional fill/label + options) and get back an SVG string with per-feature
//! `<title>` hover tooltips. Pair with DuckDB-Wasm (spatial extension) to read
//! shapefiles/GeoJSON in the browser and `ST_AsText(geom)` them into `geometry`.

use serde_json::Value as J;
use wasm_bindgen::prelude::*;

use crate::data::Value;
use crate::geom::sf::GeomSf;
use crate::prelude::*;
use crate::spatial::SfProjection;

/// Render a `geom_sf` map from a JSON spec, returning an SVG document.
///
/// Spec fields:
/// - `geometry`: `[String]` — WKT (required)
/// - `fill`: `[Number]` — choropleth values (optional)
/// - `label`: `[String]` — hover labels (optional)
/// - `fill_name`, `title`: `String` (optional)
/// - `width`, `height`: `Number` (default 800×600)
/// - `projection`: `"mercator"` | `"platecarree"` (default)
#[wasm_bindgen]
pub fn render_geo(spec_json: &str) -> Result<String, JsValue> {
    render_geo_impl(spec_json).map_err(|e| JsValue::from_str(&e))
}

fn render_geo_impl(spec_json: &str) -> Result<String, String> {
    let v: J = serde_json::from_str(spec_json).map_err(|e| format!("bad spec JSON: {e}"))?;

    let geom = v["geometry"]
        .as_array()
        .ok_or("spec.geometry must be an array of WKT strings")?;
    let geometry: Vec<Value> = geom
        .iter()
        .map(|g| Value::Str(g.as_str().unwrap_or_default().to_string()))
        .collect();
    let mut cols: Vec<(String, Vec<Value>)> = vec![("geometry".to_string(), geometry)];

    let mut aes = Aes::new();
    let has_fill = v.get("fill").and_then(|f| f.as_array()).is_some();
    if let Some(fill) = v.get("fill").and_then(|f| f.as_array()) {
        let col = fill
            .iter()
            .map(|x| x.as_f64().map(Value::Float).unwrap_or(Value::Na))
            .collect();
        cols.push(("__fill".to_string(), col));
        aes = aes.fill("__fill");
    }
    if let Some(label) = v.get("label").and_then(|l| l.as_array()) {
        let col = label
            .iter()
            .map(|x| Value::Str(x.as_str().unwrap_or_default().to_string()))
            .collect();
        cols.push(("__label".to_string(), col));
        aes = aes.label("__label");
    }

    let num = |k: &str, d: u32| v.get(k).and_then(|x| x.as_u64()).unwrap_or(d as u64) as u32;
    let (width, height) = (num("width", 800), num("height", 600));
    let projection = match v.get("projection").and_then(|x| x.as_str()) {
        Some("mercator") => SfProjection::Mercator,
        _ => SfProjection::PlateCarree,
    };

    let mut plot = GGPlot::new(cols)
        .aes(aes)
        .geom_sf_with(GeomSf::default().project(projection))
        .coord_sf()
        .theme(theme_minimal());
    if has_fill {
        plot = plot.scale_fill_viridis_c();
    }
    if let Some(t) = v.get("title").and_then(|x| x.as_str()) {
        plot = plot.title(t);
    }

    plot.render_svg_native_with_size(width, height)
        .map_err(|e| format!("render failed: {e:?}"))
}
