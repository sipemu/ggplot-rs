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
    /// Per-aesthetic guides: `{"fill": "none", "color": {"title": "…"}}`.
    #[serde(default)]
    pub guides: BTreeMap<String, GuideSpec>,
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
    /// Legend key picture (`"point"`, `"path"`, `"boxplot"`, …; ggplot2's
    /// `key_glyph`).
    pub key_glyph: Option<String>,
    /// Show the layer in the legends (`true`/`false`), or per aesthetic
    /// (`{"color": true, "fill": false}`).
    pub show_legend: Option<ShowLegendSpec>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum ShowLegendSpec {
    All(bool),
    Per(BTreeMap<String, bool>),
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
    /// stat `"connect"`: `"hv"` (default), `"vh"`, `"mid"`, `"linear"`.
    pub connection: Option<String>,
    /// stat `"summary_hex"`: `"mean"` (default), `"median"`, `"min"`, `"max"`,
    /// `"sum"` of the raw `z` column.
    pub fun: Option<String>,
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
            connection,
            fun,
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
            adjust,
            connection,
            fun
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
    /// #408: `{"identity": true}`, `{"values": {level: n}}` or
    /// `{"range": [lo, hi]}` (size: radius, ggplot2's `scale_radius`).
    pub size: Option<ValueScaleSpec>,
    pub alpha: Option<ValueScaleSpec>,
    pub linewidth: Option<ValueScaleSpec>,
    /// `{"identity": true}`: the column holds `pch` codes / shape names.
    pub shape: Option<ValueScaleSpec>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ValueScaleSpec {
    pub identity: Option<bool>,
    pub values: Option<BTreeMap<String, f64>>,
    pub range: Option<[f64; 2]>,
    /// Binned (`scale_*_binned(n.breaks)`, #408): about this many breaks;
    /// `range` sets the output range.
    pub binned: Option<usize>,
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
    /// About this many breaks (`n.breaks`, #408).
    pub n_breaks: Option<usize>,
    /// A time-of-day axis: seconds, `HH:MM:SS` labels (`scale_x_time`).
    pub time: Option<bool>,
    /// Binned position scale (`scale_x_binned(n.breaks)`, #408).
    pub binned: Option<usize>,
    /// A secondary axis (x: top, y: right; #409): `true` repeats the
    /// primary (`dup_axis()`), or `{ name, scale, offset }` for a linear
    /// `secondary = primary * scale + offset`.
    pub sec_axis: Option<SecAxisSpec>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum SecAxisSpec {
    Dup(bool),
    Full(SecAxisFull),
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct SecAxisFull {
    pub name: Option<String>,
    pub scale: Option<f64>,
    pub offset: Option<f64>,
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
    /// #408: diverging binned `["#low", "#mid", "#high"]` for a numeric
    /// column (`scale_*_steps2`), `mid` at `midpoint` (default 0).
    pub steps2: Option<[String; 3]>,
    pub midpoint: Option<f64>,
    /// Bin count for `steps2` / `"palette": "viridis_b:X"` (default 5).
    pub bins: Option<usize>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct FacetSpec {
    pub wrap: Option<String>,
    pub ncol: Option<usize>,
    pub rows: Option<String>,
    pub cols: Option<String>,
    /// `"fixed"` (default), `"free"`, `"free_x"`, `"free_y"`.
    pub scales: Option<String>,
    /// wrap: `"h"` (default, row by row) or `"v"` (column by column).
    pub dir: Option<String>,
    /// wrap: first row at the top (default `true`; ggplot2's `as.table`).
    pub as_table: Option<bool>,
    /// Wrap strip labels at this many characters (`label_wrap_gen(width)`).
    pub label_wrap: Option<usize>,
    /// wrap: `"top"` (default), `"bottom"`, `"left"`, `"right"` (#412).
    pub strip_position: Option<String>,
    /// grid: `"x"`, `"y"` or `"both"` — column strips below, row strips
    /// left (#412).
    pub switch: Option<String>,
    /// `"margins"` (default), `"all"`, `"all_x"`, `"all_y"`: which panels
    /// get axes (#412).
    pub axes: Option<String>,
    /// `"all"` (default) or `"margins"`: label the inner axes or not.
    pub axis_labels: Option<String>,
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
    /// `cartesian`: mirror the panel — `"x"`, `"y"` or `"xy"`.
    pub reverse: Option<String>,
    /// `cartesian`: keep the scale padding — `false` for none, or per side
    /// `[top, right, bottom, left]` (#411).
    pub expand: Option<ExpandSpec>,
    /// `radial`: theta start / end in radians (clockwise from 12 o'clock).
    pub start: Option<f64>,
    pub end: Option<f64>,
    /// `radial`: inner radius as a fraction of the outer (a donut).
    pub inner_radius: Option<f64>,
    /// `radial`: r axis along the start spoke (`r.axis.inside`, #411).
    pub r_axis_inside: Option<bool>,
    /// `radial`: theta labels follow the circle (`rotate.angle`, #411).
    pub rotate_angle: Option<bool>,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(untagged)]
pub enum ExpandSpec {
    All(bool),
    Sides([bool; 4]),
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
pub enum GuideSpec {
    /// `"none"` drops the legend.
    Name(String),
    Full(Box<GuideFull>),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuideFull {
    pub title: Option<String>,
    /// `"legend"` (default), `"colourbar"` / `"colorbar"`,
    /// `"coloursteps"` / `"colorsteps"` (#409).
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub reverse: Option<bool>,
    pub barwidth: Option<f64>,
    pub barheight: Option<f64>,
    pub nbin: Option<usize>,
    /// `"gradient"` or `"rectangles"`.
    pub display: Option<String>,
    pub frame: Option<bool>,
    pub ticks: Option<bool>,
    pub even_steps: Option<bool>,
    pub show_limits: Option<bool>,
    /// Fixed key appearance (`override.aes`, #409).
    pub override_aes: Option<OverrideSpec>,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct OverrideSpec {
    pub color: Option<String>,
    pub fill: Option<String>,
    /// A shape name (`"square"`) or ggplot2 `pch` code.
    pub shape: Option<serde_json::Value>,
    pub linetype: Option<String>,
    pub size: Option<f64>,
    pub alpha: Option<f64>,
    pub linewidth: Option<f64>,
}

/// The `GuideColourbar` a `"colourbar"` / `"coloursteps"` guide spec means
/// (`None` for a plain legend with no colourbar option set).
fn colourbar_spec(
    name: &str,
    kind: Option<&str>,
    g: Option<&GuideFull>,
) -> Result<Option<crate::guide::config::GuideColourbar>, String> {
    use crate::guide::config::{ColourbarDisplay, GuideColourbar};
    let mut bar = match kind {
        None | Some("legend") => {
            let has_bar_opt = g.is_some_and(|g| {
                g.barwidth.is_some()
                    || g.barheight.is_some()
                    || g.nbin.is_some()
                    || g.display.is_some()
                    || g.frame.is_some()
                    || g.ticks.is_some()
                    || g.even_steps.is_some()
                    || g.show_limits.is_some()
            });
            if !has_bar_opt {
                return Ok(None);
            }
            GuideColourbar::new()
        }
        Some("colourbar" | "colorbar") => GuideColourbar::new(),
        Some("coloursteps" | "colorsteps") => GuideColourbar::steps(),
        Some(o) => {
            return Err(format!(
                "guides.{name}: unknown guide '{o}' (expected none, legend, colourbar, coloursteps)"
            ))
        }
    };
    let Some(g) = g else {
        return Ok(Some(bar));
    };
    if let Some(w) = g.barwidth {
        bar = bar.with_bar_width(w);
    }
    if let Some(h) = g.barheight {
        bar = bar.with_bar_height(h);
    }
    if let Some(n) = g.nbin {
        bar = bar.with_nbin(n);
    }
    if let Some(d) = &g.display {
        bar = bar.with_display(match d.as_str() {
            "gradient" => ColourbarDisplay::Gradient,
            "rectangles" => ColourbarDisplay::Rectangles,
            o => {
                return Err(format!(
                    "guides.{name}.display: unknown '{o}' (expected gradient, rectangles)"
                ))
            }
        });
    }
    if let Some(f) = g.frame {
        bar = bar.with_frame(f);
    }
    if let Some(t) = g.ticks {
        bar = bar.with_ticks(t);
    }
    if let Some(e) = g.even_steps {
        bar = bar.with_even_steps(e);
    }
    if let Some(l) = g.show_limits {
        bar = bar.with_show_limits(l);
    }
    Ok(Some(bar))
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
            plot =
                apply_stat(plot, s, &layer.params).map_err(|e| format!("layers[{i}].stat: {e}"))?;
        }
        if let Some(p) = &layer.position {
            plot = apply_position(plot, p).map_err(|e| format!("layers[{i}].position: {e}"))?;
        }
        if let Some(k) = &layer.key_glyph {
            let g = crate::guide::key::KeyGlyph::parse(k).ok_or(format!(
                "layers[{i}].key_glyph: unknown glyph '{k}' (expected point, path, rect, polygon, \
                 boxplot, crossbar, pointrange, linerange, smooth, text, label, vline, vpath, \
                 abline, timeseries, dotplot, blank)"
            ))?;
            plot = plot.key_glyph(g);
        }
        match &layer.show_legend {
            None => {}
            Some(ShowLegendSpec::All(b)) => plot = plot.show_legend(*b),
            Some(ShowLegendSpec::Per(m)) => {
                for (aes, show) in m {
                    let a = match aes.as_str() {
                        "color" | "colour" => Aesthetic::Color,
                        "fill" => Aesthetic::Fill,
                        "shape" => Aesthetic::Shape,
                        "size" => Aesthetic::Size,
                        "alpha" => Aesthetic::Alpha,
                        "linetype" => Aesthetic::Linetype,
                        "linewidth" => Aesthetic::Linewidth,
                        o => {
                            return Err(format!(
                                "layers[{i}].show_legend: '{o}' is not a legend aesthetic"
                            ))
                        }
                    };
                    plot = plot.show_legend_for(a, *show);
                }
            }
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
    for (name, aes, v) in [
        ("size", Aesthetic::Size, &spec.scales.size),
        ("alpha", Aesthetic::Alpha, &spec.scales.alpha),
        ("linewidth", Aesthetic::Linewidth, &spec.scales.linewidth),
        ("shape", Aesthetic::Shape, &spec.scales.shape),
    ] {
        if let Some(v) = v {
            plot = apply_value_scale(plot, aes, v).map_err(|e| format!("scales.{name}: {e}"))?;
        }
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
            format!("theme: {e} (expected gray, bw, minimal, classic, dark, light, void, linedraw, test)")
        })?),
        Some(ThemeSpec::Config(cfg)) => plot.theme(
            cfg.apply(crate::theme::presets::theme_minimal())
                .map_err(|e| format!("theme: {e}"))?,
        ),
    };
    for (name, g) in &spec.guides {
        let aes = match name.as_str() {
            "color" | "colour" => Aesthetic::Color,
            "fill" => Aesthetic::Fill,
            "shape" => Aesthetic::Shape,
            "size" => Aesthetic::Size,
            "alpha" => Aesthetic::Alpha,
            "linetype" => Aesthetic::Linetype,
            "linewidth" => Aesthetic::Linewidth,
            o => return Err(format!("guides.{o}: not a legend aesthetic")),
        };
        plot = match g {
            GuideSpec::Name(n) if n == "none" => plot.guide_none(aes),
            GuideSpec::Name(n) => match colourbar_spec(name, Some(n), None)? {
                Some(bar) => plot.guide_colourbar(aes, bar),
                None => plot,
            },
            GuideSpec::Full(g) => {
                if let Some(t) = &g.title {
                    plot = plot.guide_title(aes.clone(), t);
                }
                if let Some(r) = g.reverse {
                    plot = plot.guide_reverse(aes.clone(), r);
                }
                if let Some(o) = &g.override_aes {
                    let mut k = crate::guide::config::KeyOverride::new();
                    let rgb = |c: &str| parse_hex(c).map(|c| (c.r, c.g, c.b));
                    if let Some(c) = &o.color {
                        k = k.color(rgb(c).map_err(|e| format!("guides.{name}: {e}"))?);
                    }
                    if let Some(c) = &o.fill {
                        k = k.fill(rgb(c).map_err(|e| format!("guides.{name}: {e}"))?);
                    }
                    if let Some(v) = &o.shape {
                        let value = match v {
                            serde_json::Value::String(s) => Value::Str(s.clone()),
                            serde_json::Value::Number(n) => {
                                Value::Float(n.as_f64().unwrap_or(-1.0))
                            }
                            _ => Value::Na,
                        };
                        let shape = crate::scale::values::shape_from_value(&value)
                            .ok_or(format!("guides.{name}.override_aes: unknown shape {v}"))?;
                        k = k.shape(shape);
                    }
                    if let Some(l) = &o.linetype {
                        k = k.linetype(
                            crate::theme::config::parse_linetype(l)
                                .map_err(|e| format!("guides.{name}: {e}"))?,
                        );
                    }
                    if let Some(v) = o.size {
                        k = k.size(v);
                    }
                    if let Some(v) = o.alpha {
                        k = k.alpha(v);
                    }
                    if let Some(v) = o.linewidth {
                        k = k.linewidth(v);
                    }
                    plot = plot.guide_override(aes.clone(), k);
                }
                match colourbar_spec(name, g.kind.as_deref(), Some(g.as_ref()))? {
                    Some(bar) => plot.guide_colourbar(aes, bar),
                    None => plot,
                }
            }
        };
    }
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
    (
        "function",
        "plots a Rust closure; send the evaluated points as data instead",
    ),
    (
        "function_xlim",
        "plots a Rust closure; send the evaluated points as data instead",
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

/// Stat names `layers[].stat` accepts (other stats follow from the geom).
pub const STATS: &[&str] = &[
    "identity",
    "unique",
    "connect",
    "summary_hex",
    "sf_coordinates",
];

/// Override the last-added layer's stat with the spec stat `name`.
pub fn apply_stat(plot: GGPlot, name: &str, p: &LayerParams) -> Result<GGPlot, String> {
    use crate::stat::connect::{Connection, StatConnect};
    use crate::stat::summary::SummaryFun;
    Ok(match name {
        "identity" => plot.stat(crate::stat::identity::StatIdentity),
        "unique" => plot.stat(crate::stat::unique::StatUnique),
        "connect" => plot.stat(StatConnect::new(match p.connection.as_deref() {
            None | Some("hv") => Connection::Hv,
            Some("vh") => Connection::Vh,
            Some("mid") => Connection::Mid,
            Some("linear") => Connection::Linear,
            Some(o) => {
                return Err(format!(
                    "unknown connection '{o}' (expected hv, vh, mid, linear)"
                ))
            }
        })),
        "summary_hex" => plot.stat(crate::stat::binhex::StatSummaryHex::new(
            match p.fun.as_deref() {
                None | Some("mean") => SummaryFun::Mean,
                Some("median") => SummaryFun::Median,
                Some("min") => SummaryFun::Min,
                Some("max") => SummaryFun::Max,
                Some("sum") => SummaryFun::Sum,
                Some(o) => {
                    return Err(format!(
                        "unknown fun '{o}' (expected mean, median, min, max, sum)"
                    ))
                }
            },
        )),
        "sf_coordinates" => plot.stat(
            crate::stat::sf_coordinates::StatSfCoordinates::default().project(projection(p)?),
        ),
        other => {
            return Err(format!(
                "unknown stat '{other}' (expected one of: {}; other stats follow from the geom)",
                STATS.join(", ")
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
    let mut sc = match (s.binned, s.time) {
        (Some(n), _) => ScaleContinuous::binned(n),
        (None, Some(true)) => ScaleContinuous::time(),
        _ => ScaleContinuous::new(),
    };
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
    if let Some(n) = s.n_breaks {
        sc = sc.with_n_breaks(n);
    }
    if let Some(n) = &s.name {
        sc = sc.with_name(n);
    }
    use crate::scale::sec_axis::SecAxis;
    match &s.sec_axis {
        None | Some(SecAxisSpec::Dup(false)) => {}
        Some(SecAxisSpec::Dup(true)) => sc = sc.with_sec_axis(SecAxis::dup()),
        Some(SecAxisSpec::Full(f)) => {
            let mut a = match (f.scale, f.offset) {
                (None, None) => SecAxis::dup(),
                (k, b) => SecAxis::linear(k.unwrap_or(1.0), b.unwrap_or(0.0)),
            };
            if let Some(n) = &f.name {
                a = a.with_name(n);
            }
            sc = sc.with_sec_axis(a);
        }
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

/// A `size` / `alpha` / `linewidth` / `shape` scale spec (#408).
fn apply_value_scale(plot: GGPlot, aes: Aesthetic, v: &ValueScaleSpec) -> Result<GGPlot, String> {
    if let Some(n) = v.binned {
        if v.identity.is_some() || v.values.is_some() {
            return Err("binned takes only range".into());
        }
        let mut b = crate::scale::values::ScaleValueBinned::new(aes, n);
        if let Some([lo, hi]) = v.range {
            b = b.with_range(lo, hi);
        }
        return Ok(plot.scale_color(b));
    }
    let set = [
        v.identity == Some(true),
        v.values.is_some(),
        v.range.is_some(),
    ]
    .iter()
    .filter(|b| **b)
    .count();
    if set != 1 {
        return Err("set exactly one of identity, values, range".into());
    }
    if v.identity == Some(true) {
        return Ok(plot.scale_color(crate::scale::values::ScaleValueIdentity::new(aes)));
    }
    if aes == Aesthetic::Shape {
        return Err("shape takes only {\"identity\": true}".into());
    }
    if let Some(values) = &v.values {
        let refs: Vec<(&str, f64)> = values.iter().map(|(k, n)| (k.as_str(), *n)).collect();
        return Ok(plot.scale_color(crate::scale::values::ScaleValueManual::new(aes, refs)));
    }
    let [lo, hi] = v.range.unwrap_or([1.0, 6.0]);
    Ok(match aes {
        Aesthetic::Size => plot.scale_radius(lo, hi),
        Aesthetic::Alpha => {
            plot.scale_alpha(crate::scale::alpha::ScaleAlphaContinuous::new().with_range(lo, hi))
        }
        _ => plot.scale_linewidth(
            crate::scale::linewidth::ScaleLinewidthContinuous::new().with_range(lo, hi),
        ),
    })
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
    let bins = s.bins.unwrap_or(5).max(1);
    if let Some([lo, mid, hi]) = &s.steps2 {
        let (lo, mid, hi) = (parse_hex(lo)?, parse_hex(mid)?, parse_hex(hi)?);
        let m = s.midpoint.unwrap_or(0.0);
        return Ok(if aes == Aesthetic::Color {
            plot.scale_color_steps2(lo, mid, hi, m, bins)
        } else {
            plot.scale_fill_steps2(lo, mid, hi, m, bins)
        });
    }
    if let Some(opt) = s
        .palette
        .as_deref()
        .and_then(|p| p.strip_prefix("viridis_b"))
    {
        let option = opt.trim_start_matches(':').chars().next().unwrap_or('D');
        return Ok(if aes == Aesthetic::Color {
            plot.scale_color_viridis_b(option, bins)
        } else {
            plot.scale_fill_viridis_b(option, bins)
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
    // #408: generated discrete palettes — "hue" (ggplot2's default colours
    // for any number of levels) and "viridis:A" … "viridis:H".
    let generated = match palette {
        Some("hue") if discrete => Some(crate::scale::color::GeneratedPalette::Hue(
            crate::scale::color::HuePalette::default(),
        )),
        Some(p) if discrete && p.starts_with("viridis:") => {
            Some(crate::scale::color::GeneratedPalette::Viridis {
                option: p[8..].chars().next().unwrap_or('D'),
                begin: 0.0,
                end: 1.0,
                direction: 1,
            })
        }
        _ => None,
    };
    if let Some(g) = generated {
        let mut d = ScaleColorDiscrete::new(aes.clone()).with_generated(g);
        if let Some(lv) = &s.levels {
            d = d.with_levels(lv.clone());
        }
        return Ok(if aes == Aesthetic::Color {
            plot.scale_color(d)
        } else {
            plot.scale_fill(d)
        });
    }
    if !discrete {
        let g = match palette.map(str::to_lowercase).as_deref() {
            None | Some("viridis") => ScaleColorGradientN::viridis(aes.clone()),
            Some("magma") => ScaleColorGradientN::magma(aes.clone()),
            Some("plasma") => ScaleColorGradientN::plasma(aes.clone()),
            Some("inferno") => ScaleColorGradientN::inferno(aes.clone()),
            // #408: a Brewer palette on a numeric column → `distiller`.
            Some(other) => match parse_palette(other) {
                Ok(p) => {
                    return Ok(if aes == Aesthetic::Color {
                        plot.scale_color_distiller(p, -1)
                    } else {
                        plot.scale_fill_distiller(p, -1)
                    })
                }
                Err(_) => {
                    return Err(format!(
                        "palette '{other}' is unknown for a numeric column (use viridis, \
                         magma, plasma, inferno, a Brewer name, or gradient)"
                    ))
                }
            },
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
    use crate::facet::FacetAxes;
    let mut axes = match f.axes.as_deref() {
        None | Some("margins") => FacetAxes::default(),
        Some("all") => FacetAxes::all(),
        Some("all_x") => FacetAxes::all_x(),
        Some("all_y") => FacetAxes::all_y(),
        Some(o) => {
            return Err(format!(
                "unknown axes '{o}' (expected margins, all, all_x, all_y)"
            ))
        }
    };
    match f.axis_labels.as_deref() {
        None | Some("all") => {}
        Some("margins") => axes = axes.margin_labels(),
        Some(o) => return Err(format!("unknown axis_labels '{o}' (expected all, margins)")),
    }
    let plot = plot.facet_axes(axes);
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
        (Some(w), None, None) => {
            let vertical = match f.dir.as_deref() {
                None | Some("h") => false,
                Some("v") => true,
                Some(o) => return Err(format!("unknown dir '{o}' (expected h, v)")),
            };
            let mut plot = plot.facet_wrap_free(w, f.ncol, scales);
            if let Some(width) = f.label_wrap {
                if let crate::facet::Facet::Wrap { labeller, .. } = &mut plot.facet {
                    *labeller = crate::facet::FacetLabeller::Wrap(width);
                }
            }
            if f.switch.is_some() {
                return Err("switch is for rows/cols; a wrap takes strip_position".into());
            }
            let side = match f.strip_position.as_deref() {
                None | Some("top") => crate::facet::StripPosition::Top,
                Some("bottom") => crate::facet::StripPosition::Bottom,
                Some("left") => crate::facet::StripPosition::Left,
                Some("right") => crate::facet::StripPosition::Right,
                Some(o) => {
                    return Err(format!(
                        "unknown strip_position '{o}' (expected top, bottom, left, right)"
                    ))
                }
            };
            Ok(plot
                .facet_wrap_dir(vertical, f.as_table.unwrap_or(true))
                .facet_strip_position(side))
        }
        (None, r, c) if r.is_some() || c.is_some() => {
            if f.strip_position.is_some() {
                return Err("strip_position is for wrap; rows/cols take switch".into());
            }
            let switch = match f.switch.as_deref() {
                None => crate::facet::FacetSwitch::None,
                Some("x") => crate::facet::FacetSwitch::X,
                Some("y") => crate::facet::FacetSwitch::Y,
                Some("both") => crate::facet::FacetSwitch::Both,
                Some(o) => return Err(format!("unknown switch '{o}' (expected x, y, both)")),
            };
            Ok(plot
                .facet_grid_free(r.as_deref(), c.as_deref(), scales)
                .facet_switch(switch))
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
            let rev = match full.and_then(|f| f.reverse.as_deref()) {
                None | Some("none") => (false, false),
                Some("x") => (true, false),
                Some("y") => (false, true),
                Some("xy") => (true, true),
                Some(o) => return Err(format!("unknown reverse '{o}' (expected x, y, xy, none)")),
            };
            let expand = match full.and_then(|f| f.expand) {
                None => [true; 4],
                Some(ExpandSpec::All(e)) => [e; 4],
                Some(ExpandSpec::Sides(s)) => s,
            };
            if rev != (false, false) || expand != [true; 4] {
                let [t, r, b, l] = expand;
                let mut c = crate::coord::cartesian::CoordCartesian::new()
                    .reverse(rev.0, rev.1)
                    .expand(t, r, b, l);
                if let Some((a, b)) = xl {
                    c = c.xlim(a, b);
                }
                if let Some((a, b)) = yl {
                    c = c.ylim(a, b);
                }
                plot.coord_cartesian_with(c)
            } else if xl.is_some() || yl.is_some() {
                plot.coord_cartesian_zoom(xl, yl)
            } else {
                plot
            }
        }
        "quickmap" => plot.coord_quickmap(),
        "map" => plot.coord_map(),
        "radial" => {
            let start = full.and_then(|f| f.start).unwrap_or(0.0);
            let end = full
                .and_then(|f| f.end)
                .unwrap_or(start + 2.0 * std::f64::consts::PI);
            let mut c = CoordPolar::new();
            c.start = start;
            c.span = end - start;
            c.inner_radius = full.and_then(|f| f.inner_radius).unwrap_or(0.0).clamp(0.0, 0.99);
            let (rev_t, rev_r) = match full.and_then(|f| f.reverse.as_deref()) {
                None | Some("none") => (false, false),
                Some("theta") => (true, false),
                Some("r") => (false, true),
                Some("thetar") => (true, true),
                Some(o) => {
                    return Err(format!(
                        "unknown reverse '{o}' for radial (expected theta, r, thetar, none)"
                    ))
                }
            };
            c = c
                .reverse(rev_t, rev_r)
                .rotate_angle(full.and_then(|f| f.rotate_angle).unwrap_or(false));
            if let Some(i) = full.and_then(|f| f.r_axis_inside) {
                c = c.r_axis_inside(i);
            }
            plot.coord_polar_with(c)
        }
        "flip" => plot.coord_flip(),
        "polar" => plot.coord_polar(),
        "fixed" | "equal" => plot.coord_fixed(full.and_then(|f| f.ratio).unwrap_or(1.0)),
        "radar" => plot.coord_radar(),
        "sf" => plot.coord_sf(),
        other => {
            return Err(format!(
                "unknown coord '{other}' (expected cartesian, flip, polar, radial, fixed, radar, quickmap, map, sf)"
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
    fn spec_stats_unique_connect_summary_hex_sf_coordinates() {
        let svg = render(&format!(
            r#"{{ {DATA}, "aes": {{"x": "x", "y": "y"}},
               "layers": [{{"geom": "point", "stat": "unique"}},
                          {{"geom": "path", "stat": "connect", "params": {{"connection": "mid"}}}}] }}"#
        ))
        .unwrap();
        assert!(svg.contains("<circle"));
        let svg = render(
            r#"{"data": {"x": [1, 2, 3, 1], "y": [1, 2, 3, 1], "z": [5, 6, 7, 8]},
                "aes": {"x": "x", "y": "y"},
                "layers": [{"geom": "hex", "stat": "summary_hex", "params": {"fun": "max"}}]}"#,
        )
        .unwrap();
        assert!(svg.contains("<polygon") || svg.contains("<path"));
        let svg = render(
            r#"{"data": {"geometry": ["POLYGON ((0 0, 4 0, 4 4, 0 4, 0 0))"], "n": ["A"]},
                "aes": {"label": "n"},
                "layers": [{"geom": "sf"}, {"geom": "text", "stat": "sf_coordinates"}]}"#,
        )
        .unwrap();
        assert!(svg.contains(">A<"), "label placed");
        for (stat, params, msg) in [
            ("connect", r#"{"connection": "zig"}"#, "unknown connection"),
            ("summary_hex", r#"{"fun": "mode"}"#, "unknown fun"),
            ("bin", "{}", "unknown stat"),
        ] {
            let e = render(&format!(
                r#"{{ {DATA}, "aes": {{"x": "x", "y": "y"}},
                   "layers": [{{"geom": "point", "stat": "{stat}", "params": {params}}}] }}"#
            ))
            .unwrap_err();
            assert!(e.contains(msg), "{stat}: {e}");
        }
    }

    #[test]
    fn key_glyph_and_show_legend() {
        let svg = render(&format!(
            r#"{{ {DATA}, "aes": {{"x": "x", "y": "y", "color": "g"}},
               "layers": [{{"geom": "line", "key_glyph": "timeseries"}},
                          {{"geom": "point", "show_legend": {{"color": false}}}}] }}"#
        ))
        .unwrap();
        assert!(svg.contains("<polyline"));
        let e = render(&format!(
            r#"{{ {DATA}, "aes": {{"x": "x", "y": "y"}},
               "layers": [{{"geom": "line", "key_glyph": "zigzag"}}] }}"#
        ))
        .unwrap_err();
        assert!(e.contains("unknown glyph"), "{e}");
    }

    #[test]
    fn new_coords_render() {
        for coord in [
            r#""quickmap""#,
            r#""map""#,
            r#"{"type": "radial", "start": -1.2, "end": 1.2, "inner_radius": 0.3}"#,
            r#"{"type": "cartesian", "reverse": "xy"}"#,
        ] {
            let svg = render(&format!(
                r#"{{ {DATA}, "aes": {{"x": "x", "y": "y"}}, "layers": [{{"geom": "point"}}],
                   "coord": {coord} }}"#
            ))
            .unwrap_or_else(|e| panic!("{coord}: {e}"));
            assert!(svg.contains("<circle"), "{coord}");
        }
    }

    #[test]
    fn guides_none_and_title() {
        let svg = render(&format!(
            r#"{{ {DATA}, "aes": {{"x": "x", "y": "y", "color": "g", "fill": "g"}},
               "layers": [{{"geom": "point"}}],
               "guides": {{"color": {{"title": "Group"}}, "fill": "none"}} }}"#
        ))
        .unwrap();
        assert!(svg.contains(">Group<"));
        let e = render(&format!(
            r#"{{ {DATA}, "aes": {{"x": "x", "y": "y"}}, "layers": [{{"geom": "point"}}],
               "guides": {{"x": "none"}} }}"#
        ))
        .unwrap_err();
        assert!(e.contains("not a legend aesthetic"), "{e}");
    }

    #[test]
    fn guides_colourbar_and_coloursteps() {
        let cont = r#""data": {"x": [1, 2, 3], "y": [1, 2, 3], "z": [0, 50, 100]},
               "aes": {"x": "x", "y": "y", "color": "z"}, "layers": [{"geom": "point"}]"#;
        let base = render(&format!("{{ {cont} }}")).unwrap();
        let steps = render(&format!(
            r#"{{ {cont}, "guides": {{"color": "coloursteps"}} }}"#
        ))
        .unwrap();
        assert!(steps.matches("<rect").count() < base.matches("<rect").count());
        let bar = render(&format!(
            r#"{{ {cont}, "guides": {{"color": {{"type": "colourbar", "nbin": 5,
               "display": "rectangles", "barheight": 120, "ticks": false, "frame": false}}}} }}"#
        ))
        .unwrap();
        assert_ne!(bar, base);
        let e = render(&format!(r#"{{ {cont}, "guides": {{"color": "bins"}} }}"#)).unwrap_err();
        assert!(e.contains("unknown guide 'bins'"), "{e}");
        let e = render(&format!(
            r#"{{ {cont}, "guides": {{"color": {{"display": "raster"}}}} }}"#
        ))
        .unwrap_err();
        assert!(e.contains("unknown 'raster'"), "{e}");
        render(&format!(
            r#"{{ {DATA}, "aes": {{"x": "x", "y": "y", "color": "g"}}, "layers": [{{"geom": "point"}}],
               "guides": {{"color": {{"reverse": true}}}} }}"#
        ))
        .unwrap();
    }

    #[test]
    fn value_scales_steps2_and_viridis_b() {
        let d = r#""data": {"x": [1, 2, 3], "y": [1, 2, 3], "g": ["a", "b", "c"],
               "s": [2, 5, 9], "z": [-10, 0, 30]}"#;
        for extra in [
            r#""aes": {"x": "x", "y": "y", "size": "s"}, "scales": {"size": {"identity": true}}"#,
            r#""aes": {"x": "x", "y": "y", "size": "g"}, "scales": {"size": {"values": {"a": 1, "b": 3, "c": 6}}}"#,
            r#""aes": {"x": "x", "y": "y", "alpha": "g"}, "scales": {"alpha": {"values": {"a": 0.2, "b": 0.5, "c": 1}}}"#,
            r#""aes": {"x": "x", "y": "y", "size": "s"}, "scales": {"size": {"range": [1, 9]}}"#,
            r#""aes": {"x": "x", "y": "y", "shape": "s"}, "scales": {"shape": {"identity": true}}"#,
            r##""aes": {"x": "x", "y": "y", "color": "z"}, "scales": {"color": {"steps2": ["#0000ff", "#ffffff", "#ff0000"], "bins": 4}}"##,
            r#""aes": {"x": "x", "y": "y", "color": "z"}, "scales": {"color": {"palette": "viridis_b:C"}}"#,
        ] {
            render(&format!(
                r#"{{ {d}, {extra}, "layers": [{{"geom": "point"}}] }}"#
            ))
            .unwrap_or_else(|e| panic!("{extra}: {e}"));
        }
        let e = render(&format!(
            r#"{{ {d}, "aes": {{"x": "x", "y": "y", "shape": "g"}},
               "scales": {{"shape": {{"values": {{"a": 1}}}}}}, "layers": [{{"geom": "point"}}] }}"#
        ))
        .unwrap_err();
        assert!(e.contains("scales.shape"), "{e}");
        let e = render(&format!(
            r#"{{ {d}, "aes": {{"x": "x", "y": "y"}}, "scales": {{"size": {{}}}}, "layers": [{{"geom": "point"}}] }}"#
        ))
        .unwrap_err();
        assert!(e.contains("exactly one"), "{e}");
    }

    #[test]
    fn facet_strip_position_and_switch() {
        let d = r#""data": {"x": [1, 2, 3, 4], "y": [1, 2, 3, 4], "r": ["p", "p", "q", "q"],
               "c": ["a", "b", "a", "b"]}, "aes": {"x": "x", "y": "y"}, "layers": [{"geom": "point"}]"#;
        let svg = render(&format!(
            r#"{{ {d}, "facet": {{"rows": "r", "cols": "c", "switch": "y"}} }}"#
        ))
        .unwrap();
        assert!(svg.contains("rotate(270.0") && svg.contains(">p<"), "{svg}");
        let svg = render(&format!(
            r#"{{ {d}, "facet": {{"wrap": "c", "strip_position": "right"}} }}"#
        ))
        .unwrap();
        assert!(svg.contains("rotate(90.0"), "{svg}");
        for (f, msg) in [
            (
                r#"{"wrap": "c", "strip_position": "middle"}"#,
                "unknown strip_position",
            ),
            (r#"{"rows": "r", "switch": "z"}"#, "unknown switch"),
            (
                r#"{"wrap": "c", "switch": "x"}"#,
                "a wrap takes strip_position",
            ),
            (
                r#"{"rows": "r", "strip_position": "left"}"#,
                "rows/cols take switch",
            ),
        ] {
            let e = render(&format!(r#"{{ {d}, "facet": {f} }}"#)).unwrap_err();
            assert!(e.contains(msg), "{f}: {e}");
        }
    }

    #[test]
    fn theme_test_preset_and_per_side_axis_text() {
        render(&format!(
            r#"{{ {DATA}, "aes": {{"x": "x", "y": "y"}}, "layers": [{{"geom": "point"}}], "theme": "test" }}"#
        ))
        .unwrap();
        let svg = render(&format!(
            r##"{{ {DATA}, "aes": {{"x": "x", "y": "y"}}, "layers": [{{"geom": "point"}}],
               "scales": {{"x": {{"name": "X"}}}},
               "theme": {{"base": "gray", "axis_text_x_top": {{"color": [200, 0, 0]}},
                          "axis_text_y_right": {{"size": 14}}}} }}"##
        ))
        .unwrap();
        // A bottom axis: the top element doesn't apply.
        assert!(!svg.contains("#C80000"), "{svg}");
    }

    #[test]
    fn coord_cartesian_expand() {
        let base = render(&format!(
            r#"{{ {DATA}, "aes": {{"x": "x", "y": "y"}}, "layers": [{{"geom": "point"}}] }}"#
        ))
        .unwrap();
        for e in ["false", "[true, false, true, false]"] {
            let svg = render(&format!(
                r#"{{ {DATA}, "aes": {{"x": "x", "y": "y"}}, "layers": [{{"geom": "point"}}],
                   "coord": {{"type": "cartesian", "expand": {e}}} }}"#
            ))
            .unwrap();
            assert_ne!(svg, base, "{e}");
        }
    }

    #[test]
    fn facet_axes_all_and_margin_labels() {
        let d = r#""data": {"x": [1, 2, 3, 4], "y": [1, 2, 3, 4], "r": ["p", "p", "q", "q"],
               "c": ["a", "b", "a", "b"]}, "aes": {"x": "x", "y": "y"}, "layers": [{"geom": "point"}]"#;
        let base = render(&format!(
            r#"{{ {d}, "facet": {{"rows": "r", "cols": "c"}} }}"#
        ))
        .unwrap();
        let all = render(&format!(
            r#"{{ {d}, "facet": {{"rows": "r", "cols": "c", "axes": "all"}} }}"#
        ))
        .unwrap();
        let margins = render(&format!(
            r#"{{ {d}, "facet": {{"rows": "r", "cols": "c", "axes": "all", "axis_labels": "margins"}} }}"#
        ))
        .unwrap();
        let n = |s: &str| s.matches("<text").count();
        assert!(
            n(&all) > n(&margins) && n(&margins) == n(&base),
            "{} {} {}",
            n(&all),
            n(&margins),
            n(&base)
        );
        let e = render(&format!(
            r#"{{ {d}, "facet": {{"wrap": "c", "axes": "some"}} }}"#
        ))
        .unwrap_err();
        assert!(e.contains("unknown axes"), "{e}");
    }

    #[test]
    fn secondary_axes() {
        let d = r#""data": {"x": [0, 10, 20], "y": [1, 2, 3]}, "aes": {"x": "x", "y": "y"},
               "layers": [{"geom": "point"}]"#;
        let svg = render(&format!(
            r#"{{ {d}, "scales": {{"x": {{"sec_axis": {{"name": "F", "scale": 1.8, "offset": 32}}}},
               "y": {{"sec_axis": true}}}} }}"#
        ))
        .unwrap();
        assert!(svg.contains(">F<") && svg.contains(">50<"), "{svg}");
    }

    #[test]
    fn n_breaks_and_time_scale() {
        let svg = render(
            r#"{ "data": {"x": [0, 3600, 7200], "y": [0, 50, 100]}, "aes": {"x": "x", "y": "y"},
                 "layers": [{"geom": "point"}],
                 "scales": {"x": {"time": true}, "y": {"n_breaks": 10}} }"#,
        )
        .unwrap();
        assert!(svg.contains(">01:00:00<") && svg.contains(">90<"), "{svg}");
    }

    #[test]
    fn guide_override_aes() {
        let svg = render(&format!(
            r#"{{ {DATA}, "aes": {{"x": "x", "y": "y", "color": "g"}}, "layers": [{{"geom": "point"}}],
               "guides": {{"color": {{"override_aes": {{"shape": "square", "size": 5, "alpha": 0.5}}}}}} }}"#
        ))
        .unwrap();
        assert!(svg.contains("opacity=\"0.5"), "{svg}");
        let e = render(&format!(
            r#"{{ {DATA}, "aes": {{"x": "x", "y": "y", "color": "g"}}, "layers": [{{"geom": "point"}}],
               "guides": {{"color": {{"override_aes": {{"shape": "blob"}}}}}} }}"#
        ))
        .unwrap_err();
        assert!(e.contains("unknown shape"), "{e}");
    }

    #[test]
    fn theme_strip_text_x_y() {
        let svg = render(
            r#"{ "data": {"x": [1, 2], "y": [1, 2], "r": ["p", "q"], "c": ["a", "b"]},
                 "aes": {"x": "x", "y": "y"}, "layers": [{"geom": "point"}],
                 "facet": {"rows": "r", "cols": "c"},
                 "theme": {"base": "gray", "strip_text_y": {"color": [200, 0, 0]},
                           "strip_background_x": {"fill": [0, 0, 200]}}}"#,
        )
        .unwrap();
        assert!(svg.contains("#C80000") && svg.contains("#0000C8"), "{svg}");
    }

    #[test]
    fn facet_wrap_dir_as_table_and_label_wrap() {
        let svg = render(&format!(
            r#"{{ {DATA}, "aes": {{"x": "x", "y": "y"}}, "layers": [{{"geom": "point"}}],
               "facet": {{"wrap": "g", "dir": "v", "as_table": false, "label_wrap": 3}} }}"#
        ))
        .unwrap();
        assert!(svg.contains(">a<") && svg.contains(">b<"));
        let e = render(&format!(
            r#"{{ {DATA}, "aes": {{"x": "x", "y": "y"}}, "layers": [{{"geom": "point"}}],
               "facet": {{"wrap": "g", "dir": "z"}} }}"#
        ))
        .unwrap_err();
        assert!(e.contains("unknown dir"), "{e}");
    }

    #[test]
    fn binned_scales() {
        let svg = render(
            r#"{ "data": {"x": [0.5, 1, 4, 9], "s": [1, 2, 3, 40]}, "aes": {"x": "x", "size": "s"},
                 "layers": [{"geom": "bar"}, {"geom": "point", "aes": {"y": "s"}}],
                 "scales": {"x": {"binned": 5}, "size": {"binned": 4, "range": [1, 8]}} }"#,
        )
        .unwrap();
        assert!(svg.contains(">2.5<"), "{svg}");
        let e = render(
            r#"{ "data": {"x": [1], "s": [1]}, "aes": {"x": "x", "size": "s"},
                 "layers": [{"geom": "point", "aes": {"y": "s"}}],
                 "scales": {"size": {"binned": 4, "identity": true}} }"#,
        )
        .unwrap_err();
        assert!(e.contains("binned takes only range"), "{e}");
    }

    #[test]
    fn radial_options_spec() {
        let d = r#""data": {"x": ["a", "b", "c"], "y": [1, 2, 3]}, "aes": {"x": "x", "y": "y"},
               "layers": [{"geom": "col"}]"#;
        let svg = render(&format!(
            r#"{{ {d}, "coord": {{"type": "radial", "reverse": "thetar", "rotate_angle": true,
               "r_axis_inside": false}} }}"#
        ))
        .unwrap();
        assert!(svg.contains("rotate("), "{svg}");
        let e = render(&format!(
            r#"{{ {d}, "coord": {{"type": "radial", "reverse": "x"}} }}"#
        ))
        .unwrap_err();
        assert!(e.contains("for radial"), "{e}");
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
    fn numeric_color_with_brewer_palette_is_a_distiller() {
        // #408: a Brewer name on a numeric column interpolates it
        // (`scale_colour_distiller`); an unknown name still errors.
        let svg = render(&format!(
            r#"{{ {DATA}, "aes": {{"x": "x", "y": "y", "color": "y"}}, "layers": [{{"geom": "point"}}],
                 "scales": {{"color": {{"palette": "Blues"}}}} }}"#
        ))
        .unwrap();
        assert!(svg.contains("<circle"));
        let e = render(&format!(
            r#"{{ {DATA}, "aes": {{"x": "x", "y": "y", "color": "y"}}, "layers": [{{"geom": "point"}}],
                 "scales": {{"color": {{"palette": "Nope"}}}} }}"#
        ))
        .unwrap_err();
        assert!(e.contains("unknown"), "{e}");
        let svg = render(&format!(
            r#"{{ {DATA}, "aes": {{"x": "x", "y": "y", "color": "g"}}, "layers": [{{"geom": "point"}}],
                 "scales": {{"color": {{"palette": "hue"}}}} }}"#
        ))
        .unwrap();
        assert!(
            svg.contains("#F8766D") && svg.contains("#00BFC4"),
            "ggplot2's 2-level hues"
        );
        let svg = render(&format!(
            r#"{{ {DATA}, "aes": {{"x": "x", "y": "y", "color": "g"}}, "layers": [{{"geom": "point"}}],
                 "scales": {{"color": {{"palette": "viridis:D"}}}} }}"#
        ))
        .unwrap();
        assert!(
            svg.contains("#440154") && svg.contains("#FDE725"),
            "viridis_d ends"
        );
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
