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

/// Render a bar chart (SVG, hover tooltips) from `{ category: [String],
/// value: [Number], width?, height?, title? }`. Bars are coloured by category
/// with the Set1 palette — matching the scatter's groups for linked views.
#[wasm_bindgen]
pub fn render_bar(spec_json: &str) -> Result<String, JsValue> {
    render_bar_impl(spec_json).map_err(|e| JsValue::from_str(&e))
}

fn render_bar_impl(spec_json: &str) -> Result<String, String> {
    let v: J = serde_json::from_str(spec_json).map_err(|e| format!("bad spec JSON: {e}"))?;
    let cat = v["category"]
        .as_array()
        .ok_or("spec.category must be an array")?;
    let val = v["value"].as_array().ok_or("spec.value must be an array")?;
    let cats: Vec<Value> = cat
        .iter()
        .map(|s| Value::Str(s.as_str().unwrap_or_default().to_string()))
        .collect();
    let vals: Vec<Value> = val
        .iter()
        .map(|x| x.as_f64().map(Value::Float).unwrap_or(Value::Na))
        .collect();
    let cols = vec![
        ("x".to_string(), cats.clone()),
        ("y".to_string(), vals),
        ("fill".to_string(), cats.clone()),
        ("label".to_string(), cats),
    ];
    let num = |k: &str, d: u32| v.get(k).and_then(|x| x.as_u64()).unwrap_or(d as u64) as u32;
    let (width, height) = (num("width", 480), num("height", 300));

    let mut plot = GGPlot::new(cols)
        .aes(Aes::new().x("x").y("y").fill("fill").label("label"))
        .geom_col()
        .scale_fill_brewer(crate::scale::palettes::PaletteName::Set1)
        .theme_minimal();
    if let Some(t) = v.get("title").and_then(|x| x.as_str()) {
        plot = plot.title(t);
    }
    plot.render_svg_native_with_size(width, height)
        .map_err(|e| format!("render failed: {e:?}"))
}

/// Render a large scatter to a raw RGBA buffer via the raster backend — for
/// point counts where SVG's one-node-per-mark would choke. Wrap the result in
/// `new ImageData(new Uint8ClampedArray(buf), width, height)` and `putImageData`
/// onto a `<canvas>`. (Feature `canvas`.)
///
/// Spec: `{ x: [Number], y: [Number], color?: [String], width?, height?, title? }`.
#[cfg(feature = "canvas")]
#[wasm_bindgen]
pub fn render_scatter_rgba(spec_json: &str) -> Result<Vec<u8>, JsValue> {
    render_scatter_impl(spec_json).map_err(|e| JsValue::from_str(&e))
}

#[cfg(feature = "canvas")]
fn render_scatter_impl(spec_json: &str) -> Result<Vec<u8>, String> {
    let v: J = serde_json::from_str(spec_json).map_err(|e| format!("bad spec JSON: {e}"))?;
    let nums = |key: &str| -> Result<Vec<Value>, String> {
        Ok(v[key]
            .as_array()
            .ok_or_else(|| format!("spec.{key} must be an array of numbers"))?
            .iter()
            .map(|x| x.as_f64().map(Value::Float).unwrap_or(Value::Na))
            .collect())
    };
    let mut cols: Vec<(String, Vec<Value>)> =
        vec![("x".to_string(), nums("x")?), ("y".to_string(), nums("y")?)];
    let mut aes = Aes::new().x("x").y("y");
    let has_color = v.get("color").and_then(|c| c.as_array()).is_some();
    if let Some(c) = v.get("color").and_then(|c| c.as_array()) {
        let col = c
            .iter()
            .map(|s| Value::Str(s.as_str().unwrap_or_default().to_string()))
            .collect();
        cols.push(("color".to_string(), col));
        aes = aes.color("color");
    }
    let num = |k: &str, d: u32| v.get(k).and_then(|x| x.as_u64()).unwrap_or(d as u64) as u32;
    let (width, height) = (num("width", 800), num("height", 600));

    let mut plot = GGPlot::new(cols)
        .aes(aes)
        .geom_point()
        .theme(theme_minimal());
    if has_color {
        plot = plot.scale_color_brewer(crate::scale::palettes::PaletteName::Set1);
    }
    if let Some(t) = v.get("title").and_then(|x| x.as_str()) {
        plot = plot.title(t);
    }
    let (_, _, rgba) = plot
        .render_rgba_with_size(width, height)
        .map_err(|e| format!("render failed: {e:?}"))?;
    Ok(rgba)
}

/// A rendered scatter plus the pixel↔data mapping needed for interactive hover.
#[cfg(feature = "canvas")]
#[wasm_bindgen]
pub struct Scatter {
    rgba: Vec<u8>,
    plot: Vec<f64>,
    xdom: Vec<f64>,
    ydom: Vec<f64>,
}

