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
    // Optional zoom window `[lo, hi]` — for pan/zoom (roam). When present the
    // map clips to that window (updating axes) instead of the equal-aspect fit.
    let lim = |k: &str| -> Option<(f64, f64)> {
        match v.get(k)?.as_array()?.as_slice() {
            [lo, hi] => Some((lo.as_f64()?, hi.as_f64()?)),
            _ => None,
        }
    };
    let (xlim, ylim) = (lim("xlim"), lim("ylim"));

    let mut builder = GGPlot::new(cols).aes(aes);
    // Optional gray basemap (e.g. country outlines) drawn behind the geometry
    // for geographic context — a separate no-fill layer sharing the scales.
    if let Some(base) = v.get("base").and_then(|b| b.as_array()) {
        let base_geo: Vec<Value> = base
            .iter()
            .map(|g| Value::Str(g.as_str().unwrap_or_default().to_string()))
            .collect();
        let mut base_geom = GeomSf::default().project(projection);
        base_geom.fill = (228, 230, 233);
        base_geom.color = (198, 201, 206);
        builder = builder
            .geom_sf_with(base_geom)
            .layer_data(vec![("geometry".to_string(), base_geo)])
            .layer_aes(Aes::new());
    }
    let mut plot = builder.geom_sf_with(GeomSf::default().project(projection));
    plot = match (xlim, ylim) {
        (Some(xl), Some(yl)) => plot.coord_cartesian_zoom(Some(xl), Some(yl)),
        _ => plot.coord_sf(),
    };
    plot = plot.theme(theme_minimal());
    if has_fill {
        plot = plot.scale_fill_viridis_c();
    }
    if let Some(t) = v.get("title").and_then(|x| x.as_str()) {
        plot = plot.title(t);
    }

    plot.render_svg_native_with_size(width, height)
        .map_err(|e| format!("render failed: {e:?}"))
}

