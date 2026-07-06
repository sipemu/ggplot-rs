//! `ggplot-rs` — a command-line Grammar-of-Graphics tool.
//!
//! Reads a parquet/CSV file or a DuckDB SQL query, maps columns to aesthetics,
//! and renders a plot to SVG/PNG. Built only with `--features cli`.
//!
//! Examples:
//!   ggplot-rs --parquet sales.parquet --x month --y revenue --geom line -o rev.png
//!   ggplot-rs --sql "SELECT region, sum(qty) q FROM 'orders/*.parquet' GROUP BY 1" \
//!             --x region --y q --geom col -o orders.svg
//!   ggplot-rs --parquet data.parquet --describe    # list columns + types

use clap::Parser;
use ggplot_rs::prelude::*;

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

    // ─── discovery ────────────────────────────────────────────────
    /// Print the input's columns, inferred types, and row count, then exit.
    /// Use this first to discover what to map.
    #[arg(long)]
    describe: bool,

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

    /// Geometry: point, line, bar, col, histogram, boxplot, violin, density,
    /// area, smooth, step, path, tile, jitter, freqpoly, sf.
    #[arg(long, default_value = "point")]
    geom: String,
    /// Map projection for `--geom sf`: `mercator` or `platecarree` (default).
    #[arg(long, value_name = "NAME")]
    projection: Option<String>,

    // ─── facets ───────────────────────────────────────────────────
    /// Facet into small multiples by this column (facet_wrap).
    #[arg(long, value_name = "COL")]
    facet_wrap: Option<String>,
    /// facet_grid as "ROW:COL" (either side may be empty, e.g. ":region").
    #[arg(long, value_name = "ROW:COL")]
    facet_grid: Option<String>,

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
    /// Theme: gray, bw, minimal, classic, dark, light, void, linedraw.
    #[arg(long, default_value = "gray")]
    theme: String,
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
    #[arg(long, default_value_t = 800)]
    width: u32,
    #[arg(long, default_value_t = 600)]
    height: u32,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse();

    // 1. Load data via DuckDB.
    let query = load::resolve_query(&args.sql, &args.parquet, &args.csv)
        .ok_or("provide one of --sql, --parquet, or --csv")?;
    let columns = load::load(&args.db, &query, args.spatial)?;

    // 2. Discovery mode: describe the schema and exit.
    if args.describe {
        load::describe(&columns);
        return Ok(());
    }

    // 3. Build the plot spec from the flags.
    let mut plot = build_plot(&args, columns)?;
    plot = apply_output_labels(plot, &args);

    // 4. Render.
    if args.stdout {
        let svg = plot
            .render_svg_with_size(args.width, args.height)
            .map_err(|e| format!("render failed: {e:?}"))?;
        print!("{svg}");
    } else {
        let out = args
            .output
            .as_deref()
            .ok_or("provide -o <file> or --stdout")?;
        plot.save_with_size(out, args.width, args.height)
            .map_err(|e| format!("save failed: {e:?}"))?;
        eprintln!("wrote {out}");
    }
    Ok(())
}

