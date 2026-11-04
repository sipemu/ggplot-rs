//! Browser (WebAssembly) bindings for ggplot-rs (`ggplot-rs-wasm` crate).
//!
//! Exposes a plotters-free renderer to JavaScript: pass a JSON spec (geometry +
//! optional fill/label + options) and get back an SVG string with per-feature
//! `<title>` hover tooltips. Pair with DuckDB-Wasm (spatial extension) to read
//! shapefiles/GeoJSON in the browser and `ST_AsText(geom)` them into `geometry`.

use serde_json::Value as J;
use wasm_bindgen::prelude::*;

use ggplot_rs::data::Value;
use ggplot_rs::geom::sf::GeomSf;
use ggplot_rs::prelude::*;
use ggplot_rs::spatial::SfProjection;

use ggplot_rs::spec;

// TypeScript mirror of `ggplot_rs::spec::PlotSpec` — keep in sync with src/spec.rs
// (the `ts_mirror_matches_spec` test checks the geom list and layer params).
#[wasm_bindgen(typescript_custom_section)]
const PLOT_SPEC_TS: &'static str = r##"
/** One data column: numbers → continuous, strings/booleans → categorical, null → missing. */
export type Column = (number | string | boolean | null)[];
/** Column-oriented data: `{ column: [values…] }`; all columns the same length. */
export type Data = Record<string, Column>;

export type Aesthetic =
  | "x" | "y" | "color" | "colour" | "fill" | "size" | "shape" | "alpha" | "group"
  | "linetype" | "linewidth" | "label" | "weight" | "xmin" | "xmax" | "ymin" | "ymax"
  | "xend" | "yend" | "angle" | "radius" | "tooltip" | "from" | "to" | "subgroup"
  | "open" | "high" | "low" | "close" | "lower" | "middle" | "upper" | "fontface" | "class";
/** Aesthetic → column name. */
export type AesMap = Partial<Record<Aesthetic, string>>;

/** Every geom `render()` accepts — `geoms()` returns the same list at runtime. */
export type GeomName =
  | "point" | "jitter" | "line" | "path" | "step" | "area" | "ribbon" | "stepribbon"
  | "col" | "bar" | "segment" | "curve" | "spoke" | "rect" | "tile" | "raster" | "polygon"
  | "text" | "label" | "text_repel" | "label_repel" | "rug" | "blank" | "hline" | "vline"
  | "abline" | "histogram" | "freqpoly" | "density" | "density_ridges" | "boxplot"
  | "violin" | "dotplot" | "smooth" | "count" | "bin2d" | "hex" | "density2d" | "contour"
  | "contour_filled" | "qq" | "qq_line" | "qq_band" | "censor_marks" | "errorbar"
  | "errorbarh" | "linerange" | "pointrange" | "crossbar" | "xrange" | "waterfall"
  | "pareto" | "funnel" | "pyramid" | "lollipop" | "dumbbell" | "bullet"
  | "bullet_vertical" | "marimekko" | "variwide" | "waffle" | "parliament" | "tilemap"
  | "packed_bubble" | "variable_pie" | "wordcloud" | "venn" | "timeline" | "calendar"
  | "flags" | "windbarb" | "treemap" | "sunburst" | "icicle" | "dendrogram" | "tree"
  | "sankey" | "chord" | "arcdiagram" | "network" | "stream" | "candlestick" | "ohlc"
  | "hlc" | "heikin_ashi" | "renko" | "point_figure" | "sf" | "sf_text" | "sf_label"
  | "flow";

