//! `ggplot-rs` — a command-line Grammar-of-Graphics tool.
//!
//! Reads a parquet/CSV file or a DuckDB SQL query, maps columns to aesthetics,
//! and renders a plot to SVG/PNG. Built only with `--features cli`.
//!
//! The flags compile to a [`ggplot_rs::spec::PlotSpec`] — the same JSON spec
//! the npm package renders — so every spec geom, param, scale, coord and theme
//! option is available here too (`--spec FILE` takes the spec directly).
//!
//! Examples:
//!   ggplot-rs --parquet sales.parquet --x month --y revenue --geom line -o rev.png
//!   ggplot-rs --sql "SELECT region, sum(qty) q FROM 'orders/*.parquet' GROUP BY 1" \
//!             --x region --y q --geom col -o orders.svg
//!   ggplot-rs --parquet data.parquet --describe    # list columns + types

use clap::Parser;
use ggplot_rs::spec::{
    self, CoordSpec, FacetSpec, LayerParams, LayerSpec, LegendSpec, PlotSpec, PositionScaleSpec,
    PositionSpec, ThemeSpec,
};

mod load;
mod theme;

/// Plot parquet files and DuckDB SQL from the shell.
#[derive(Parser, Debug)]
#[command(name = "ggplot-rs", version, about, long_about = None)]
struct Args {
    // ─── input (one of) ───────────────────────────────────────────
    /// Parquet file to read (via DuckDB `read_parquet`).
    #[arg(long, value_name = "PATH")]
    parquet: Option<String>,
    /// CSV file to read (via DuckDB `read_csv_auto`).
    #[arg(long, value_name = "PATH")]
    csv: Option<String>,
    /// DuckDB SQL query (may reference parquet/csv via read_parquet('...') etc.).
    #[arg(long, value_name = "SQL")]
    sql: Option<String>,
    /// DuckDB database file to attach (default: in-memory).
    #[arg(long, value_name = "PATH")]
    db: Option<String>,
    /// Load the DuckDB `spatial` extension, enabling `ST_Read(...)` (shapefiles,
    /// GeoJSON, GeoPackage, …) and `ST_AsText(geom)` in `--sql`.
    #[arg(long)]
    spatial: bool,

    // ─── spec ─────────────────────────────────────────────────────
    /// A JSON plot spec (the npm package's `render()` schema). Its `data` is
    /// used unless --parquet/--csv/--sql is given; other flags override it.
    #[arg(long, value_name = "FILE")]
    spec: Option<String>,

    // ─── discovery ────────────────────────────────────────────────
    /// Print the input's columns, inferred types, and row count, then exit.
    /// Use this first to discover what to map.
    #[arg(long)]
    describe: bool,
    /// Print every geom name --geom accepts, then exit.
    #[arg(long)]
    list_geoms: bool,

    // ─── aesthetics ───────────────────────────────────────────────
    #[arg(long)]
    x: Option<String>,
    #[arg(long)]
    y: Option<String>,
    #[arg(long)]
    color: Option<String>,
    #[arg(long)]
    fill: Option<String>,
    #[arg(long)]
    size: Option<String>,
    #[arg(long)]
    shape: Option<String>,
    #[arg(long)]
    group: Option<String>,
    #[arg(long)]
    label: Option<String>,
    /// Any other aesthetic as AES=COL (repeatable), e.g. `--aes ymin=lo
    /// --aes ymax=hi`, `--aes weight=n`, `--aes from=src --aes to=dst`.
    #[arg(long = "aes", value_name = "AES=COL")]
    aes: Vec<String>,

    /// Geometry (repeatable — one layer each, drawn in order), e.g.
    /// `--geom point --geom smooth`. Default `point`. See --list-geoms.
    #[arg(long, value_name = "NAME")]
    geom: Vec<String>,
    /// Geom parameter KEY=VALUE (repeatable), applied to every layer, e.g.
    /// `--param bins=30`, `--param method=loess`, `--param yintercept=0`,
    /// `--param levels=region,country`. VALUE is JSON if it parses, a
    /// comma list becomes an array, anything else is a string.
    #[arg(long, value_name = "KEY=VALUE")]
    param: Vec<String>,
    /// Position adjustment for every layer: identity, stack, fill, dodge,
    /// dodge2, jitter, nudge.
    #[arg(long, value_name = "NAME")]
    position: Option<String>,
    /// Map projection for `--geom sf` (and sf_text/sf_label/flow): `mercator`,
    /// `platecarree` (default), `equal-earth`, `albers-usa`, `albers-europe`,
    /// `utm:<zone>[s]`.
    #[arg(long, value_name = "NAME")]
    projection: Option<String>,

    // ─── facets ───────────────────────────────────────────────────
    /// Facet into small multiples by this column (facet_wrap).
    #[arg(long, value_name = "COL")]
    facet_wrap: Option<String>,
    /// facet_grid as "ROW:COL" (either side may be empty, e.g. ":region").
    #[arg(long, value_name = "ROW:COL")]
    facet_grid: Option<String>,
    /// Facet scales: fixed (default), free, free_x, free_y.
    #[arg(long, value_name = "MODE")]
    facet_scales: Option<String>,