/// Bounding box `[minx, miny, maxx, maxy]` of a spec's `geometry` (WKT array) —
/// so JS can initialise a pan/zoom (roam) window and zoom around the cursor.
#[wasm_bindgen]
pub fn geo_bounds(spec_json: &str) -> Result<Vec<f64>, JsValue> {
    let v: J = serde_json::from_str(spec_json).map_err(|e| JsValue::from_str(&format!("{e}")))?;
    let geom = v["geometry"]
        .as_array()
        .ok_or_else(|| JsValue::from_str("spec.geometry must be an array of WKT strings"))?;
    let (mut x0, mut y0, mut x1, mut y1) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for g in geom {
        if let Some(b) = g
            .as_str()
            .and_then(crate::spatial::parse_wkt)
            .and_then(|g| g.bounds())
        {
            x0 = x0.min(b.0);
            y0 = y0.min(b.1);
            x1 = x1.max(b.2);
            y1 = y1.max(b.3);
        }
    }
    if !x0.is_finite() {
        return Err(JsValue::from_str("no valid geometry"));
    }
    Ok(vec![x0, y0, x1, y1])
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

/// Render a histogram (SVG) from a numeric array; `bins` sets the bin count —
/// drive it from a slider for an interactive histogram. Feature `wasm`.
#[wasm_bindgen]
pub fn render_hist(
    values: &[f64],
    bins: u32,
    width: u32,
    height: u32,
    title: String,
) -> Result<String, JsValue> {
    render_hist_impl(values, bins, width, height, &title).map_err(|e| JsValue::from_str(&e))
}

fn render_hist_impl(
    values: &[f64],
    bins: u32,
    width: u32,
    height: u32,
    title: &str,
) -> Result<String, String> {
    if values.is_empty() {
        return Err("no values to histogram".into());
    }
    let cols = vec![(
        "x".to_string(),
        values.iter().map(|&v| Value::Float(v)).collect(),
    )];
    let geom = crate::geom::histogram::GeomHistogram::default().with_bins(bins.max(1) as usize);
    let mut plot = GGPlot::new(cols)
        .aes(Aes::new().x("x"))
        .geom_histogram_with(geom)
        .theme_minimal();
    if !title.is_empty() {
        plot = plot.title(title);
    }
    plot.render_svg_native_with_size(width, height)
        .map_err(|e| format!("render failed: {e:?}"))
}

fn jval(x: &J) -> Value {
    match x {
        J::Number(n) => n.as_f64().map(Value::Float).unwrap_or(Value::Na),
        J::String(s) => Value::Str(s.clone()),
        J::Bool(b) => Value::Bool(*b),
        _ => Value::Na,
    }
}

fn brewer(name: &str) -> crate::scale::palettes::PaletteName {
    use crate::scale::palettes::PaletteName as P;
    match name {
        "Set2" => P::Set2,
        "Set3" => P::Set3,
        "Dark2" => P::Dark2,
        "Paired" => P::Paired,
        "Accent" => P::Accent,
        "Pastel1" => P::Pastel1,
        "Spectral" => P::Spectral,
        _ => P::Set1,
    }
}

fn smooth_geom(v: &J) -> crate::geom::smooth::GeomSmooth {
    let g = crate::geom::smooth::GeomSmooth::default();
    match v.get("method").and_then(|x| x.as_str()) {
        Some("loess") => g.loess(v.get("span").and_then(|x| x.as_f64()).unwrap_or(0.75)),
        _ => g,
    }
}

/// General grammar-of-graphics renderer — expose most geoms to JS via a spec:
/// `{ data: {col:[...]}, geom, aes:{x,y,color,fill,group,size}, title, width,
/// height, flip, log_x, log_y, palette, bins, smooth:"lm"|"loess" }`. Returns an
/// SVG document (with hover tooltips where the geom emits them).
#[wasm_bindgen]
pub fn render_plot(spec_json: &str) -> Result<String, JsValue> {
    render_plot_impl(spec_json).map_err(|e| JsValue::from_str(&e))
}

fn render_plot_impl(spec_json: &str) -> Result<String, String> {
    let v: J = serde_json::from_str(spec_json).map_err(|e| format!("bad spec JSON: {e}"))?;
    let data = v
        .get("data")
        .and_then(|d| d.as_object())
        .ok_or("spec.data must be an object of columns")?;
    let mut cols: Vec<(String, Vec<Value>)> = Vec::new();
    let mut is_str: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    for (k, arr) in data {
        let vals: Vec<Value> = arr
            .as_array()
            .map(|a| a.iter().map(jval).collect())
            .unwrap_or_default();
        let s = vals
            .iter()
            .find(|x| !matches!(x, Value::Na))
            .map(|x| matches!(x, Value::Str(_)))
            .unwrap_or(false);
        is_str.insert(k.clone(), s);
        cols.push((k.clone(), vals));
    }

    let aes_o = v.get("aes").and_then(|a| a.as_object());
    let get = |n: &str| aes_o.and_then(|o| o.get(n)).and_then(|x| x.as_str());
    let mut aes = Aes::new();
    for (name, set) in [
        ("x", 0),
        ("y", 1),
        ("color", 2),
        ("fill", 3),
        ("group", 4),
        ("size", 5),
    ] {
        if let Some(c) = get(name) {
            aes = match set {
                0 => aes.x(c),
                1 => aes.y(c),
                2 => aes.color(c),
                3 => aes.fill(c),
                4 => aes.group(c),
                _ => aes.size(c),
            };
        }
    }
    let color_col = get("color").map(String::from);
    let fill_col = get("fill").map(String::from);

    let num = |k: &str, d: u32| v.get(k).and_then(|x| x.as_u64()).unwrap_or(d as u64) as u32;
    let (width, height) = (num("width", 640), num("height", 400));
    let geom = v.get("geom").and_then(|x| x.as_str()).unwrap_or("point");
    let bins = v.get("bins").and_then(|x| x.as_u64()).unwrap_or(30) as usize;

    // Informative hover labels for point-like geoms: "<xname>: <xval>, …" (+group),
    // read by geom_point as its tooltip.
    if matches!(geom, "point" | "jitter") && get("label").is_none() {
        if let (Some(xn), Some(yn)) = (get("x"), get("y")) {
            let find = |n: &str| cols.iter().find(|(k, _)| k == n).map(|(_, v)| v.clone());
            if let (Some(xc), Some(yc)) = (find(xn), find(yn)) {
                let cn = get("color");
                let cc = cn.and_then(find);
                let f = crate::geom::tip_value;
                let label: Vec<Value> = (0..xc.len().min(yc.len()))
                    .map(|i| {
                        let base = format!("{xn}: {}, {yn}: {}", f(&xc[i]), f(&yc[i]));
                        match (cn, &cc) {
                            (Some(_), Some(cc)) => Value::Str(format!("{} — {base}", f(&cc[i]))),
                            _ => Value::Str(base),
                        }
                    })
                    .collect();
                cols.push(("label".to_string(), label));
            }
        }
    }

    let mut plot = GGPlot::new(cols).aes(aes);
    plot = match geom {
        "jitter" => plot.geom_jitter(),
        "line" => plot.geom_line(),
        "path" => plot.geom_path(),
        "step" => plot.geom_step(),
        "area" => plot.geom_area(),
        "col" => plot.geom_col(),
        "bar" => plot.geom_bar(),
        "boxplot" => plot.geom_boxplot(),
        "violin" => plot.geom_violin(),
        "density" => plot.geom_density(),
        "freqpoly" => plot.geom_freqpoly(),
        "histogram" => plot
            .geom_histogram_with(crate::geom::histogram::GeomHistogram::default().with_bins(bins)),
        "bin2d" => plot.geom_bin2d(),
        "hex" => plot.geom_hex(),
        "tile" => plot.geom_tile(),
        "smooth" => plot.geom_smooth_with(smooth_geom(&v)),
        _ => plot.geom_point(),
    };
    if geom != "smooth" && v.get("smooth").is_some() {
        plot = plot.geom_smooth_with(smooth_geom(&v));
    }

    if v.get("flip").and_then(|x| x.as_bool()).unwrap_or(false) {
        plot = plot.coord_flip();
    }
    if v.get("log_y").and_then(|x| x.as_bool()).unwrap_or(false) {
        plot = plot.scale_y_log10();
    }
    if v.get("log_x").and_then(|x| x.as_bool()).unwrap_or(false) {
        plot = plot.scale_x_log10();
    }

    let pal = brewer(v.get("palette").and_then(|x| x.as_str()).unwrap_or("Set1"));
    // Fixed factor order (optional) → stable series colors under legend toggling.
    let color_levels: Option<Vec<String>> =
        v.get("color_levels").and_then(|a| a.as_array()).map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        });
    if let Some(cc) = &color_col {
        plot = if *is_str.get(cc).unwrap_or(&false) {
            let mut s = crate::scale::color::ScaleColorDiscrete::new(Aesthetic::Color)
                .with_named_palette(&pal);
            if let Some(lv) = color_levels.clone() {
                s = s.with_levels(lv);
            }
            plot.scale_color(s)
        } else {
            plot.scale_color_viridis_c()
        };
    }
    if let Some(fc) = &fill_col {
        plot = if *is_str.get(fc).unwrap_or(&false) {
            plot.scale_fill_brewer(pal)
        } else {
            plot.scale_fill_viridis_c()
        };
    } else if matches!(geom, "bin2d" | "hex") {
        plot = plot.scale_fill_viridis_c();
    }
    if !v.get("legend").and_then(|x| x.as_bool()).unwrap_or(true) {
        plot = plot.show_legend(false);
    }

    plot = plot.theme_minimal();
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

    // Draw the unselected points first and the brushed subset last, so the
    // selection sits crisp on top of the faded rest.
    let has_sel = selected.len() >= n;
    let order: Vec<usize> = if has_sel {
        let mut o: Vec<usize> = (0..n).filter(|&i| selected[i] == 0).collect();
        o.extend((0..n).filter(|&i| selected[i] != 0));
        o
    } else {
        (0..n).collect()
    };

    let mut cols: Vec<(String, Vec<Value>)> = vec![
        (
            "x".to_string(),
            order.iter().map(|&i| Value::Float(x[i])).collect(),
        ),
        (
            "y".to_string(),
            order.iter().map(|&i| Value::Float(y[i])).collect(),
        ),
    ];
    let mut aes = Aes::new().x("x").y("y");
    let has_group = !group_names.is_empty() && group_idx.len() >= n;
    if has_group {
        let col = order
            .iter()
            .map(|&i| {
                Value::Str(
                    group_names
                        .get(group_idx[i] as usize)
                        .cloned()
                        .unwrap_or_default(),
                )
            })
            .collect();
        cols.push(("g".to_string(), col));
        aes = aes.color("g");
    }
    // Per-point alpha (geom_point reads a raw `alpha` column): the brushed
    // subset stays fully opaque, the rest fade to a faint fog.
    if has_sel {
        let alpha = order
            .iter()
            .map(|&i| Value::Float(if selected[i] != 0 { 1.0 } else { 0.05 }))
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