export interface LayerParams {
  /** histogram: bin count. */
  bins?: number;
  /** smooth: fitting method (default "lm"). */
  method?: "lm" | "loess";
  /** smooth (loess): span (default 0.75). */
  span?: number;
  /** smooth: draw the confidence ribbon (default true). */
  se?: boolean;
  /** hline (required). */
  yintercept?: number;
  /** vline (required). */
  xintercept?: number;
  /** abline (slope required, intercept default 0). */
  slope?: number;
  intercept?: number;
  /** sunburst / icicle (required): hierarchy columns, outermost first. */
  levels?: string[];
  /** waffle (default 10) / parliament (default 6): rows. */
  rows?: number;
  /** renko / point_figure (required): price box size. */
  box_size?: number;
  /** point_figure: reversal in boxes (default 3). */
  reversal?: number;
  /** tilemap tile shape (default "square"). */
  shape?: "square" | "hexagon" | "circle" | "diamond";
  /** windbarb (required): speed column (knots). */
  speed?: string;
  /** windbarb (required): direction column (degrees the wind comes from). */
  direction?: string;
  /** censor_marks (required): 0/1 censoring-indicator column. */
  censor?: string;
  /** sf / sf_text / sf_label / flow: map projection (default "platecarree"). */
  projection?: "platecarree" | "mercator" | "equal-earth" | "albers-usa" | "albers-europe" | `utm:${string}`;
  /** bullet / bullet_vertical: target column. */
  target?: string;
  /** bullet / bullet_vertical: qualitative range band upper bounds. */
  ranges?: number[];
  /** boxplot: notched box (`notch = TRUE`). */
  notch?: boolean;
  /** boxplot: box width ∝ √n. */
  varwidth?: boolean;
  /** boxplot: whisker reach in IQRs (default 1.5). */
  coef?: number;
  /** boxplot: draw outliers (default true; false also drops them from the scale). */
  outliers?: boolean;
  /** boxplot: whisker cap width relative to the box (0 = none). */
  staplewidth?: number;
  /** violin: trim to the data range (default true). */
  trim?: boolean;
  /** violin: `"area"` (default), `"count"`, `"width"`. */
  scale?: "area" | "count" | "width";
  /** violin: draw lines at these sample quantiles. */
  quantiles?: number[];
  /** violin: fixed kernel bandwidth (default `bw.nrd0`). */
  bw?: number;
  /** violin: bandwidth multiplier. */
  adjust?: number;
  /** stat "connect": how consecutive points join (default "hv"). */
  connection?: "hv" | "vh" | "mid" | "linear";
  /** stat "summary_hex": summary of the raw `z` column (default "mean"). */
  fun?: "mean" | "median" | "min" | "max" | "sum";
}

export type PositionName = "identity" | "stack" | "fill" | "dodge" | "dodge2" | "jitter" | "nudge";
export type Position =
  | PositionName
  | {
      type: PositionName; width?: number; height?: number; padding?: number; x?: number; y?: number;
      /** dodge / dodge2: keep one group's width in slots with fewer groups (default "total"). */
      preserve?: "total" | "single";
    };

/** A per-aesthetic guide: `"none"`, a colourbar kind, or its options (#409). */
export type GuideSpec =
  | "none"
  | "colourbar"
  | "colorbar"
  | "coloursteps"
  | "colorsteps"
  | {
      title?: string;
      reverse?: boolean;
      type?: "legend" | "colourbar" | "colorbar" | "coloursteps" | "colorsteps";
      /** Bar width / height in px. */
      barwidth?: number;
      barheight?: number;
      nbin?: number;
      display?: "gradient" | "rectangles";
      frame?: boolean;
      ticks?: boolean;
      even_steps?: boolean;
      show_limits?: boolean;
    };

