---
name: plot-data
description: Render charts from parquet/CSV files or DuckDB SQL using the `ggplot-rs` CLI (Grammar of Graphics). Use when asked to plot, chart, visualize, or graph tabular data on disk (parquet/CSV) or the result of a SQL query, and produce an SVG or PNG.
---

# Plotting tabular data with the `ggplot-rs` CLI

`ggplot-rs` is a command-line Grammar-of-Graphics tool. It reads a **parquet** or
**CSV** file, or runs a **DuckDB SQL** query, maps columns to aesthetics, and
writes an **SVG/PNG** chart. DuckDB is the query engine, so `--sql` can read
parquet globs directly with `read_parquet('data/*.parquet')`.

## Install / locate the binary

The binary is built only with the `cli` feature:

```sh
cargo install ggplot-rs --features cli      # → `ggplot-rs` on PATH
# or, in this repo:
cargo run --features cli --bin ggplot-rs -- <args>
```

## Workflow (do this in order)

1. **Discover the schema first.** Never guess column names — list them:
   ```sh
   ggplot-rs --parquet data.parquet --describe
   ```
   This prints `rows, columns` and one line per column: `name  type  non-null/total`.
   For a query, describe its result: `ggplot-rs --sql "SELECT ..." --describe`.
2. **Map columns to aesthetics** with `--x/--y/--color/--fill/--size/--shape/--group`.
3. **Pick a geom** with `--geom` (default `point`).
4. **Render** to a file with `-o out.png` (format from extension) or `--stdout` for SVG.

## Input (choose one)

| Flag | Meaning |
|------|---------|
| `--parquet PATH` | read a parquet file (`read_parquet`) |
| `--csv PATH` | read a CSV (`read_csv_auto`) |
| `--sql "QUERY"` | run DuckDB SQL; can reference files via `read_parquet('f.parquet')`, `'f.csv'`, globs |
| `--db PATH` | attach a DuckDB database file (default: in-memory) |

## Aesthetics & geoms

- Aesthetics: `--x --y --color --fill --size --shape --group --label`.
- `--geom`: `point line bar col histogram boxplot violin density area smooth step path tile jitter freqpoly`.
  - `histogram`/`density`/`bar` need only `--x`. `boxplot`/`violin` need a discrete `--x` and numeric `--y`.
  - `col` plots `--y` as-is; `bar` counts rows per `--x`.

## Facets, scales, labels, theme

- Small multiples: `--facet-wrap COL`, or `--facet-grid "ROW:COL"` (either side may be empty, e.g. `--facet-grid ":region"`).
- `--log-x`, `--log-y`, `--flip` (swap axes).
- Labels: `--title --subtitle --xlab --ylab --caption`.
- `--theme`: `gray bw minimal classic dark light void linedraw`.
- Output size: `--width` `--height` (pixels; default 800×600).

## Examples

```sh
# parquet → line chart PNG
ggplot-rs --parquet sales.parquet --x month --y revenue --geom line -o rev.png

# aggregate with SQL, faceted bars
ggplot-rs --sql "SELECT region, product, sum(qty) q FROM 'orders/*.parquet' GROUP BY 1,2" \
  --x product --y q --fill product --geom col --facet-wrap region --theme minimal -o orders.svg

# distribution, colored by group, log y
ggplot-rs --parquet measurements.parquet --x value --color sensor --geom density --log-x -o dist.png

# emit SVG to stdout (e.g. to embed)
ggplot-rs --csv data.csv --x a --y b --geom point --stdout > plot.svg
```

## Tips for agents

- Always run `--describe` before plotting unfamiliar data; choose numeric columns
  for continuous axes and low-cardinality string columns for `--color/--fill/--facet-*`.
- Prefer `--sql` to aggregate/filter server-side rather than plotting raw rows.
- On `error: ...`, read the message: unknown column names surface as a DuckDB
  binder error from `--describe`/the query; `unknown --geom`/`--theme` list the valid set.
- Exit code is non-zero on failure; the output path is echoed to stderr on success.