fn build_plot(args: &Args, columns: Vec<(String, Vec<Value>)>) -> Result<GGPlot, String> {
    // Validate every referenced column up front with a helpful message.
    let names: Vec<&str> = columns.iter().map(|(n, _)| n.as_str()).collect();
    let check = |col: &Option<String>| -> Result<(), String> {
        match col {
            Some(c) if !names.contains(&c.as_str()) => Err(format!(
                "column '{c}' not found. Available columns: {}",
                names.join(", ")
            )),
            _ => Ok(()),
        }
    };
    for col in [
        &args.x,
        &args.y,
        &args.color,
        &args.fill,
        &args.size,
        &args.shape,
        &args.group,
        &args.label,
        &args.facet_wrap,
    ] {
        check(col)?;
    }

    // Aesthetic mapping from the provided flags.
    let mut aes = Aes::new();
    let set = |a: Aes, col: &Option<String>, f: fn(Aes, &str) -> Aes| match col {
        Some(c) => f(a, c),
        None => a,
    };
    aes = set(aes, &args.x, |a, c| a.x(c));
    aes = set(aes, &args.y, |a, c| a.y(c));
    aes = set(aes, &args.color, |a, c| a.color(c));
    aes = set(aes, &args.fill, |a, c| a.fill(c));
    aes = set(aes, &args.size, |a, c| a.size(c));
    aes = set(aes, &args.shape, |a, c| a.shape(c));
    aes = set(aes, &args.group, |a, c| a.group(c));
    aes = set(aes, &args.label, |a, c| a.label(c));

    let mut plot = GGPlot::new(columns).aes(aes);

    // Geometry.
    plot = match args.geom.as_str() {
        "point" => plot.geom_point(),
        "line" => plot.geom_line(),
        "bar" => plot.geom_bar(),
        "col" => plot.geom_col(),
        "histogram" => plot.geom_histogram(),
        "boxplot" => plot.geom_boxplot(),
        "violin" => plot.geom_violin(),
        "density" => plot.geom_density(),
        "area" => plot.geom_area(),
        "smooth" => plot.geom_smooth(),
        "step" => plot.geom_step(),
        "path" => plot.geom_path(),
        "tile" => plot.geom_tile(),
        "jitter" => plot.geom_jitter(),
        "freqpoly" => plot.geom_freqpoly(),
        "sf" => {
            let projection = match args.projection.as_deref() {
                Some("mercator") => ggplot_rs::spatial::SfProjection::Mercator,
                None | Some("platecarree") | Some("plate") => {
                    ggplot_rs::spatial::SfProjection::PlateCarree
                }
                Some(other) => return Err(format!("unknown --projection '{other}'")),
            };
            plot.geom_sf_with(ggplot_rs::geom::sf::GeomSf::default().project(projection))
        }
        other => return Err(format!("unknown --geom '{other}'")),
    };

    // A map keeps its shape under an equal-aspect spatial coord.
    if args.geom == "sf" && !args.flip {
        plot = plot.coord_sf();
    }

    // Facets.
    if let Some(col) = &args.facet_wrap {
        plot = plot.facet_wrap(col, None);
    } else if let Some(spec) = &args.facet_grid {
        let (row, col) = spec.split_once(':').unwrap_or((spec.as_str(), ""));
        let row = (!row.is_empty()).then_some(row);
        let col = (!col.is_empty()).then_some(col);
        plot = plot.facet_grid(row, col);
    }

    // Scales / coords.
    if args.log_x {
        plot = plot.scale_x_log10();
    }
    if args.log_y {
        plot = plot.scale_y_log10();
    }
    if args.flip {
        plot = plot.coord_flip();
    }

    // Theme: preset → optional custom config overrides → optional --primary.
    let cfg = match &args.theme_config {
        Some(path) => Some(theme::load(path)?),
        None => None,
    };
    // Start from the --theme preset; the config (which may set its own `base`)
    // overrides on top.
    let base = theme::preset(&args.theme)?;
    let mut th = match &cfg {
        Some(c) => c.apply(base)?,
        None => base,
    };
    if let Some(p) = &args.primary {
        th.primary = Some(theme::parse_rgb(p)?);
    }
    plot = plot.theme(th);

    // Palette (flag wins over the config file), applied to color and fill.
    let palette = args
        .palette
        .clone()
        .or_else(|| cfg.as_ref().and_then(|c| c.palette.clone()));
    if let Some(name) = palette {
        let p = theme::parse_palette(&name)?;
        plot = plot.scale_color_brewer(p.clone()).scale_fill_brewer(p);
    }

    Ok(plot)
}

fn apply_output_labels(mut plot: GGPlot, args: &Args) -> GGPlot {
    if let Some(t) = &args.title {
        plot = plot.title(t);
    }
    if let Some(t) = &args.subtitle {
        plot = plot.subtitle(t);
    }
    if let Some(t) = &args.xlab {
        plot = plot.xlab(t);
    }
    if let Some(t) = &args.ylab {
        plot = plot.ylab(t);
    }
    if let Some(t) = &args.caption {
        plot = plot.caption(t);
    }
    plot
}