    // ─── labels ───────────────────────────────────────────────────
    #[arg(long)]
    title: Option<String>,
    #[arg(long)]
    subtitle: Option<String>,
    #[arg(long)]
    xlab: Option<String>,
    #[arg(long)]
    ylab: Option<String>,
    #[arg(long)]
    caption: Option<String>,

    // ─── scales / coords / theme ──────────────────────────────────
    /// log10-transform the x axis.
    #[arg(long)]
    log_x: bool,
    /// log10-transform the y axis.
    #[arg(long)]
    log_y: bool,
    /// Flip x and y (coord_flip).
    #[arg(long)]
    flip: bool,
    /// Coordinate system: cartesian, flip, polar, radial, fixed, radar,
    /// quickmap, map, sf.
    #[arg(long, value_name = "NAME")]
    coord: Option<String>,
    /// Legend position: right, left, top, bottom, none, inside.
    #[arg(long, value_name = "POS")]
    legend: Option<String>,
    /// Theme: gray (default), bw, minimal, classic, dark, light, void, linedraw.
    #[arg(long)]
    theme: Option<String>,
    /// Custom theme: a TOML/JSON file of element overrides on top of the preset.
    #[arg(long, value_name = "FILE")]
    theme_config: Option<String>,
    /// Discrete color/fill palette (Set1, Dark2, viridis, RdBu, …).
    #[arg(long, value_name = "NAME")]
    palette: Option<String>,
    /// Brand/primary color "r,g,b" — default for single-series geoms.
    #[arg(long, value_name = "R,G,B")]
    primary: Option<String>,

    // ─── output ───────────────────────────────────────────────────
    /// Output file; format from extension (.svg/.png/.jpg/...).
    #[arg(short, long, value_name = "FILE")]
    output: Option<String>,
    /// Write SVG to stdout instead of a file.
    #[arg(long)]
    stdout: bool,
    /// Width in pixels (default 800, or the spec's).
    #[arg(long)]
    width: Option<u32>,
    /// Height in pixels (default 600, or the spec's).
    #[arg(long)]
    height: Option<u32>,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse();

    if args.list_geoms {
        for g in spec::geoms() {
            println!("{g}");
        }
        return Ok(());
    }

    // 1. The spec: from --spec, or empty; the flags are applied on top.
    let mut spec = match &args.spec {
        Some(path) => {
            let text = std::fs::read_to_string(path).map_err(|e| format!("reading {path}: {e}"))?;
            spec::parse(&text)?
        }
        None => PlotSpec::default(),
    };

    // 2. Data via DuckDB, unless the spec carries its own.
    let query = load::resolve_query(&args.sql, &args.parquet, &args.csv);
    let columns = match &query {
        Some(q) => Some(load::load(&args.db, q, args.spatial)?),
        None if args.spec.is_some() && !args.describe => None,
        None => return Err("provide one of --sql, --parquet, --csv (or --spec with data)".into()),
    };

    // 3. Discovery mode: describe the schema and exit.
    if args.describe {
        load::describe(&columns.unwrap_or_default());
        return Ok(());
    }

    apply_flags(&mut spec, &args)?;
    let plot = match columns {
        Some(c) => spec::build_plot_with_data(&spec, c)?,
        None => spec::build_plot(&spec)?,
    };
    let width = args.width.or(spec.width).unwrap_or(800);
    let height = args.height.or(spec.height).unwrap_or(600);

    // 4. Render.
    if args.stdout {
        let svg = plot
            .render_svg_with_size(width, height)
            .map_err(|e| format!("render failed: {e:?}"))?;
        print!("{svg}");
    } else {
        let out = args
            .output
            .as_deref()
            .ok_or("provide -o <file> or --stdout")?;
        plot.save_with_size(out, width, height)
            .map_err(|e| format!("save failed: {e:?}"))?;
        eprintln!("wrote {out}");
    }
    Ok(())
}

/// `KEY=VALUE` → (key, value).
fn split_kv<'a>(s: &'a str, flag: &str) -> Result<(&'a str, &'a str), String> {
    s.split_once('=')
        .filter(|(k, _)| !k.is_empty())
        .ok_or(format!("{flag} expects KEY=VALUE, got '{s}'"))
}

/// A `--param` value: JSON if it parses, a comma list → array, else a string.
fn param_value(v: &str) -> serde_json::Value {
    use serde_json::Value as J;
    if let Ok(j) = serde_json::from_str::<J>(v) {
        return j;
    }
    if v.contains(',') {
        return J::Array(
            v.split(',')
                .map(|p| serde_json::from_str(p).unwrap_or_else(|_| J::String(p.to_string())))
                .collect(),
        );
    }
    J::String(v.to_string())
}

