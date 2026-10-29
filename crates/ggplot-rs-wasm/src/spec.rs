//! The layered plot spec behind the npm package's `render()` (#382).
//!
//! A JSON object that maps one-to-one onto the `GGPlot` builder:
//! `{ data, aes, layers: [{ geom, aes?, data?, params?, stat?, position? }],
//! scales, facet, coord, labels, theme, legend, width, height }`. Plain Rust
//! (no wasm-bindgen types) so it is unit-tested natively; `lib.rs` wraps it.
//! The TypeScript mirror of these types lives in `PLOT_SPEC_TS` in `lib.rs` —
//! keep the two in sync.

use std::collections::{BTreeMap, HashMap};

use serde::Deserialize;
use serde_json::Value as J;

use ggplot_rs::data::Value;
use ggplot_rs::facet::FacetScales;
use ggplot_rs::geom::histogram::GeomHistogram;
use ggplot_rs::geom::smooth::GeomSmooth;
use ggplot_rs::prelude::*;
use ggplot_rs::scale::color::{RGBAColor, ScaleColorDiscrete};
use ggplot_rs::scale::continuous::ScaleContinuous;
use ggplot_rs::scale::gradient_n::ScaleColorGradientN;
use ggplot_rs::scale::transform::ScaleTransform;
use ggplot_rs::theme::config::{parse_palette, preset, ThemeConfig};
use ggplot_rs::theme::LegendPosition;

/// Column-oriented data: `{ column: [values…] }`.
type Columns = BTreeMap<String, Vec<J>>;
/// Aesthetic → column name.
type AesMap = BTreeMap<String, String>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlotSpec {
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

#[derive(Deserialize)]
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
#[derive(Deserialize, Default)]
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
    let spec: PlotSpec =
        serde_json::from_str(spec_json).map_err(|e| format!("invalid plot spec: {e}"))?;
    render_spec(spec)
}

