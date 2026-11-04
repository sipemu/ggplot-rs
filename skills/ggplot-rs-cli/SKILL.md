---
name: ggplot-rs-cli
description: Render charts from parquet/CSV files or DuckDB SQL with the `ggplot-rs` command-line tool (Grammar of Graphics, ggplot2-style) and produce an SVG or PNG. Use when asked to plot, chart, visualize, or graph tabular data on disk (parquet/CSV/shapefile/GeoJSON) or the result of a SQL query from the shell.
---

# Plotting tabular data with the `ggplot-rs` CLI

`ggplot-rs` reads a **parquet** or **CSV** file, or runs a **DuckDB SQL**
query, maps columns to aesthetics, and writes an **SVG/PNG** chart. DuckDB is
the query engine, so `--sql` can read parquet globs directly with
`read_parquet('data/*.parquet')`. The flags compile to the same JSON plot spec
the npm package `@sipemu/ggplot-rs` renders, so every geom and option it has is
available here too (`--spec FILE` takes that spec directly).

## Install

Prebuilt binaries for Linux, macOS and Windows are attached to every GitHub
release (<https://github.com/sipemu/ggplot-rs/releases/latest>):

```sh
# Linux x86_64 (other targets: aarch64-apple-darwin, x86_64-apple-darwin,
# x86_64-pc-windows-msvc .zip)
curl -L https://github.com/sipemu/ggplot-rs/releases/latest/download/ggplot-rs-x86_64-unknown-linux-gnu.tar.gz | tar xz
./ggplot-rs --version

cargo binstall ggplot-rs                # downloads the same prebuilt binary
cargo install ggplot-rs --features cli  # or build from source (bundles DuckDB; slow)
```

## Workflow (do this in order)

1. **Discover the schema first.** Never guess column names:
   ```sh
   ggplot-rs --parquet data.parquet --describe
   ```
   Prints `rows, columns` and one line per column: `name  type  non-null/total`.
   For a query: `ggplot-rs --sql "SELECT ..." --describe`.
2. **Map columns to aesthetics** with `--x --y --color --fill --size --shape
   --group --label`, or any aesthetic via `--aes NAME=COL` (repeatable).
3. **Pick geoms** with `--geom` (repeatable, one layer each; default `point`).
   `--list-geoms` prints them all.
4. **Render** to a file with `-o out.png` / `--output out.png` (format from extension) or `--stdout`
   for SVG.

## Input (choose one)

| Flag | Meaning |
|------|---------|
| `--parquet PATH` | read a parquet file (`read_parquet`) |
| `--csv PATH` | read a CSV (`read_csv_auto`) |
| `--sql "QUERY"` | run DuckDB SQL; can reference files via `read_parquet('f.parquet')`, `'f.csv'`, globs |
| `--db PATH` | attach a DuckDB database file (default: in-memory) |
| `--spatial` | load DuckDB `spatial`: `ST_Read('x.shp')`, `ST_AsText(geom)` in `--sql` |
| `--spec FILE` | a JSON plot spec; its `data` is used when no input flag is given |

## Geoms

`--geom` accepts every spec geom:

- basic: `point jitter line path step area ribbon stepribbon col bar segment curve
  spoke rect tile raster polygon text label text_repel label_repel rug blank`
- reference lines: `hline vline abline` (need `--param yintercept=` /
  `xintercept=` / `slope=`)
- distributions: `histogram freqpoly density density_ridges boxplot violin dotplot
  smooth count bin2d hex density2d contour contour_filled qq qq_line qq_band
  censor_marks`
- intervals: `errorbar errorbarh linerange pointrange crossbar xrange`
  (map `--aes ymin=.. --aes ymax=..`)
- business: `waterfall pareto funnel pyramid lollipop dumbbell bullet
  bullet_vertical marimekko variwide waffle parliament tilemap packed_bubble
  variable_pie wordcloud venn timeline calendar flags windbarb`
- hierarchies & networks: `treemap sunburst icicle dendrogram tree sankey chord
  arcdiagram network stream` (sankey/chord: `--aes from=.. --aes to=..
  --aes weight=..`; sunburst/icicle: `--param levels=a,b`)
- finance: `candlestick ohlc hlc heikin_ashi renko point_figure`
  (`--aes open=.. --aes high=.. --aes low=.. --aes close=..`)
- spatial: `sf sf_text sf_label flow` (a WKT `geometry` column; see Maps)

Rules of thumb: `histogram`/`density`/`bar` need only `--x`; `col` plots `--y`
as-is while `bar` counts rows per `--x`; `boxplot`/`violin` want a discrete
`--x` and numeric `--y`.

## Parameters and positions

`--param KEY=VALUE` (repeatable) sets a geom parameter on every layer. VALUE is
JSON if it parses, a comma list becomes an array, anything else is a string.
Keys: `bins` (histogram), `method` (`lm`/`loess`) `span` `se` (smooth),
`yintercept` `xintercept` `slope` `intercept` (reference lines), `levels`
(sunburst/icicle), `rows` (waffle/parliament), `box_size` `reversal`
(renko/point_figure), `shape` (tilemap: square/hexagon/circle/diamond), `speed`
`direction` (windbarb columns), `censor` (censor_marks column), `projection`
(spatial), `target` `ranges` (bullet), `notch` `varwidth` `coef` `outliers`
`staplewidth` (boxplot), `trim` `scale` (`area`/`count`/`width`) `quantiles`
`bw` `adjust` (violin), `connection` (stat connect: `hv`/`vh`/`mid`/`linear`),
`fun` (stat summary_hex: `mean`/`median`/`min`/`max`/`sum`).

`--spec` layers can also set `"stat"`: `identity`, `unique` (drop duplicate
rows), `connect` (steps), `summary_hex` (summary of a `z` column per hexagon),
`sf_coordinates` (a point per WKT geometry, e.g. `geom: "text"` labels on a map).

`--position NAME`: `identity stack fill dodge dodge2 jitter nudge`.

## Facets, scales, coords, labels

- Small multiples: `--facet-wrap COL`, or `--facet-grid "ROW:COL"` (either side
  may be empty, e.g. `":region"`); `--facet-scales fixed|free|free_x|free_y`.
- `--log-x`, `--log-y`, `--flip`; `--coord cartesian|flip|polar|fixed|radar|sf`.
- Labels: `--title --subtitle --xlab --ylab --caption`. Axis titles are not
  filled from column names — pass `--xlab`/`--ylab` when you want them.
- `--legend right|left|top|bottom|none|inside`.
- Size: `--width --height` (pixels; default 800×600).

## Theming

- `--theme gray|bw|minimal|classic|dark|light|void|linedraw` (default `gray`).
- `--palette NAME` — discrete color/fill palette for categorical mappings:
  `Set1 Set2 Set3 Dark2 Paired Accent`, sequential (`Blues Greens Reds YlOrRd …`),
  diverging (`RdBu Spectral PiYG …`), perceptual (`viridis magma plasma inferno`).
- `--primary "r,g,b"` — brand color for single-series geoms.
- `--theme-config FILE` — TOML or JSON element overrides on top of the preset:
  ```toml
  base = "minimal"
  primary = [200, 60, 40]
  palette = "RdBu"
  [title]
  size = 22
  color = [40, 40, 90]
  [panel_grid_minor]
  visible = false
  [axis_text_x]
  angle = 45
  ```
  Sections: `text` (root), `title subtitle caption axis_title_x/y axis_text_x/y
  legend_title legend_text strip_text` (`size color family face angle hjust vjust
  visible`); `axis_line axis_ticks panel_grid_major panel_grid_minor panel_border`
  (`color width linetype visible`); `panel_background plot_background
  legend_background strip_background` (`fill color width visible`). Colors are
  `[r, g, b]`. Top-level: `aspect_ratio panel_ontop axis_minor_ticks
  title_position tag_position legend_direction primary palette base`.

## Maps

```sh
ggplot-rs --spatial \
  --sql "SELECT ST_AsText(geom) AS geometry, name, pop_est
         FROM ST_Read('ne_110m_admin_0_countries.shp')" \
  --geom sf --fill pop_est --projection mercator --theme void -o world.png
```

`--projection platecarree|mercator|equal-earth|albers-usa|albers-europe|utm:<zone>[s]`;
an equal-aspect `coord_sf` is added automatically for `--geom sf`.

## Examples

```sh
# parquet → line chart PNG
ggplot-rs --parquet sales.parquet --x month --y revenue --geom line -o rev.png

# aggregate with SQL, faceted dodged bars
ggplot-rs --sql "SELECT region, product, sum(qty) q FROM 'orders/*.parquet' GROUP BY 1,2" \
  --x product --y q --fill product --geom col --facet-wrap region --theme minimal -o orders.svg

# points + loess trend without the ribbon
ggplot-rs --csv d.csv --x x --y y --color g --geom point --geom smooth \
  --param method=loess --param se=false -o trend.svg

# intervals + a reference line
ggplot-rs --csv d.csv --x x --y y --aes ymin=lo --aes ymax=hi \
  --geom pointrange --geom hline --param yintercept=0 -o ci.png

# sankey
ggplot-rs --csv flows.csv --aes from=src --aes to=dst --aes weight=n --geom sankey -o flows.svg

# a full JSON spec, data from DuckDB
ggplot-rs --spec chart.json --parquet data.parquet -o chart.svg

# SVG to stdout
ggplot-rs --csv data.csv --x a --y b --stdout > plot.svg
```

## Tips for agents

- Always `--describe` unfamiliar data; numeric columns for continuous axes,
  low-cardinality strings for `--color/--fill/--facet-*`.
- Aggregate/filter in `--sql` rather than plotting millions of raw rows.
- Errors name the problem and list the valid set: unknown columns list the
  available ones, unknown geoms list all geoms, unknown `--param` keys list the
  valid keys. Exit code is non-zero on failure; `wrote <file>` goes to stderr on
  success.
