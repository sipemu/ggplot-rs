//! A declarative JSON plot spec (feature `spec`).
//!
//! A JSON object that maps one-to-one onto the [`GGPlot`] builder:
//! `{ data, aes, layers: [{ geom, aes?, data?, params?, stat?, position? }],
//! scales, facet, coord, labels, theme, legend, width, height }`.
//!
//! This one module is the engine behind every non-Rust front end, so they
//! stay on par: the npm package's `render()` (crates/ggplot-rs-wasm) and the
//! CLI's `--spec` / `--geom` / `--param` (src/bin/ggplot.rs). Every
//! `GGPlot::geom_*` builder is either reachable from [`geoms`] or listed in
//! [`EXCLUDED_GEOMS`] with a reason — the `every_builder_geom_is_in_the_spec`
//! test enforces that. The TypeScript mirror lives in `PLOT_SPEC_TS` in
//! crates/ggplot-rs-wasm/src/lib.rs; a test there checks it against [`geoms`].
//!
//! ```
//! let svg = ggplot_rs::spec::render_json(r#"{
//!     "data": {"x": [1, 2, 3], "y": [2, 4, 3]},
//!     "aes": {"x": "x", "y": "y"},
//!     "layers": [{"geom": "point"}, {"geom": "line"}]
//! }"#).unwrap().0;
//! assert!(svg.starts_with("<svg"));
//! ```

use std::collections::{BTreeMap, HashMap};

use serde::Deserialize;
use serde_json::Value as J;

use crate::data::Value;
use crate::facet::FacetScales;
use crate::geom::histogram::GeomHistogram;
use crate::geom::smooth::GeomSmooth;
use crate::prelude::*;
use crate::scale::color::{RGBAColor, ScaleColorDiscrete};
use crate::scale::continuous::ScaleContinuous;
use crate::scale::gradient_n::ScaleColorGradientN;
use crate::scale::transform::ScaleTransform;
use crate::spatial::SfProjection;
use crate::theme::config::{parse_palette, preset, ThemeConfig};
use crate::theme::LegendPosition;

/// Column-oriented data: `{ column: [values…] }`.
pub type Columns = BTreeMap<String, Vec<J>>;
/// Aesthetic → column name.
pub type AesMap = BTreeMap<String, String>;

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PlotSpec {
    /// Plot data. May be empty when the caller supplies the data itself
    /// ([`build_plot_with_data`], e.g. the CLI's DuckDB input).
    #[serde(default)]
    pub data: Columns,
    #[serde(default)]
    pub aes: AesMap,
    pub layers: Vec<LayerSpec>,
    #[serde(default)]
    pub scales: ScalesSpec,
    pub facet: Option<FacetSpec>,
    pub coord: Option<CoordSpec>,
    #[serde(default)]
    pub labels: LabelsSpec,
    pub theme: Option<ThemeSpec>,
    pub legend: Option<LegendSpec>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct LayerSpec {
    pub geom: String,
    pub aes: Option<AesMap>,
    pub data: Option<Columns>,
    #[serde(default)]
    pub params: LayerParams,
    pub stat: Option<String>,
    pub position: Option<PositionSpec>,
}

/// Geom parameters. Each is read only by the geoms it applies to.
#[derive(Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct LayerParams {
    /// histogram / freqpoly bin count.
    pub bins: Option<usize>,
    /// smooth: `"lm"` (default) or `"loess"`.
    pub method: Option<String>,
    /// smooth (loess) span.
    pub span: Option<f64>,
    /// smooth: draw the confidence ribbon (default true).
    pub se: Option<bool>,
    /// hline.
    pub yintercept: Option<f64>,
    /// vline.
    pub xintercept: Option<f64>,
    /// abline.
    pub slope: Option<f64>,
    /// abline.
    pub intercept: Option<f64>,
    /// sunburst / icicle hierarchy columns, outermost first.
    pub levels: Option<Vec<String>>,
    /// waffle / parliament: rows of squares / seat rows.
    pub rows: Option<usize>,
    /// renko / point_figure: price box size.
    pub box_size: Option<f64>,
    /// point_figure: reversal in boxes (default 3).
    pub reversal: Option<usize>,
    /// tilemap: `"square"` (default), `"hexagon"`, `"circle"`, `"diamond"`.
    pub shape: Option<String>,
    /// windbarb: speed column (knots).
    pub speed: Option<String>,
    /// windbarb: direction column (degrees the wind comes from).
    pub direction: Option<String>,
    /// censor_marks: 0/1 censoring-indicator column.
    pub censor: Option<String>,
    /// sf / sf_text / sf_label / flow: map projection (see
    /// [`SfProjection::parse`]); default plate carrée.
    pub projection: Option<String>,
    /// bullet / bullet_vertical: target column.
    pub target: Option<String>,
    /// bullet / bullet_vertical: qualitative range band upper bounds.
    pub ranges: Option<Vec<f64>>,
    /// boxplot: notched box (`notch = TRUE`).
    pub notch: Option<bool>,
    /// boxplot: box width ∝ √n.
    pub varwidth: Option<bool>,
    /// boxplot: whisker reach in IQRs (default 1.5).
    pub coef: Option<f64>,
    /// boxplot: draw outliers (default true; false also drops them from the scale).
    pub outliers: Option<bool>,
    /// boxplot: whisker cap width relative to the box (0 = none).
    pub staplewidth: Option<f64>,
    /// violin: trim to the data range (default true).
    pub trim: Option<bool>,
    /// violin: `"area"` (default), `"count"`, `"width"`.
    pub scale: Option<String>,
    /// violin: draw lines at these sample quantiles.
    pub quantiles: Option<Vec<f64>>,
    /// violin: fixed kernel bandwidth (default `bw.nrd0`).
    pub bw: Option<f64>,
    /// violin: bandwidth multiplier.
    pub adjust: Option<f64>,
}