pub fn render_spec(spec: PlotSpec) -> Result<(String, Vec<String>), String> {
    let (width, height) = (spec.width.unwrap_or(640), spec.height.unwrap_or(400));
    if width == 0 || height == 0 {
        return Err("width and height must be positive".into());
    }
    if spec.layers.is_empty() {
        return Err("spec.layers must contain at least one layer".into());
    }

    let plot_data = columns(&spec.data)?;
    let plot_names: Vec<String> = spec.data.keys().cloned().collect();
    let plot_aes = build_aes(&spec.aes, &plot_names, "aes")?;
    // Column → "holds strings?", to pick discrete vs. continuous color scales.
    let mut is_str: HashMap<String, bool> = string_columns(&spec.data);

    let mut plot = GGPlot::new(plot_data).aes(plot_aes);
    for (i, layer) in spec.layers.iter().enumerate() {
        let names: Vec<String> = match &layer.data {
            Some(d) => d.keys().cloned().collect(),
            None => plot_names.clone(),
        };
        plot =
            add_geom(plot, &layer.geom, &layer.params).map_err(|e| format!("layers[{i}]: {e}"))?;
        if let Some(d) = &layer.data {
            plot = plot.layer_data(columns(d).map_err(|e| format!("layers[{i}].data: {e}"))?);
            for (k, v) in string_columns(d) {
                is_str.entry(k).or_insert(v);
            }
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
                "identity" => plot.stat(ggplot_rs::stat::identity::StatIdentity),
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
            cfg.apply(ggplot_rs::theme::presets::theme_minimal())
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

    plot.render_svg_native_with_warnings(width, height)
        .map_err(|e| format!("render failed: {e}"))
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

fn string_columns(data: &Columns) -> HashMap<String, bool> {
    data.iter()
        .map(|(k, vals)| {
            let s = vals
                .iter()
                .find(|x| !x.is_null())
                .map(|x| x.is_string() || x.is_boolean())
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

/// Geom names `add_geom` understands, for the error message and the docs.
pub const GEOMS: &[&str] = &[
    "point",
    "jitter",
    "line",
    "path",
    "step",
    "area",
    "ribbon",
    "col",
    "bar",
    "histogram",
    "freqpoly",
    "density",
    "boxplot",
    "violin",
    "dotplot",
    "smooth",
    "text",
    "label",
    "errorbar",
    "linerange",
    "pointrange",
    "crossbar",
    "segment",
    "rect",
    "tile",
    "raster",
    "polygon",
    "rug",
    "count",
    "bin2d",
    "hex",
    "density2d",
    "contour",
    "qq",
    "qq_line",
    "hline",
    "vline",
    "abline",
    "blank",
    "waterfall",
    "pareto",
    "sankey",
    "chord",
    "treemap",
    "sunburst",
    "icicle",
    "funnel",
    "stream",
    "lollipop",
    "dumbbell",
    "candlestick",
    "ohlc",
    "calendar",
];

fn add_geom(plot: GGPlot, geom: &str, p: &LayerParams) -> Result<GGPlot, String> {
    let need = |v: Option<f64>, k: &str| v.ok_or(format!("geom '{geom}' needs params.{k}"));
    Ok(match geom {
        "point" => plot.geom_point(),
        "jitter" => plot.geom_jitter(),
        "line" => plot.geom_line(),
        "path" => plot.geom_path(),
        "step" => plot.geom_step(),
        "area" => plot.geom_area(),
        "ribbon" => plot.geom_ribbon(),
        "col" => plot.geom_col(),
        "bar" => plot.geom_bar(),
        "histogram" => match p.bins {
            Some(b) => plot.geom_histogram_with(GeomHistogram::default().with_bins(b.max(1))),
            None => plot.geom_histogram(),
        },
        "freqpoly" => plot.geom_freqpoly(),
        "density" => plot.geom_density(),
        "boxplot" => plot.geom_boxplot(),
        "violin" => plot.geom_violin(),
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
        "text" => plot.geom_text(),
        "label" => plot.geom_label(),
        "errorbar" => plot.geom_errorbar(),
        "linerange" => plot.geom_linerange(),
        "pointrange" => plot.geom_pointrange(),
        "crossbar" => plot.geom_crossbar(),
        "segment" => plot.geom_segment(),
        "rect" => plot.geom_rect(),
        "tile" => plot.geom_tile(),
        "raster" => plot.geom_raster(),
        "polygon" => plot.geom_polygon(),
        "rug" => plot.geom_rug(),
        "count" => plot.geom_count(),
        "bin2d" => plot.geom_bin2d(),
        "hex" => plot.geom_hex(),
        "density2d" => plot.geom_density2d(),
        "contour" => plot.geom_contour(),
        "qq" => plot.geom_qq(),
        "qq_line" => plot.geom_qq_line(),
        "hline" => plot.geom_hline(need(p.yintercept, "yintercept")?),
        "vline" => plot.geom_vline(need(p.xintercept, "xintercept")?),
        "abline" => plot.geom_abline(need(p.slope, "slope")?, p.intercept.unwrap_or(0.0)),
        "blank" => plot.geom_blank(),
        "waterfall" => plot.geom_waterfall(),
        "pareto" => plot.geom_pareto(),
        "sankey" => plot.geom_sankey(),
        "chord" => plot.geom_chord(),
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
        "funnel" => plot.geom_funnel(),
        "stream" => plot.geom_stream(),
        "lollipop" => plot.geom_lollipop(),
        "dumbbell" => plot.geom_dumbbell(),
        "candlestick" => plot.geom_candlestick(),
        "ohlc" => plot.geom_ohlc(),
        "calendar" => plot.geom_calendar(),
        other => {
            return Err(format!(
                "unknown geom '{other}' (expected one of: {})",
                GEOMS.join(", ")
            ))
        }
    })
}

fn apply_position(plot: GGPlot, p: &PositionSpec) -> Result<GGPlot, String> {
    use ggplot_rs::position as pos;
    let (kind, full) = match p {
        PositionSpec::Name(n) => (n.as_str(), None),
        PositionSpec::Full(f) => (f.kind.as_str(), Some(f)),
    };
    let get = |f: fn(&PositionFull) -> Option<f64>, d: f64| full.and_then(f).unwrap_or(d);
    Ok(match kind {
        "identity" => plot.position(pos::identity::PositionIdentity),
        "stack" => plot.position(pos::stack::position_stack()),
        "fill" => plot.position(pos::fill::position_fill()),
        "dodge" => plot.position(pos::dodge::position_dodge(get(|f| f.width, 0.9))),
        "dodge2" => plot.position(pos::dodge2::PositionDodge2::new(get(|f| f.padding, 0.1))),
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
        other => {
            return Err(format!(
                "unknown coord '{other}' (expected cartesian, flip, polar, fixed, radar)"
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
            ..Default::default()
        };
        for g in GEOMS {
            let plot = GGPlot::new(vec![("x".to_string(), vec![Value::Float(1.0)])]);
            assert!(add_geom(plot, g, &p).is_ok(), "{g}");
        }
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