/// Fold the command-line flags into `spec` (flags win over the spec file).
fn apply_flags(spec: &mut PlotSpec, args: &Args) -> Result<(), String> {
    // Aesthetics.
    for (aes, col) in [
        ("x", &args.x),
        ("y", &args.y),
        ("color", &args.color),
        ("fill", &args.fill),
        ("size", &args.size),
        ("shape", &args.shape),
        ("group", &args.group),
        ("label", &args.label),
    ] {
        if let Some(c) = col {
            spec.aes.insert(aes.into(), c.clone());
        }
    }
    for kv in &args.aes {
        let (k, v) = split_kv(kv, "--aes")?;
        spec.aes.insert(k.into(), v.into());
    }

    // Layers: --geom replaces the spec's layers; default a single point layer.
    if !args.geom.is_empty() {
        spec.layers = args
            .geom
            .iter()
            .map(|g| LayerSpec {
                geom: g.clone(),
                ..Default::default()
            })
            .collect();
    } else if spec.layers.is_empty() {
        spec.layers.push(LayerSpec {
            geom: "point".into(),
            ..Default::default()
        });
    }
    let mut params = serde_json::Map::new();
    for kv in &args.param {
        let (k, v) = split_kv(kv, "--param")?;
        params.insert(k.into(), param_value(v));
    }
    if let Some(p) = &args.projection {
        params.insert("projection".into(), p.clone().into());
    }
    if !params.is_empty() {
        let overlay: LayerParams = serde_json::from_value(serde_json::Value::Object(params))
            .map_err(|e| format!("--param: {e}"))?;
        for layer in &mut spec.layers {
            layer.params.overlay(overlay.clone());
        }
    }
    if let Some(pos) = &args.position {
        for layer in &mut spec.layers {
            layer.position = Some(PositionSpec::Name(pos.clone()));
        }
    }

    // Facets.
    if let Some(col) = &args.facet_wrap {
        spec.facet = Some(FacetSpec {
            wrap: Some(col.clone()),
            ..Default::default()
        });
    } else if let Some(g) = &args.facet_grid {
        let (row, col) = g.split_once(':').unwrap_or((g.as_str(), ""));
        spec.facet = Some(FacetSpec {
            rows: (!row.is_empty()).then(|| row.to_string()),
            cols: (!col.is_empty()).then(|| col.to_string()),
            ..Default::default()
        });
    }
    if let (Some(s), Some(f)) = (&args.facet_scales, spec.facet.as_mut()) {
        f.scales = Some(s.clone());
    }

    // Scales / coords.
    let log10 = |s: &mut Option<PositionScaleSpec>| {
        s.get_or_insert_with(Default::default).trans = Some("log10".into());
    };
    if args.log_x {
        log10(&mut spec.scales.x);
    }
    if args.log_y {
        log10(&mut spec.scales.y);
    }
    if let Some(c) = &args.coord {
        spec.coord = Some(CoordSpec::Name(c.clone()));
    } else if args.flip {
        spec.coord = Some(CoordSpec::Name("flip".into()));
    } else if spec.coord.is_none() && spec.layers.iter().any(|l| l.geom == "sf") {
        // A map keeps its shape under an equal-aspect spatial coord.
        spec.coord = Some(CoordSpec::Name("sf".into()));
    }
    if let Some(l) = &args.legend {
        spec.legend = Some(LegendSpec::Name(l.clone()));
    }

    // Labels.
    let l = &mut spec.labels;
    for (slot, flag) in [
        (&mut l.title, &args.title),
        (&mut l.subtitle, &args.subtitle),
        (&mut l.x, &args.xlab),
        (&mut l.y, &args.ylab),
        (&mut l.caption, &args.caption),
    ] {
        if flag.is_some() {
            slot.clone_from(flag);
        }
    }

    // Theme: --theme-config file (or the spec's config) → --theme preset
    // (default gray) → --primary / --palette on top.
    let flags_touch_theme = args.theme.is_some()
        || args.theme_config.is_some()
        || args.primary.is_some()
        || args.palette.is_some();
    if flags_touch_theme || spec.theme.is_none() {
        let mut cfg = match (&args.theme_config, spec.theme.take()) {
            (Some(path), _) => theme::load(path)?,
            (None, Some(ThemeSpec::Config(c))) => *c,
            (None, Some(ThemeSpec::Preset(p))) => theme::ThemeConfig {
                base: Some(p),
                ..Default::default()
            },
            (None, None) => theme::ThemeConfig::default(),
        };
        if let Some(t) = &args.theme {
            cfg.base = Some(t.clone());
        } else if cfg.base.is_none() {
            cfg.base = Some("gray".into());
        }
        theme::preset(cfg.base.as_deref().unwrap_or("gray"))?;
        if let Some(p) = &args.primary {
            let (r, g, b) = theme::parse_rgb(p)?;
            cfg.primary = Some([r, g, b]);
        }
        if let Some(p) = &args.palette {
            theme::parse_palette(p)?;
            cfg.palette = Some(p.clone());
        }
        spec.theme = Some(ThemeSpec::Config(Box::new(cfg)));
    }
    Ok(())
}