#[cfg(feature = "canvas")]
#[wasm_bindgen]
impl Scatter {
    /// The RGBA buffer for `ctx.putImageData`.
    #[wasm_bindgen(getter)]
    pub fn rgba(&self) -> Vec<u8> {
        self.rgba.clone()
    }
    /// Panel rect in pixels: `[x, y, width, height]`.
    #[wasm_bindgen(getter)]
    pub fn plot(&self) -> Vec<f64> {
        self.plot.clone()
    }
    /// Expanded x data range `[min, max]` (maps to the panel's left/right edge).
    #[wasm_bindgen(getter)]
    pub fn xdom(&self) -> Vec<f64> {
        self.xdom.clone()
    }
    /// Expanded y data range `[min, max]` (maps to the panel's bottom/top edge).
    #[wasm_bindgen(getter)]
    pub fn ydom(&self) -> Vec<f64> {
        self.ydom.clone()
    }
}

/// Render a large scatter from typed arrays (no JSON round-trip) to the raster
/// backend, returning the pixels **and** the pixel↔data mapping so JS can do
/// nearest-point hover. Points are coloured by `group_idx` (a compact per-point
/// index, `0..group_names.len()`) — pass an empty `group_names` for no colour.
///
/// `selected` (length `x.len()` or empty) dims the plot: points with a non-zero
/// flag keep full opacity, the rest fade — for highlighting a brushed subset.
#[cfg(feature = "canvas")]
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn render_scatter_xy(
    x: &[f64],
    y: &[f64],
    group_idx: &[u32],
    group_names: Vec<String>,
    selected: &[u8],
    width: u32,
    height: u32,
    title: String,
) -> Result<Scatter, JsValue> {
    render_scatter_xy_impl(
        x,
        y,
        group_idx,
        group_names,
        selected,
        width,
        height,
        &title,
    )
    .map_err(|e| JsValue::from_str(&e))
}

#[cfg(feature = "canvas")]
#[allow(clippy::too_many_arguments)]
fn render_scatter_xy_impl(
    x: &[f64],
    y: &[f64],
    group_idx: &[u32],
    group_names: Vec<String>,
    selected: &[u8],
    width: u32,
    height: u32,
    title: &str,
) -> Result<Scatter, String> {
    let n = x.len().min(y.len());
    if n == 0 {
        return Err("empty x/y arrays".into());
    }
    // Default ggplot continuous expansion (5% each side) → the data value at the
    // panel edges, for inverting pixels back to data in JS.
    let expand = |v: &[f64]| {
        let mn = v.iter().copied().fold(f64::INFINITY, f64::min);
        let mx = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let pad = if (mx - mn).abs() < 1e-12 {
            1.0
        } else {
            (mx - mn) * 0.05
        };
        (mn - pad, mx + pad)
    };
    let (xe0, xe1) = expand(&x[..n]);
    let (ye0, ye1) = expand(&y[..n]);

    let mut cols: Vec<(String, Vec<Value>)> = vec![
        (
            "x".to_string(),
            x[..n].iter().map(|v| Value::Float(*v)).collect(),
        ),
        (
            "y".to_string(),
            y[..n].iter().map(|v| Value::Float(*v)).collect(),
        ),
    ];
    let mut aes = Aes::new().x("x").y("y");
    let has_group = !group_names.is_empty() && group_idx.len() >= n;
    if has_group {
        let col = group_idx[..n]
            .iter()
            .map(|&i| Value::Str(group_names.get(i as usize).cloned().unwrap_or_default()))
            .collect();
        cols.push(("g".to_string(), col));
        aes = aes.color("g");
    }
    // An unmapped `alpha` column is read per-point by geom_point (raw value),
    // so a brushed subset stays opaque while the rest fades.
    if selected.len() >= n {
        let alpha = (0..n)
            .map(|i| Value::Float(if selected[i] != 0 { 1.0 } else { 0.10 }))
            .collect();
        cols.push(("alpha".to_string(), alpha));
    }
    let mut plot = GGPlot::new(cols)
        .aes(aes)
        .geom_point()
        .theme(theme_minimal());
    if has_group {
        plot = plot.scale_color_brewer(crate::scale::palettes::PaletteName::Set1);
    }
    if !title.is_empty() {
        plot = plot.title(title);
    }
    let (rgba, plot_area) = plot
        .render_rgba_area_with_size(width, height)
        .map_err(|e| format!("render failed: {e:?}"))?;
    Ok(Scatter {
        rgba,
        plot: plot_area.to_vec(),
        xdom: vec![xe0, xe1],
        ydom: vec![ye0, ye1],
    })
}