impl LayerParams {
    /// Copy every parameter set in `other` onto `self` (the CLI's `--param`
    /// over a `--spec` file's layers). Destructures exhaustively, so a new
    /// field can't be forgotten here.
    pub fn overlay(&mut self, other: LayerParams) {
        let LayerParams {
            bins,
            method,
            span,
            se,
            yintercept,
            xintercept,
            slope,
            intercept,
            levels,
            rows,
            box_size,
            reversal,
            shape,
            speed,
            direction,
            censor,
            projection,
            target,
            ranges,
            notch,
            varwidth,
            coef,
            outliers,
            staplewidth,
            trim,
            scale,
            quantiles,
            bw,
            adjust,
        } = other;
        macro_rules! set {
            ($($f:ident),*) => { $( if $f.is_some() { self.$f = $f; } )* };
        }
        set!(
            bins,
            method,
            span,
            se,
            yintercept,
            xintercept,
            slope,
            intercept,
            levels,
            rows,
            box_size,
            reversal,
            shape,
            speed,
            direction,
            censor,
            projection,
            target,
            ranges,
            notch,
            varwidth,
            coef,
            outliers,
            staplewidth,
            trim,
            scale,
            quantiles,
            bw,
            adjust
        );
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum PositionSpec {
    Name(String),
    Full(PositionFull),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PositionFull {
    #[serde(rename = "type")]
    pub kind: String,
    /// dodge width, jitter width.
    pub width: Option<f64>,
    /// jitter height.
    pub height: Option<f64>,
    /// dodge2 padding.
    pub padding: Option<f64>,
    /// nudge offsets.
    pub x: Option<f64>,
    pub y: Option<f64>,
    /// dodge / dodge2: `"total"` (default) or `"single"` (elements keep one
    /// group's width in slots with fewer groups).
    pub preserve: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ScalesSpec {
    pub x: Option<PositionScaleSpec>,
    pub y: Option<PositionScaleSpec>,
    #[serde(alias = "colour")]
    pub color: Option<ColorScaleSpec>,
    pub fill: Option<ColorScaleSpec>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PositionScaleSpec {
    /// `"identity"`, `"log10"`, `"log2"`, `"ln"`, `"sqrt"`, `"reverse"`,
    /// `"logit"`, `"probit"`, `"pseudo_log"`, `"reciprocal"`, `"exp"`.
    pub trans: Option<String>,
    pub limits: Option<[f64; 2]>,
    pub breaks: Option<Vec<f64>>,
    pub name: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ColorScaleSpec {
    /// A named palette: discrete (`"Set1"`, `"Dark2"`, `"npg"`, …) for a
    /// categorical column; `"viridis"`/`"magma"`/`"plasma"`/`"inferno"` for
    /// a numeric one.
    pub palette: Option<String>,
    /// Fixed level order for a categorical column.
    pub levels: Option<Vec<String>>,
    /// Manual `{ level: "#rrggbb" }` colors.
    pub values: Option<BTreeMap<String, String>>,
    /// Two-color gradient `["#low", "#high"]` for a numeric column.
    pub gradient: Option<[String; 2]>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FacetSpec {
    pub wrap: Option<String>,
    pub ncol: Option<usize>,
    pub rows: Option<String>,
    pub cols: Option<String>,
    /// `"fixed"` (default), `"free"`, `"free_x"`, `"free_y"`.
    pub scales: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum CoordSpec {
    Name(String),
    Full(CoordFull),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoordFull {
    #[serde(rename = "type")]
    pub kind: String,
    /// `fixed`: y/x aspect ratio (default 1).
    pub ratio: Option<f64>,
    /// `cartesian`: zoom window.
    pub xlim: Option<[f64; 2]>,
    pub ylim: Option<[f64; 2]>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct LabelsSpec {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub caption: Option<String>,
    pub tag: Option<String>,
    pub x: Option<String>,
    pub y: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ThemeSpec {
    Preset(String),
    Config(Box<ThemeConfig>),
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum LegendSpec {
    /// `"right"`, `"left"`, `"top"`, `"bottom"`, `"none"`, `"inside"` (auto).
    Name(String),
    /// Panel-relative `[x, y]` inside the panel.
    Inside([f64; 2]),
}

/// Parse a JSON spec and render it to an SVG document plus build warnings.
pub fn render_json(spec_json: &str) -> Result<(String, Vec<String>), String> {
    render_spec(parse(spec_json)?)
}

pub fn render_spec(spec: PlotSpec) -> Result<(String, Vec<String>), String> {
    let (width, height) = spec.size()?;
    build_plot(&spec)?
        .render_svg_native_with_warnings(width, height)
        .map_err(|e| format!("render failed: {e}"))
}

/// Parse a JSON spec.
pub fn parse(spec_json: &str) -> Result<PlotSpec, String> {
    serde_json::from_str(spec_json).map_err(|e| format!("invalid plot spec: {e}"))
}

impl PlotSpec {
    /// Output size in pixels (default 640 × 400).
    pub fn size(&self) -> Result<(u32, u32), String> {
        let (width, height) = (self.width.unwrap_or(640), self.height.unwrap_or(400));
        if width == 0 || height == 0 {
            return Err("width and height must be positive".into());
        }
        Ok((width, height))
    }
}

/// Build the [`GGPlot`] a spec describes, from the spec's own `data`.
pub fn build_plot(spec: &PlotSpec) -> Result<GGPlot, String> {
    build_plot_with_data(spec, columns(&spec.data)?)
}

/// Build the [`GGPlot`] a spec describes over caller-supplied plot data
/// (the spec's `data` is ignored). Layer-level `data` still comes from the spec.
pub fn build_plot_with_data(
    spec: &PlotSpec,
    plot_data: Vec<(String, Vec<Value>)>,
) -> Result<GGPlot, String> {
    if spec.layers.is_empty() {
        return Err("spec.layers must contain at least one layer".into());
    }
    let plot_names: Vec<String> = plot_data.iter().map(|(k, _)| k.clone()).collect();
    let plot_aes = build_aes(&spec.aes, &plot_names, "aes")?;
    // Column → "holds strings?", to pick discrete vs. continuous color scales.
    let mut is_str: HashMap<String, bool> = string_columns(&plot_data);

    let mut plot = GGPlot::new(plot_data).aes(plot_aes);
    for (i, layer) in spec.layers.iter().enumerate() {
        let names: Vec<String> = match &layer.data {
            Some(d) => d.keys().cloned().collect(),
            None => plot_names.clone(),
        };
        plot =
            add_geom(plot, &layer.geom, &layer.params).map_err(|e| format!("layers[{i}]: {e}"))?;
        if let Some(d) = &layer.data {
            let d = columns(d).map_err(|e| format!("layers[{i}].data: {e}"))?;
            for (k, v) in string_columns(&d) {
                is_str.entry(k).or_insert(v);
            }
            plot = plot.layer_data(d);
        }
        if let Some(a) = &layer.aes {
            // A layer mapping adds to (and overrides) the plot-level one, as
            // in ggplot2 — `layer_aes` replaces, so merge here first.
            let mut merged = spec.aes.clone();
            merged.extend(a.iter().map(|(k, v)| (k.clone(), v.clone())));
            plot = plot.layer_aes(build_aes(&merged, &names, &format!("layers[{i}].aes"))?);
        } else if layer.data.is_some() {
            // Plot-level columns must exist in the layer's own data too.
            build_aes(&spec.aes, &names, &format!("layers[{i}] (inherited aes)"))?;
        }
        if let Some(s) = &layer.stat {
            plot = match s.as_str() {
                "identity" => plot.stat(crate::stat::identity::StatIdentity),
                other => {
                    return Err(format!(
                        "layers[{i}].stat: unknown stat '{other}' (supported: \"identity\"; \
                         other stats follow from the geom)"
                    ))
                }
            };
        }
        if let Some(p) = &layer.position {
            plot = apply_position(plot, p).map_err(|e| format!("layers[{i}].position: {e}"))?;
        }
    }

    // The column each color aesthetic maps (plot level, else first layer that
    // maps it) — decides discrete vs. continuous palettes.
    let mapped = |aes: &str| -> Option<String> {
        spec.aes.get(aes).cloned().or_else(|| {
            spec.layers
                .iter()
                .find_map(|l| l.aes.as_ref().and_then(|a| a.get(aes).cloned()))
        })
    };
    if let Some(s) = &spec.scales.x {
        plot = plot.scale_x_continuous(position_scale(s).map_err(|e| format!("scales.x: {e}"))?);
    }
    if let Some(s) = &spec.scales.y {
        plot = plot.scale_y_continuous(position_scale(s).map_err(|e| format!("scales.y: {e}"))?);
    }
    // `theme.palette` (a scale, not a theme element) colors any categorical
    // color/fill mapping that has no explicit scale.
    let theme_palette = match &spec.theme {
        Some(ThemeSpec::Config(cfg)) => cfg.palette.clone(),
        _ => None,
    };
    for (aes, scale) in [
        (Aesthetic::Color, &spec.scales.color),
        (Aesthetic::Fill, &spec.scales.fill),
    ] {
        let name = if aes == Aesthetic::Color {
            "color"
        } else {
            "fill"
        };
        let column = mapped(name).or_else(|| (name == "color").then(|| mapped("colour")).flatten());
        let discrete = column
            .as_ref()
            .map(|c| *is_str.get(c).unwrap_or(&false))
            .unwrap_or(true);
        if let Some(s) = scale {
            plot = apply_color_scale(plot, aes, s, discrete)
                .map_err(|e| format!("scales.{name}: {e}"))?;
        } else if let (Some(pal), Some(_), true) = (&theme_palette, &column, discrete) {
            let s = ColorScaleSpec {
                palette: Some(pal.clone()),
                ..Default::default()
            };
            plot = apply_color_scale(plot, aes, &s, true)
                .map_err(|e| format!("theme.palette: {e}"))?;
        }
    }

    if let Some(f) = &spec.facet {
        plot = apply_facet(plot, f).map_err(|e| format!("facet: {e}"))?;
    }
    if let Some(c) = &spec.coord {
        plot = apply_coord(plot, c).map_err(|e| format!("coord: {e}"))?;
    }

    // Theme before legend: a preset replaces the whole theme.
    plot = match &spec.theme {
        None => plot.theme_minimal(),
        Some(ThemeSpec::Preset(name)) => plot.theme(preset(name).map_err(|e| {
            format!("theme: {e} (expected gray, bw, minimal, classic, dark, light, void, linedraw)")
        })?),
        Some(ThemeSpec::Config(cfg)) => plot.theme(
            cfg.apply(crate::theme::presets::theme_minimal())
                .map_err(|e| format!("theme: {e}"))?,
        ),
    };
    if let Some(l) = &spec.legend {
        plot = match l {
            LegendSpec::Inside([x, y]) => plot.legend_position_inside(*x, *y),
            LegendSpec::Name(n) => match n.as_str() {
                "right" => plot.legend_position(LegendPosition::Right),
                "left" => plot.legend_position(LegendPosition::Left),
                "top" => plot.legend_position(LegendPosition::Top),
                "bottom" => plot.legend_position(LegendPosition::Bottom),
                "none" => plot.legend_position(LegendPosition::None),
                "inside" => plot.legend_inside_auto(),
                other => {
                    return Err(format!(
                        "legend: unknown position '{other}' (expected right, left, top, bottom, \
                         none, inside, or [x, y])"
                    ))
                }
            },
        };
    }

    let l = &spec.labels;
    if let Some(t) = &l.title {
        plot = plot.title(t);
    }
    if let Some(t) = &l.subtitle {
        plot = plot.subtitle(t);
    }
    if let Some(t) = &l.caption {
        plot = plot.caption(t);
    }
    if let Some(t) = &l.tag {
        plot = plot.tag(t);
    }
    if let Some(t) = &l.x {
        plot = plot.xlab(t);
    }
    if let Some(t) = &l.y {
        plot = plot.ylab(t);
    }

    Ok(plot)
}

fn jval(x: &J) -> Value {
    match x {
        J::Number(n) => n.as_f64().map(Value::Float).unwrap_or(Value::Na),
        J::String(s) => Value::Str(s.clone()),
        J::Bool(b) => Value::Bool(*b),
        _ => Value::Na,
    }
}

fn columns(data: &Columns) -> Result<Vec<(String, Vec<Value>)>, String> {
    let mut len = None;
    let mut out = Vec::with_capacity(data.len());
    for (k, vals) in data {
        match len {
            None => len = Some((k, vals.len())),
            Some((k0, n)) if n != vals.len() => {
                return Err(format!(
                    "columns must have equal length: '{k0}' has {n} values, '{k}' has {}",
                    vals.len()
                ))
            }
            _ => {}
        }
        out.push((k.clone(), vals.iter().map(jval).collect()));
    }
    Ok(out)
}

fn string_columns(data: &[(String, Vec<Value>)]) -> HashMap<String, bool> {
    data.iter()
        .map(|(k, vals)| {
            let s = vals
                .iter()
                .find(|x| !matches!(x, Value::Na))
                .map(|x| matches!(x, Value::Str(_) | Value::Bool(_)))
                .unwrap_or(false);
            (k.clone(), s)
        })
        .collect()
}

/// Aesthetic names the spec accepts, in the order the error message lists them.
const AES_NAMES: &[&str] = &[
    "x",
    "y",
    "color",
    "colour",
    "fill",
    "size",
    "shape",
    "alpha",
    "group",
    "linetype",
    "linewidth",
    "label",
    "weight",
    "xmin",
    "xmax",
    "ymin",
    "ymax",
    "xend",
    "yend",
    "angle",
    "radius",
    "tooltip",
    "from",
    "to",
    "subgroup",
    "open",
    "high",
    "low",
    "close",
    "lower",
    "middle",
    "upper",
    "fontface",
    "class",
];

fn build_aes(map: &AesMap, available: &[String], ctx: &str) -> Result<Aes, String> {
    let mut aes = Aes::new();
    for (name, col) in map {
        if !available.iter().any(|c| c == col) {
            return Err(format!(
                "{ctx}.{name}: column '{col}' not found (available: {})",
                available.join(", ")
            ));
        }
        aes = match name.as_str() {
            "x" => aes.x(col),
            "y" => aes.y(col),
            "color" | "colour" => aes.color(col),
            "fill" => aes.fill(col),
            "size" => aes.size(col),
            "shape" => aes.shape(col),
            "alpha" => aes.alpha(col),
            "group" => aes.group(col),
            "linetype" => aes.linetype(col),
            "linewidth" => aes.linewidth(col),
            "label" => aes.label(col),
            "weight" => aes.weight(col),
            "xmin" => aes.xmin(col),
            "xmax" => aes.xmax(col),
            "ymin" => aes.ymin(col),
            "ymax" => aes.ymax(col),
            "xend" => aes.xend(col),
            "yend" => aes.yend(col),
            "angle" => aes.angle(col),
            "radius" => aes.radius(col),
            "tooltip" => aes.tooltip(col),
            "from" => aes.from(col),
            "to" => aes.to(col),
            "subgroup" => aes.subgroup(col),
            "open" => aes.open(col),
            "high" => aes.high(col),
            "low" => aes.low(col),
            "close" => aes.close(col),
            "lower" => aes.lower(col),
            "middle" => aes.middle(col),
            "upper" => aes.upper(col),
            "fontface" => aes.fontface(col),
            "class" => aes.class(col),
            other => {
                return Err(format!(
                    "{ctx}: unknown aesthetic '{other}' (expected one of: {})",
                    AES_NAMES.join(", ")
                ))
            }
        };
    }
    Ok(aes)
}

/// Geom names [`add_geom`] understands in every build (see [`geoms`] for the
/// feature-gated extras), in the order the docs and error messages list them.
pub const GEOMS: &[&str] = &[
    // basic
    "point",
    "jitter",
    "line",
    "path",
    "step",
    "area",
    "ribbon",
    "stepribbon",
    "col",
    "bar",
    "segment",
    "curve",
    "spoke",
    "rect",
    "tile",
    "raster",
    "polygon",
    "text",
    "label",
    "text_repel",
    "label_repel",
    "rug",
    "blank",
    // reference lines
    "hline",
    "vline",
    "abline",
    // distributions & statistics
    "histogram",
    "freqpoly",
    "density",
    "density_ridges",
    "boxplot",
    "violin",
    "dotplot",
    "smooth",
    "count",
    "bin2d",
    "hex",
    "density2d",
    "contour",
    "contour_filled",
    "qq",
    "qq_line",
    "qq_band",
    "censor_marks",
    // intervals
    "errorbar",
    "errorbarh",
    "linerange",
    "pointrange",
    "crossbar",
    "xrange",
    // business charts
    "waterfall",
    "pareto",
    "funnel",
    "pyramid",
    "lollipop",
    "dumbbell",
    "bullet",
    "bullet_vertical",
    "marimekko",
    "variwide",
    "waffle",
    "parliament",
    "tilemap",
    "packed_bubble",
    "variable_pie",
    "wordcloud",
    "venn",
    "timeline",
    "calendar",
    "flags",
    "windbarb",
    // hierarchies & networks
    "treemap",
    "sunburst",
    "icicle",
    "dendrogram",
    "tree",
    "sankey",
    "chord",
    "arcdiagram",
    "network",
    "stream",
    // finance
    "candlestick",
    "ohlc",
    "hlc",
    "heikin_ashi",
    "renko",
    "point_figure",
    // spatial
    "sf",
    "sf_text",
    "sf_label",
    "flow",
];

/// `GGPlot::geom_*` builders that are deliberately not spec geoms, with the
/// reason — they take arguments a JSON layer can't express (closures, extra
/// tables) or duplicate a spec geom. Kept honest by a test.
pub const EXCLUDED_GEOMS: &[(&str, &str)] = &[
    (
        "bracket",
        "annotation with literal positions; use labels/annotate in Rust",
    ),
    (
        "bracket_many",
        "annotation with literal positions; use labels/annotate in Rust",
    ),
    ("bracket_table", "takes a second data table; Rust only"),
    (
        "hline_aes",
        "data-driven hline; use a `segment` / `hline` layer in the spec",
    ),
    (
        "vline_aes",
        "data-driven vline; use a `segment` / `vline` layer in the spec",
    ),
    (
        "abline_aes",
        "data-driven abline; use an `abline` layer in the spec",
    ),
];

/// Every geom name the spec accepts in this build: [`GEOMS`] plus the
/// feature-gated ones (`quantile` with `regression`).
pub fn geoms() -> Vec<&'static str> {
    #[allow(unused_mut)]
    let mut g = GEOMS.to_vec();
    #[cfg(feature = "regression")]
    g.push("quantile");
    g
}

fn projection(p: &LayerParams) -> Result<SfProjection, String> {
    p.projection
        .as_deref()
        .map(SfProjection::parse)
        .transpose()
        .map(Option::unwrap_or_default)
}

/// Add the layer `geom` (a spec geom name) to `plot`, configured by `p`.
pub fn add_geom(plot: GGPlot, geom: &str, p: &LayerParams) -> Result<GGPlot, String> {
    let need = |v: Option<f64>, k: &str| v.ok_or(format!("geom '{geom}' needs params.{k}"));
    let need_str =
        |v: &Option<String>, k: &str| v.clone().ok_or(format!("geom '{geom}' needs params.{k}"));
    let bullet = || {
        let mut g = crate::geom::bullet::GeomBullet::default();
        if let Some(t) = &p.target {
            g = g.with_target(t);
        }
        if let Some(r) = &p.ranges {
            g = g.with_ranges(r.clone());
        }
        g
    };
    Ok(match geom {
        "point" => plot.geom_point(),
        "jitter" => plot.geom_jitter(),
        "line" => plot.geom_line(),
        "path" => plot.geom_path(),
        "step" => plot.geom_step(),
        "area" => plot.geom_area(),
        "ribbon" => plot.geom_ribbon(),
        "stepribbon" => plot.geom_stepribbon(),
        "col" => plot.geom_col(),
        "bar" => plot.geom_bar(),
        "segment" => plot.geom_segment(),
        "curve" => plot.geom_curve(),
        "spoke" => plot.geom_spoke(),
        "rect" => plot.geom_rect(),
        "tile" => plot.geom_tile(),
        "raster" => plot.geom_raster(),
        "polygon" => plot.geom_polygon(),
        "text" => plot.geom_text(),
        "label" => plot.geom_label(),
        "text_repel" => plot.geom_text_repel(),
        "label_repel" => plot.geom_label_repel(),
        "rug" => plot.geom_rug(),
        "blank" => plot.geom_blank(),
        "hline" => plot.geom_hline(need(p.yintercept, "yintercept")?),
        "vline" => plot.geom_vline(need(p.xintercept, "xintercept")?),
        "abline" => plot.geom_abline(need(p.slope, "slope")?, p.intercept.unwrap_or(0.0)),
        "histogram" => match p.bins {
            Some(b) => plot.geom_histogram_with(GeomHistogram::default().with_bins(b.max(1))),
            None => plot.geom_histogram(),
        },
        "freqpoly" => plot.geom_freqpoly(),
        "density" => plot.geom_density(),
        "density_ridges" => plot.geom_density_ridges(),
        "boxplot" => {
            let d = crate::geom::boxplot::GeomBoxplot::default();
            plot.geom_boxplot_with(crate::geom::boxplot::GeomBoxplot {
                notch: p.notch.unwrap_or(false),
                varwidth: p.varwidth.unwrap_or(false),
                coef: p.coef.unwrap_or(1.5),
                outliers: p.outliers.unwrap_or(true),
                staple_width: p.staplewidth.unwrap_or(d.staple_width),
                ..d
            })
        }
        "violin" => {
            use crate::geom::violin::{GeomViolin, ViolinScale};
            let mut g = GeomViolin {
                trim: p.trim.unwrap_or(true),
                bw: p.bw,
                adjust: p.adjust.unwrap_or(1.0),
                quantiles: p.quantiles.clone().unwrap_or_default(),
                ..Default::default()
            };
            g.scale = match p.scale.as_deref() {
                None | Some("area") => ViolinScale::Area,
                Some("count") => ViolinScale::Count,
                Some("width") => ViolinScale::Width,
                Some(o) => {
                    return Err(format!(
                        "unknown violin scale '{o}' (expected area, count, width)"
                    ))
                }
            };
            plot.geom_violin_with(g)
        }
        "dotplot" => plot.geom_dotplot(),
        "smooth" => {
            let mut g = GeomSmooth::default();
            match p.method.as_deref() {
                None | Some("lm") => {}
                Some("loess") => g = g.loess(p.span.unwrap_or(0.75)),
                Some(m) => return Err(format!("unknown smooth method '{m}' (expected lm, loess)")),
            }
            if let Some(se) = p.se {
                g.se = se;
            }
            plot.geom_smooth_with(g)
        }
        "count" => plot.geom_count(),
        "bin2d" => plot.geom_bin2d(),
        "hex" => plot.geom_hex(),
        "density2d" => plot.geom_density2d(),
        "contour" => plot.geom_contour(),
        "contour_filled" => plot.geom_contour_filled(),
        "qq" => plot.geom_qq(),
        "qq_line" => plot.geom_qq_line(),
        "qq_band" => plot.geom_qq_band(),
        "censor_marks" => plot.geom_censor_marks(&need_str(&p.censor, "censor")?),
        #[cfg(feature = "regression")]
        "quantile" => plot.geom_quantile(),
        "errorbar" => plot.geom_errorbar(),
        "errorbarh" => plot.geom_errorbarh(),
        "linerange" => plot.geom_linerange(),
        "pointrange" => plot.geom_pointrange(),
        "crossbar" => plot.geom_crossbar(),
        "xrange" => plot.geom_xrange(),
        "waterfall" => plot.geom_waterfall(),
        "pareto" => plot.geom_pareto(),
        "funnel" => plot.geom_funnel(),
        "pyramid" => plot.geom_pyramid(),
        "lollipop" => plot.geom_lollipop(),
        "dumbbell" => plot.geom_dumbbell(),
        "bullet" => plot.geom_bullet(bullet()),
        "bullet_vertical" => plot.geom_bullet_vertical(bullet()),
        "marimekko" => plot.geom_marimekko(),
        "variwide" => plot.geom_variwide(),
        "waffle" => plot.geom_waffle(p.rows.unwrap_or(10).max(1)),
        "parliament" => plot.geom_parliament(p.rows.unwrap_or(6).max(1)),
        "tilemap" => {
            use crate::geom::tilemap::TileShape;
            plot.geom_tilemap(match p.shape.as_deref() {
                None | Some("square") => TileShape::Square,
                Some("hexagon") => TileShape::Hexagon,
                Some("circle") => TileShape::Circle,
                Some("diamond") => TileShape::Diamond,
                Some(o) => {
                    return Err(format!(
                        "unknown tilemap shape '{o}' (expected square, hexagon, circle, diamond)"
                    ))
                }
            })
        }
        "packed_bubble" => plot.geom_packed_bubble(),
        "variable_pie" => plot.geom_variable_pie(),
        "wordcloud" => plot.geom_wordcloud(),
        "venn" => plot.geom_venn(),
        "timeline" => plot.geom_timeline(),
        "calendar" => plot.geom_calendar(),
        "flags" => plot.geom_flags(),
        "windbarb" => plot.geom_windbarb(crate::geom::windbarb::GeomWindbarb::new(
            need_str(&p.speed, "speed")?,
            need_str(&p.direction, "direction")?,
        )),
        "treemap" => plot.geom_treemap(),
        "sunburst" | "icicle" => {
            let levels = p.levels.clone().filter(|l| !l.is_empty()).ok_or(format!(
                "geom '{geom}' needs params.levels (hierarchy columns)"
            ))?;
            if geom == "sunburst" {
                plot.geom_sunburst(levels)
            } else {
                plot.geom_icicle(levels)
            }
        }
        "dendrogram" => plot.geom_dendrogram(),
        "tree" => plot.geom_tree(),
        "sankey" => plot.geom_sankey(),
        "chord" => plot.geom_chord(),
        "arcdiagram" => plot.geom_arcdiagram(),
        "network" => plot.geom_network(),
        "stream" => plot.geom_stream(),
        "candlestick" => plot.geom_candlestick(),
        "ohlc" => plot.geom_ohlc(),
        "hlc" => plot.geom_hlc(),
        "heikin_ashi" => plot.geom_heikin_ashi(),
        "renko" => plot.geom_renko(need(p.box_size, "box_size")?),
        "point_figure" => {
            plot.geom_point_figure(need(p.box_size, "box_size")?, p.reversal.unwrap_or(3))
        }
        "sf" => plot.geom_sf_with(crate::geom::sf::GeomSf::default().project(projection(p)?)),
        "sf_text" => plot
            .geom_sf_text_with(crate::geom::sf_text::GeomSfText::default().project(projection(p)?)),
        "sf_label" => plot.geom_sf_label_with(
            crate::geom::sf_text::GeomSfLabel::default().project(projection(p)?),
        ),
        "flow" => plot.geom_flow(crate::geom::flow::GeomFlow::default().project(projection(p)?)),
        other => {
            return Err(format!(
                "unknown geom '{other}' (expected one of: {})",
                geoms().join(", ")
            ))
        }
    })
}

/// Apply a spec position to the last-added layer.
pub fn apply_position(plot: GGPlot, p: &PositionSpec) -> Result<GGPlot, String> {
    use crate::position as pos;
    let (kind, full) = match p {
        PositionSpec::Name(n) => (n.as_str(), None),
        PositionSpec::Full(f) => (f.kind.as_str(), Some(f)),
    };
    let get = |f: fn(&PositionFull) -> Option<f64>, d: f64| full.and_then(f).unwrap_or(d);
    let single = match full.and_then(|f| f.preserve.as_deref()) {
        None | Some("total") => false,
        Some("single") => true,
        Some(o) => return Err(format!("unknown preserve '{o}' (expected total, single)")),
    };
    Ok(match kind {
        "identity" => plot.position(pos::identity::PositionIdentity),
        "stack" => plot.position(pos::stack::position_stack()),
        "fill" => plot.position(pos::fill::position_fill()),
        "dodge" => {
            let mut d = pos::dodge::position_dodge(get(|f| f.width, 0.9));
            d.preserve_single = single;
            plot.position(d)
        }
        "dodge2" => {
            let mut d = pos::dodge2::PositionDodge2::new(get(|f| f.padding, 0.1));
            d.preserve_single = single;
            plot.position(d)
        }
        "jitter" => plot.position(pos::jitter::PositionJitter::new(
            get(|f| f.width, 0.4),
            get(|f| f.height, 0.0),
        )),
        "nudge" => plot.position(pos::nudge::PositionNudge::new(
            get(|f| f.x, 0.0),
            get(|f| f.y, 0.0),
        )),
        other => {
            return Err(format!(
                "unknown position '{other}' (expected identity, stack, fill, dodge, dodge2, \
                 jitter, nudge)"
            ))
        }
    })
}

fn position_scale(s: &PositionScaleSpec) -> Result<ScaleContinuous, String> {
    let mut sc = ScaleContinuous::new();
    if let Some(t) = &s.trans {
        let t = match t.as_str() {
            "identity" => ScaleTransform::Identity,
            "log10" => ScaleTransform::Log10,
            "log2" => ScaleTransform::Log2,
            "ln" | "log" => ScaleTransform::Ln,
            "sqrt" => ScaleTransform::Sqrt,
            "reverse" => ScaleTransform::Reverse,
            "logit" => ScaleTransform::Logit,
            "probit" => ScaleTransform::Probit,
            "pseudo_log" => ScaleTransform::PseudoLog,
            "reciprocal" => ScaleTransform::Reciprocal,
            "exp" => ScaleTransform::Exp,
            other => return Err(format!("unknown trans '{other}'")),
        };
        sc = sc.with_transform(t);
    }
    if let Some([lo, hi]) = s.limits {
        sc = sc.with_limits(lo, hi);
    }
    if let Some(b) = &s.breaks {
        sc = sc.with_breaks(b.clone());
    }
    if let Some(n) = &s.name {
        sc = sc.with_name(n);
    }
    Ok(sc)
}

/// `"#rgb"` / `"#rrggbb"` → color.
fn parse_hex(s: &str) -> Result<RGBAColor, String> {
    let h = s.strip_prefix('#').unwrap_or(s);
    let h: String = if h.len() == 3 {
        h.chars().flat_map(|c| [c, c]).collect()
    } else {
        h.to_string()
    };
    let byte = |i: usize| {
        h.get(i..i + 2)
            .and_then(|b| u8::from_str_radix(b, 16).ok())
            .ok_or(format!("bad color '{s}' (expected #rrggbb)"))
    };
    if h.len() != 6 {
        return Err(format!("bad color '{s}' (expected #rrggbb)"));
    }
    Ok(RGBAColor::new(byte(0)?, byte(2)?, byte(4)?))
}

fn apply_color_scale(
    plot: GGPlot,
    aes: Aesthetic,
    s: &ColorScaleSpec,
    discrete: bool,
) -> Result<GGPlot, String> {
    if let Some(values) = &s.values {
        let parsed: Vec<(String, RGBAColor)> = values
            .iter()
            .map(|(k, v)| parse_hex(v).map(|c| (k.clone(), c)))
            .collect::<Result<_, _>>()?;
        let refs: Vec<(&str, RGBAColor)> = parsed.iter().map(|(k, c)| (k.as_str(), *c)).collect();
        return Ok(if aes == Aesthetic::Color {
            plot.scale_color_manual(refs)
        } else {
            plot.scale_fill_manual(refs)
        });
    }
    if let Some([lo, hi]) = &s.gradient {
        let (lo, hi) = (parse_hex(lo)?, parse_hex(hi)?);
        return Ok(if aes == Aesthetic::Color {
            plot.scale_color_gradient(lo, hi)
        } else {
            plot.scale_fill_gradient(lo, hi)
        });
    }
    let palette = s.palette.as_deref();
    if !discrete {
        let g = match palette.map(str::to_lowercase).as_deref() {
            None | Some("viridis") => ScaleColorGradientN::viridis(aes.clone()),
            Some("magma") => ScaleColorGradientN::magma(aes.clone()),
            Some("plasma") => ScaleColorGradientN::plasma(aes.clone()),
            Some("inferno") => ScaleColorGradientN::inferno(aes.clone()),
            Some(other) => {
                return Err(format!(
                    "palette '{other}' is discrete but the mapped column is numeric (use \
                     viridis, magma, plasma, inferno, or gradient)"
                ))
            }
        };
        return Ok(if aes == Aesthetic::Color {
            plot.scale_color(g)
        } else {
            plot.scale_fill(g)
        });
    }
    let mut d = ScaleColorDiscrete::new(aes.clone());
    if let Some(name) = palette {
        d = d.with_named_palette(&parse_palette(name)?);
    }
    if let Some(lv) = &s.levels {
        d = d.with_levels(lv.clone());
    }
    Ok(if aes == Aesthetic::Color {
        plot.scale_color(d)
    } else {
        plot.scale_fill(d)
    })
}

fn apply_facet(plot: GGPlot, f: &FacetSpec) -> Result<GGPlot, String> {
    let scales = match f.scales.as_deref() {
        None | Some("fixed") => FacetScales::Fixed,
        Some("free") => FacetScales::Free,
        Some("free_x") => FacetScales::FreeX,
        Some("free_y") => FacetScales::FreeY,
        Some(o) => {
            return Err(format!(
                "unknown scales '{o}' (expected fixed, free, free_x, free_y)"
            ))
        }
    };
    match (&f.wrap, &f.rows, &f.cols) {
        (Some(w), None, None) => Ok(plot.facet_wrap_free(w, f.ncol, scales)),
        (None, r, c) if r.is_some() || c.is_some() => {
            Ok(plot.facet_grid_free(r.as_deref(), c.as_deref(), scales))
        }
        _ => Err("set either `wrap` or `rows`/`cols`".into()),
    }
}

fn apply_coord(plot: GGPlot, c: &CoordSpec) -> Result<GGPlot, String> {
    let (kind, full) = match c {
        CoordSpec::Name(n) => (n.as_str(), None),
        CoordSpec::Full(f) => (f.kind.as_str(), Some(f)),
    };
    Ok(match kind {
        "cartesian" => {
            let xl = full.and_then(|f| f.xlim).map(|[a, b]| (a, b));
            let yl = full.and_then(|f| f.ylim).map(|[a, b]| (a, b));
            if xl.is_some() || yl.is_some() {
                plot.coord_cartesian_zoom(xl, yl)
            } else {
                plot
            }
        }
        "flip" => plot.coord_flip(),
        "polar" => plot.coord_polar(),
        "fixed" | "equal" => plot.coord_fixed(full.and_then(|f| f.ratio).unwrap_or(1.0)),
        "radar" => plot.coord_radar(),
        "sf" => plot.coord_sf(),
        other => {
            return Err(format!(
                "unknown coord '{other}' (expected cartesian, flip, polar, fixed, radar, sf)"
            ))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(spec: &str) -> Result<String, String> {
        render_json(spec).map(|(svg, _)| svg)
    }

    const DATA: &str =
        r#""data": {"x": [1, 2, 3, 4], "y": [2, 4, 3, 5], "g": ["a", "a", "b", "b"]}"#;

    #[test]
    fn minimal_point_plot_renders_svg() {
        let svg = render(&format!(
            r#"{{ {DATA}, "aes": {{"x": "x", "y": "y"}}, "layers": [{{"geom": "point"}}] }}"#
        ))
        .unwrap();
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("<circle"));
    }

    #[test]
    fn layered_spec_with_everything() {
        let svg = render(&format!(
            r##"{{ {DATA}, "aes": {{"x": "x", "y": "y", "color": "g"}},
               "layers": [{{"geom": "point"}}, {{"geom": "smooth", "params": {{"se": false}}}},
                          {{"geom": "hline", "params": {{"yintercept": 3}}}}],
               "scales": {{"color": {{"palette": "Dark2"}}, "y": {{"limits": [0, 6]}}}},
               "facet": {{"wrap": "g"}}, "labels": {{"title": "Hello", "x": "X axis"}},
               "theme": {{"base": "bw", "primary": [10, 20, 30]}}, "legend": "bottom",
               "width": 500, "height": 300 }}"##
        ))
        .unwrap();
        assert!(svg.contains("Hello"));
        assert!(svg.contains("X axis"));
        assert!(svg.contains(r#"width="500""#));
    }

    #[test]
    fn unknown_column_lists_available() {
        let e = render(&format!(
            r#"{{ {DATA}, "aes": {{"x": "nope"}}, "layers": [{{"geom": "point"}}] }}"#
        ))
        .unwrap_err();
        assert!(e.contains("column 'nope' not found"), "{e}");
        assert!(e.contains("available: g, x, y"), "{e}");
    }

    #[test]
    fn unknown_geom_aes_and_field_are_reported() {
        let e = render(&format!(
            r#"{{ {DATA}, "layers": [{{"geom": "pointy"}}] }}"#
        ))
        .unwrap_err();
        assert!(e.contains("unknown geom 'pointy'"), "{e}");
        let e = render(&format!(
            r#"{{ {DATA}, "aes": {{"colr": "g"}}, "layers": [{{"geom": "point"}}] }}"#
        ))
        .unwrap_err();
        assert!(e.contains("unknown aesthetic 'colr'"), "{e}");
        let e = render(&format!(r#"{{ {DATA}, "layer": [] }}"#)).unwrap_err();
        assert!(e.contains("invalid plot spec"), "{e}");
    }

    #[test]
    fn every_listed_geom_is_recognised() {
        let p = LayerParams {
            yintercept: Some(1.0),
            xintercept: Some(1.0),
            slope: Some(1.0),
            levels: Some(vec!["g".into()]),
            box_size: Some(1.0),
            speed: Some("x".into()),
            direction: Some("x".into()),
            censor: Some("x".into()),
            ..Default::default()
        };
        for g in geoms() {
            let plot = GGPlot::new(vec![("x".to_string(), vec![Value::Float(1.0)])]);
            assert!(add_geom(plot, g, &p).is_ok(), "{g}");
        }
    }

    /// Every `pub fn geom_*` builder in the crate (minus `*_with` variants)
    /// must be a spec geom or an explained exclusion — so a new geom can't
    /// land in Rust without reaching the CLI and the npm package.
    #[test]
    fn every_builder_geom_is_in_the_spec() {
        fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for e in std::fs::read_dir(dir).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    out.push(p);
                }
            }
        }
        let mut files = Vec::new();
        walk(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &mut files,
        );
        let mut builders = std::collections::BTreeSet::new();
        for f in files {
            let text = std::fs::read_to_string(&f).unwrap();
            for line in text.lines() {
                let Some(rest) = line.trim_start().strip_prefix("pub fn geom_") else {
                    continue;
                };
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                if !name.ends_with("_with") {
                    builders.insert(name);
                }
            }
        }
        assert!(
            builders.len() > 50,
            "scan found too few builders: {builders:?}"
        );
        let excluded: Vec<&str> = EXCLUDED_GEOMS.iter().map(|(g, _)| *g).collect();
        let feature_gated = ["quantile"];
        let missing: Vec<&String> = builders
            .iter()
            .filter(|b| {
                !GEOMS.contains(&b.as_str())
                    && !excluded.contains(&b.as_str())
                    && !feature_gated.contains(&b.as_str())
            })
            .collect();
        assert!(
            missing.is_empty(),
            "GGPlot::geom_* builders missing from spec::GEOMS (add them to add_geom, or to \
             EXCLUDED_GEOMS with a reason): {missing:?}"
        );
        // Aliases of other geoms (icicle → sunburst-family) are fine; but no
        // spec geom may also be excluded, and no duplicates.
        for g in GEOMS {
            assert!(
                !excluded.contains(g),
                "{g} is both a spec geom and excluded"
            );
        }
        let uniq: std::collections::BTreeSet<_> = GEOMS.iter().collect();
        assert_eq!(uniq.len(), GEOMS.len(), "duplicate entry in GEOMS");
    }

    #[test]
    fn projection_names_parse() {
        for ok in [
            "platecarree",
            "plate",
            "mercator",
            "equal-earth",
            "equalearth",
            "albers-usa",
            "albers-europe",
            "utm:33",
            "utm:33s",
        ] {
            assert!(SfProjection::parse(ok).is_ok(), "{ok}");
        }
        assert_eq!(
            SfProjection::parse("utm:33s"),
            Ok(SfProjection::utm(33, true))
        );
        for (bad, msg) in [
            ("utm:0", "1-60"),
            ("utm:abc", "bad UTM zone"),
            ("robinson", "unknown projection"),
        ] {
            let e = SfProjection::parse(bad).unwrap_err();
            assert!(e.contains(msg), "{bad}: {e}");
        }
    }

    #[test]
    fn overlay_copies_only_set_params() {
        let mut base = LayerParams {
            bins: Some(5),
            method: Some("lm".into()),
            ..Default::default()
        };
        base.overlay(LayerParams {
            method: Some("loess".into()),
            rows: Some(3),
            ranges: Some(vec![1.0]),
            ..Default::default()
        });
        assert_eq!(base.bins, Some(5));
        assert_eq!(base.method.as_deref(), Some("loess"));
        assert_eq!(base.rows, Some(3));
        assert_eq!(base.ranges, Some(vec![1.0]));
    }

    #[test]
    fn build_with_caller_data_and_spatial_geoms() {
        let spec = parse(
            r#"{"aes": {"fill": "v"},
                "layers": [{"geom": "sf", "params": {"projection": "mercator"}},
                           {"geom": "sf_text", "aes": {"label": "n"}},
                           {"geom": "sf_label", "aes": {"label": "n"}}],
                "coord": "sf"}"#,
        )
        .unwrap();
        let data = vec![
            (
                "geometry".to_string(),
                vec![Value::Str("POLYGON ((0 0, 10 0, 10 10, 0 10, 0 0))".into())],
            ),
            ("v".to_string(), vec![Value::Float(1.0)]),
            ("n".to_string(), vec![Value::Str("a".into())]),
        ];
        let svg = build_plot_with_data(&spec, data)
            .unwrap()
            .render_svg_native_with_size(300, 200)
            .unwrap();
        assert!(svg.starts_with("<svg"));
        let e = render(
            r#"{"data": {"geometry": ["POINT (1 1)"]},
                "layers": [{"geom": "sf", "params": {"projection": "robinson"}}]}"#,
        )
        .unwrap_err();
        assert!(e.contains("unknown projection"), "{e}");
    }

    #[test]
    fn parameterised_geoms_validate_params() {
        let plot = || GGPlot::new(vec![("x".to_string(), vec![Value::Float(1.0)])]);
        for shape in ["square", "hexagon", "circle", "diamond"] {
            let p = LayerParams {
                shape: Some(shape.into()),
                ..Default::default()
            };
            assert!(add_geom(plot(), "tilemap", &p).is_ok(), "{shape}");
        }
        let p = LayerParams {
            shape: Some("star".into()),
            ..Default::default()
        };
        assert!(add_geom(plot(), "tilemap", &p).is_err());
        let p = LayerParams {
            target: Some("x".into()),
            ranges: Some(vec![1.0, 2.0]),
            ..Default::default()
        };
        assert!(add_geom(plot(), "bullet", &p).is_ok());
        assert!(add_geom(plot(), "bullet_vertical", &p).is_ok());
        for (g, need) in [
            ("renko", "box_size"),
            ("point_figure", "box_size"),
            ("windbarb", "speed"),
            ("censor_marks", "censor"),
            ("hline", "yintercept"),
        ] {
            let Err(e) = add_geom(plot(), g, &LayerParams::default()) else {
                panic!("{g} should need {need}");
            };
            assert!(e.contains(need), "{g}: {e}");
        }
        let p = LayerParams {
            method: Some("gam".into()),
            ..Default::default()
        };
        assert!(add_geom(plot(), "smooth", &p).is_err());
    }

    #[test]
    fn grouped_boxplot_with_preserve_single_renders() {
        let svg = render(&format!(
            r#"{{ {DATA}, "aes": {{"x": "g", "y": "y", "fill": "g"}},
               "layers": [{{"geom": "boxplot", "position": {{"type": "dodge2", "preserve": "single"}}}}] }}"#
        ))
        .unwrap();
        assert!(svg.contains("<rect"));
        let e = render(&format!(
            r#"{{ {DATA}, "aes": {{"x": "g", "y": "y"}},
               "layers": [{{"geom": "col", "position": {{"type": "dodge", "preserve": "some"}}}}] }}"#
        ))
        .unwrap_err();
        assert!(e.contains("unknown preserve"), "{e}");
    }

    #[test]
    fn boxplot_and_violin_params() {
        let svg = render(&format!(
            r#"{{ {DATA}, "aes": {{"x": "g", "y": "y"}},
               "layers": [{{"geom": "violin", "params": {{"scale": "count", "trim": false,
                            "quantiles": [0.25, 0.5, 0.75], "adjust": 1.5, "bw": 0.5}}}},
                          {{"geom": "boxplot", "params": {{"notch": true, "varwidth": true,
                            "coef": 1.0, "outliers": false, "staplewidth": 0}}}}] }}"#
        ))
        .unwrap();
        assert!(svg.contains("<polygon"));
        let e = render(&format!(
            r#"{{ {DATA}, "aes": {{"x": "g", "y": "y"}},
               "layers": [{{"geom": "violin", "params": {{"scale": "big"}}}}] }}"#
        ))
        .unwrap_err();
        assert!(e.contains("unknown violin scale"), "{e}");
    }

    #[test]
    fn size_and_empty_layers_are_validated() {
        let e = render(&format!(r#"{{ {DATA}, "layers": [], "width": 0 }}"#)).unwrap_err();
        assert!(e.contains("positive"), "{e}");
        let e = render(&format!(r#"{{ {DATA}, "layers": [] }}"#)).unwrap_err();
        assert!(e.contains("at least one layer"), "{e}");
    }

    #[test]
    fn treemap_and_sankey_render() {
        let svg = render(
            r#"{"data": {"k": ["a", "b", "c"], "v": [3, 2, 1]},
                "aes": {"weight": "v", "label": "k", "fill": "k"},
                "layers": [{"geom": "treemap"}]}"#,
        )
        .unwrap();
        assert!(svg.contains("<svg"));
        let svg = render(
            r#"{"data": {"s": ["a", "a", "b"], "t": ["b", "c", "c"], "w": [3, 2, 1]},
                "aes": {"from": "s", "to": "t", "weight": "w"},
                "layers": [{"geom": "sankey"}]}"#,
        )
        .unwrap();
        assert!(svg.contains("<svg"));
    }

    #[test]
    fn numeric_color_with_discrete_palette_errors() {
        let e = render(&format!(
            r#"{{ {DATA}, "aes": {{"x": "x", "y": "y", "color": "y"}}, "layers": [{{"geom": "point"}}],
                 "scales": {{"color": {{"palette": "Set1"}}}} }}"#
        ))
        .unwrap_err();
        assert!(e.contains("numeric"), "{e}");
    }

    #[test]
    fn manual_values_and_coord_and_position() {
        let svg = render(&format!(
            r##"{{ {DATA}, "aes": {{"x": "g", "y": "y", "fill": "g"}},
               "layers": [{{"geom": "col", "position": {{"type": "dodge", "width": 0.8}}}}],
               "scales": {{"fill": {{"values": {{"a": "#ff0000", "b": "#00f"}}}}}},
               "coord": "flip" }}"##
        ))
        .unwrap();
        assert!(
            svg.contains("#FF0000") || svg.contains("#ff0000") || svg.contains("255,0,0"),
            "manual red missing"
        );
    }

    #[test]
    fn mismatched_column_lengths_error() {
        let e = render(r#"{"data": {"x": [1, 2], "y": [1]}, "aes": {"x": "x", "y": "y"}, "layers": [{"geom": "point"}]}"#)
            .unwrap_err();
        assert!(e.contains("equal length"), "{e}");
    }
}