export interface Layer {
  geom: GeomName;
  /** Merged over the plot-level `aes` (layer wins). */
  aes?: AesMap;
  /** Layer-specific data, replacing the plot's. */
  data?: Data;
  params?: LayerParams;
  /** Override the geom's default stat: "identity" (plot values as given), "unique"
   *  (drop duplicate rows), "connect" (steps, see params.connection), "summary_hex"
   *  (hexagons of params.fun over a `z` column), "sf_coordinates" (a point per WKT
   *  `geometry`, e.g. text on a map). */
  stat?: "identity" | "unique" | "connect" | "summary_hex" | "sf_coordinates";
  /** Legend key picture (ggplot2's key_glyph); default per geom. */
  key_glyph?: "point" | "path" | "rect" | "polygon" | "boxplot" | "crossbar" | "pointrange"
    | "linerange" | "smooth" | "text" | "label" | "vline" | "vpath" | "abline" | "timeseries"
    | "dotplot" | "blank";
  /** Show this layer in the legends, or per aesthetic: `{ color: true, fill: false }`. */
  show_legend?: boolean | Partial<Record<"color" | "colour" | "fill" | "shape" | "size" | "alpha" | "linetype" | "linewidth", boolean>>;
  position?: Position;
}

export interface PositionScale {
  trans?: "identity" | "log10" | "log2" | "ln" | "log" | "sqrt" | "reverse" | "logit"
    | "probit" | "pseudo_log" | "reciprocal" | "exp";
  limits?: [number, number];
  breaks?: number[];
  /** About this many breaks (ggplot2's n.breaks). */
  n_breaks?: number;
  /** Time-of-day axis: values in seconds, HH:MM:SS labels (scale_x_time). */
  time?: boolean;
  /** Axis title. */
  name?: string;
  /** Secondary axis (x: top, y: right): `true` repeats the primary
   *  (`dup_axis()`), or `secondary = primary * scale + offset`. */
  sec_axis?: boolean | { name?: string; scale?: number; offset?: number };
}

export interface ColorScale {
  /** Discrete ("Set1", "Dark2", "Paired", "npg", "lancet", …) for a categorical
   *  column; "viridis" | "magma" | "plasma" | "inferno" for a numeric one. */
  palette?: string;
  /** Fixed level order for a categorical column. */
  levels?: string[];
  /** Manual colors: `{ level: "#rrggbb" }`. */
  values?: Record<string, string>;
  /** Two-color gradient `["#low", "#high"]` for a numeric column. */
  gradient?: [string, string];
  /** Diverging binned `["#low", "#mid", "#high"]` (`scale_*_steps2`), mid at `midpoint` (default 0). */
  steps2?: [string, string, string];
  midpoint?: number;
  /** Bins for `steps2` / `palette: "viridis_b:A"…"viridis_b:H"` (default 5). */
  bins?: number;
}

/** A size / alpha / linewidth / shape scale: exactly one of these (#408). */
export interface ValueScale {
  /** The column already holds the values (shape: `pch` codes or names). */
  identity?: boolean;
  /** Manual `{ level: number }` (not for shape). */
  values?: Record<string, number>;
  /** Output range `[lo, hi]` (size: radius, ggplot2's `scale_radius`). */
  range?: [number, number];
}

/** Which panels get axes, and whether inner axes are labelled (#412). */
export interface FacetAxesSpec {
  axes?: "margins" | "all" | "all_x" | "all_y";
  axis_labels?: "all" | "margins";
}

export type Facet = FacetAxesSpec & (
  | {
      wrap: string; ncol?: number; scales?: "fixed" | "free" | "free_x" | "free_y";
      /** "h" (row by row, default) or "v" (column by column). */
      dir?: "h" | "v";
      /** First row at the top (default true; ggplot2's as.table). */
      as_table?: boolean;
      /** Wrap strip labels at this many characters (label_wrap_gen). */
      label_wrap?: number;
      /** The side of each panel its strip sits on (default "top"). */
      strip_position?: "top" | "bottom" | "left" | "right";
    }
  | {
      rows?: string; cols?: string; scales?: "fixed" | "free" | "free_x" | "free_y";
      /** Column strips below ("x"), row strips left ("y"), or both. */
      switch?: "x" | "y" | "both";
    });

export type CoordName =
  | "cartesian" | "flip" | "polar" | "radial" | "fixed" | "equal" | "radar" | "quickmap" | "map" | "sf";
export type Coord =
  | CoordName
  | {
      type: CoordName; ratio?: number; xlim?: [number, number]; ylim?: [number, number];
      /** cartesian: mirror the panel. */
      reverse?: "x" | "y" | "xy" | "none";
      /** cartesian: keep the scale padding — `false`, or `[top, right, bottom, left]`. */
      expand?: boolean | [boolean, boolean, boolean, boolean];
      /** radial: theta start / end in radians (clockwise from 12 o'clock) and a donut hole (0–1). */
      start?: number; end?: number; inner_radius?: number;
    };

export interface Labels {
  title?: string;
  subtitle?: string;
  caption?: string;
  tag?: string;
  x?: string;
  y?: string;
}

export type ThemePreset = "gray" | "grey" | "bw" | "minimal" | "classic" | "dark" | "light" | "void" | "linedraw" | "test";
export interface TextElement {
  family?: string; size?: number; color?: [number, number, number];
  face?: "plain" | "bold" | "italic"; hjust?: number; vjust?: number; angle?: number; visible?: boolean;
}
export interface LineElement {
  color?: [number, number, number]; width?: number; visible?: boolean;
  linetype?: "solid" | "dashed" | "dotted" | "dashdot" | "longdash" | "twodash";
}
export interface RectElement {
  fill?: [number, number, number]; color?: [number, number, number]; width?: number; visible?: boolean;
}
/** A partial theme overlay (same schema as the CLI's `--theme-config`). */
export interface ThemeConfig {
  /** Preset to start from (default "minimal"). */
  base?: ThemePreset;
  /** Brand color `[r, g, b]` for single-series geoms. */
  primary?: [number, number, number];
  /** Default discrete palette for categorical color/fill mappings. */
  palette?: string;
  aspect_ratio?: number;
  panel_ontop?: boolean;
  axis_minor_ticks?: boolean;
  title_position?: "panel" | "plot";
  tag_position?: "topleft" | "topright" | "bottomleft" | "bottomright";
  legend_direction?: "vertical" | "horizontal";
  text?: TextElement; title?: TextElement; subtitle?: TextElement; caption?: TextElement;
  axis_title_x?: TextElement; axis_title_y?: TextElement;
  axis_text_x?: TextElement; axis_text_y?: TextElement;
  /** A top x axis's / right (secondary) y axis's labels (#413). */
  axis_text_x_top?: TextElement; axis_text_y_right?: TextElement;
  legend_title?: TextElement; legend_text?: TextElement; strip_text?: TextElement;
  axis_line?: LineElement; axis_ticks?: LineElement;
  panel_grid_major?: LineElement; panel_grid_minor?: LineElement; panel_border?: LineElement;
  panel_background?: RectElement; plot_background?: RectElement;
  legend_background?: RectElement; strip_background?: RectElement;
  [key: string]: unknown;
}

/** "inside" places the legend automatically in the emptiest panel corner. */
export type Legend = "right" | "left" | "top" | "bottom" | "none" | "inside" | [number, number];

/** A layered grammar-of-graphics plot, mirroring the ggplot-rs builder. */
export interface PlotSpec {
  data: Data;
  /** Plot-level mapping inherited by every layer. */
  aes?: AesMap;
  layers: Layer[];
  scales?: { x?: PositionScale; y?: PositionScale; color?: ColorScale; colour?: ColorScale; fill?: ColorScale; size?: ValueScale; alpha?: ValueScale; linewidth?: ValueScale; shape?: ValueScale };
  facet?: Facet;
  coord?: Coord;
  labels?: Labels;
  /** A preset name or a theme overlay (default "minimal"). */
  theme?: ThemePreset | ThemeConfig;
  legend?: Legend;
  /** Per-aesthetic guides: `"none"` drops a legend, `{ title }` names it. */
  guides?: Partial<Record<"color" | "colour" | "fill" | "shape" | "size" | "alpha" | "linetype" | "linewidth", GuideSpec>>;
  /** Pixels (default 640 × 400). */
  width?: number;
  height?: number;
}
"##;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "PlotSpec | string")]
    pub type PlotSpecInput;
}

/// Accept a spec as a JS object or as JSON text.
fn spec_json(spec: PlotSpecInput) -> Result<String, JsValue> {
    let v: JsValue = spec.into();
    if let Some(s) = v.as_string() {
        return Ok(s);
    }
    js_sys::JSON::stringify(&v)
        .map(String::from)
        .map_err(|_| JsValue::from_str("spec is not JSON-serialisable"))
}

/// Render a layered plot spec (a {@link PlotSpec} object or its JSON text) to
/// an SVG document. Throws an `Error` naming the offending field — unknown
/// columns list the available ones.
#[wasm_bindgen]
pub fn render(spec: PlotSpecInput) -> Result<String, JsValue> {
    let json = spec_json(spec)?;
    spec::render_json(&json)
        .map(|(svg, _)| svg)
        .map_err(|e| js_sys::Error::new(&e).into())
}

/// An SVG plus the build warnings (rows dropped for non-finite values, layers
/// whose stat produced no data, …).
#[wasm_bindgen]
pub struct RenderResult {
    svg: String,
    warnings: Vec<String>,
}

#[wasm_bindgen]
impl RenderResult {
    #[wasm_bindgen(getter)]
    pub fn svg(&self) -> String {
        self.svg.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn warnings(&self) -> Vec<String> {
        self.warnings.clone()
    }
}

/// Like {@link render}, but also returns the build warnings.
#[wasm_bindgen(js_name = renderWithWarnings)]
pub fn render_with_warnings(spec: PlotSpecInput) -> Result<RenderResult, JsValue> {
    let json = spec_json(spec)?;
    spec::render_json(&json)
        .map(|(svg, warnings)| RenderResult { svg, warnings })
        .map_err(|e| js_sys::Error::new(&e).into())
}

/// The geom names {@link render} accepts.
#[wasm_bindgen]
pub fn geoms() -> Vec<String> {
    spec::geoms().iter().map(|g| g.to_string()).collect()
}

/// The ggplot-rs version this package was built from.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Render a `geom_sf` map from a JSON spec, returning an SVG document.
///
/// Spec fields:
/// - `geometry`: `[String]` — WKT (required)
/// - `fill`: `[Number]` — choropleth values (optional)
/// - `label`: `[String]` — hover labels (optional)
/// - `fill_name`, `title`: `String` (optional)
/// - `width`, `height`: `Number` (default 800×600)
/// - `projection`: `"mercator"` | `"platecarree"` (default) | `"equal-earth"` |
///   `"albers-usa"` | `"albers-europe"` | `"utm:<zone>"` / `"utm:<zone>s"` (south,
///   zone 1-60)
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
    // Lenient: an unknown name or bad UTM zone falls back to plate carrée.
    let projection = v
        .get("projection")
        .and_then(|x| x.as_str())
        .and_then(|p| SfProjection::parse(p).ok())
        .unwrap_or_default();
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
            .and_then(ggplot_rs::spatial::parse_wkt)
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
        .scale_fill_brewer(ggplot_rs::scale::palettes::PaletteName::Set1)
        .theme_minimal();
    // The x-axis already labels the categories, so the fill legend is redundant.
    if !v.get("legend").and_then(|x| x.as_bool()).unwrap_or(true) {
        plot = plot.show_legend(false);
    }
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
    let geom = ggplot_rs::geom::histogram::GeomHistogram::default().with_bins(bins.max(1) as usize);
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

fn brewer(name: &str) -> ggplot_rs::scale::palettes::PaletteName {
    use ggplot_rs::scale::palettes::PaletteName as P;
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

fn smooth_geom(v: &J) -> ggplot_rs::geom::smooth::GeomSmooth {
    let g = ggplot_rs::geom::smooth::GeomSmooth::default();
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
                let f = ggplot_rs::format::format_value;
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
        "histogram" => plot.geom_histogram_with(
            ggplot_rs::geom::histogram::GeomHistogram::default().with_bins(bins),
        ),
        "bin2d" => plot.geom_bin2d(),
        "hex" => plot.geom_hex(),
        "tile" => plot.geom_tile(),
        "density2d" => plot.geom_density2d(),
        "count" => plot.geom_count(),
        "rug" => plot.geom_rug(),
        "smooth" => plot.geom_smooth_with(smooth_geom(&v)),
        _ => plot.geom_point(),
    };
    if geom != "smooth" && v.get("smooth").is_some() {
        plot = plot.geom_smooth_with(smooth_geom(&v));
    }

    if v.get("flip").and_then(|x| x.as_bool()).unwrap_or(false) {
        plot = plot.coord_flip();
    }
    if v.get("coord").and_then(|x| x.as_str()) == Some("polar") {
        plot = plot.coord_polar();
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
            let mut s = ggplot_rs::scale::color::ScaleColorDiscrete::new(Aesthetic::Color)
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

/// Axes/gridlines frame for a WebGL scatter overlay: the SVG (no marks) plus the
/// panel rect in pixels and the expanded data domain that maps to the panel edges.
#[wasm_bindgen]
pub struct Frame {
    svg: String,
    plot: Vec<f64>,
    xdom: Vec<f64>,
    ydom: Vec<f64>,
}

#[wasm_bindgen]
impl Frame {
    #[wasm_bindgen(getter)]
    pub fn svg(&self) -> String {
        self.svg.clone()
    }
    /// Panel rect `[x, y, width, height]` in pixels.
    #[wasm_bindgen(getter)]
    pub fn plot(&self) -> Vec<f64> {
        self.plot.clone()
    }
    /// Expanded x range `[min, max]` mapping to the panel's left/right edge.
    #[wasm_bindgen(getter)]
    pub fn xdom(&self) -> Vec<f64> {
        self.xdom.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn ydom(&self) -> Vec<f64> {
        self.ydom.clone()
    }
}

/// Render just the axes/gridlines (no marks, via `geom_blank`) for the given
/// data ranges, returning the SVG plus the panel rect and expanded domain — the
/// frame a WebGL/canvas layer draws points onto.
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn scatter_frame(
    xmin: f64,
    xmax: f64,
    ymin: f64,
    ymax: f64,
    width: u32,
    height: u32,
    title: String,
) -> Result<Frame, JsValue> {
    scatter_frame_impl(xmin, xmax, ymin, ymax, width, height, &title)
        .map_err(|e| JsValue::from_str(&e))
}

#[allow(clippy::too_many_arguments)]
fn scatter_frame_impl(
    xmin: f64,
    xmax: f64,
    ymin: f64,
    ymax: f64,
    width: u32,
    height: u32,
    title: &str,
) -> Result<Frame, String> {
    let cols = vec![
        (
            "x".to_string(),
            vec![Value::Float(xmin), Value::Float(xmax)],
        ),
        (
            "y".to_string(),
            vec![Value::Float(ymin), Value::Float(ymax)],
        ),
    ];
    let mut plot = GGPlot::new(cols)
        .aes(Aes::new().x("x").y("y"))
        .geom_blank()
        .theme_minimal();
    if !title.is_empty() {
        plot = plot.title(title);
    }
    let (svg, pa) = plot
        .render_svg_area_with_size(width, height)
        .map_err(|e| format!("frame render failed: {e:?}"))?;
    let ex = if (xmax - xmin).abs() < 1e-12 {
        1.0
    } else {
        (xmax - xmin) * 0.05
    };
    let ey = if (ymax - ymin).abs() < 1e-12 {
        1.0
    } else {
        (ymax - ymin) * 0.05
    };
    Ok(Frame {
        svg,
        plot: pa.to_vec(),
        xdom: vec![xmin - ex, xmax + ex],
        ydom: vec![ymin - ey, ymax + ey],
    })
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
        plot = plot.scale_color_brewer(ggplot_rs::scale::palettes::PaletteName::Set1);
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
        // Fixed factor order so the group→colour mapping is stable regardless of
        // the draw-order reshuffle above (else selection colours don't match the
        // linked bar, which always uses the canonical order).
        let s = ggplot_rs::scale::color::ScaleColorDiscrete::new(Aesthetic::Color)
            .with_named_palette(&ggplot_rs::scale::palettes::PaletteName::Set1)
            .with_levels(group_names.clone());
        plot = plot.scale_color(s);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The hand-written TypeScript types must name every spec geom and every
    /// `LayerParams` field, or `.d.ts` users can't reach them.
    #[test]
    fn ts_mirror_matches_spec() {
        // `#[wasm_bindgen(typescript_custom_section)]` consumes the const, so
        // read the TypeScript from this file's source.
        let ts = include_str!("lib.rs");
        let block = |start: &str, end: &str| {
            let a = ts.find(start).expect(start);
            let b = a + ts[a..].find(end).unwrap();
            &ts[a..b]
        };
        let ts_geoms = block("export type GeomName =", ";");
        for g in spec::GEOMS {
            assert!(
                ts_geoms.contains(&format!("\"{g}\"")),
                "GeomName lacks \"{g}\""
            );
        }
        let ts_params = block("export interface LayerParams {", "\n}");
        let rs = include_str!("../../../src/spec.rs");
        let rs = &rs[rs.find("pub struct LayerParams {").unwrap()..];
        let rs = &rs[..rs.find("\n}").unwrap()];
        for line in rs.lines().skip(1) {
            if let Some(field) = line
                .trim()
                .strip_prefix("pub ")
                .and_then(|l| l.split(':').next())
            {
                assert!(
                    ts_params.contains(&format!("{field}?:")),
                    "LayerParams lacks {field}"
                );
            }
        }
    }

    fn spec(projection: &str) -> String {
        format!(
            r#"{{"geometry": ["POLYGON ((0 0, 10 0, 10 10, 0 10, 0 0))"], "projection": "{projection}"}}"#
        )
    }

    /// #177: `render_geo`'s `projection` field accepts the new named
    /// projections/UTM zones ggplot-rs's `SfProjection` gained, each
    /// producing a render distinct from the `platecarree` default (proving
    /// the string was actually parsed into that projection, not silently
    /// falling back).
    #[test]
    fn projection_names_are_recognised() {
        let plate = render_geo_impl(&spec("platecarree")).expect("platecarree");
        for name in [
            "equal-earth",
            "albers-usa",
            "albers-europe",
            "utm:33",
            "utm:33s",
        ] {
            let svg = render_geo_impl(&spec(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_ne!(svg, plate, "{name} should reproject, not pass through");
        }
    }

    #[test]
    fn utm_zone_out_of_range_falls_back_to_platecarree() {
        let plate = render_geo_impl(&spec("platecarree")).expect("platecarree");
        let bad_zone = render_geo_impl(&spec("utm:0")).expect("bad zone still renders");
        let non_numeric = render_geo_impl(&spec("utm:abc")).expect("non-numeric still renders");
        assert_eq!(bad_zone, plate);
        assert_eq!(non_numeric, plate);
    }

    #[test]
    fn unknown_projection_name_falls_back_to_platecarree() {
        let plate = render_geo_impl(&spec("platecarree")).expect("platecarree");
        let unknown = render_geo_impl(&spec("robinson")).expect("unknown still renders");
        assert_eq!(unknown, plate);
    }
}
