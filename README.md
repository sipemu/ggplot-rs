# ggplot-rs

[![CI](https://github.com/sipemu/ggplot-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/sipemu/ggplot-rs/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/ggplot-rs.svg)](https://crates.io/crates/ggplot-rs)
[![Documentation](https://docs.rs/ggplot-rs/badge.svg)](https://docs.rs/ggplot-rs)
[![codecov](https://codecov.io/gh/sipemu/ggplot-rs/branch/main/graph/badge.svg)](https://codecov.io/gh/sipemu/ggplot-rs)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#license)

A Rust implementation of ggplot2's Grammar of Graphics, rendering through the
[plotters](https://github.com/plotters-rs/plotters) backend.

**Validated against R.** Computed layers — binning, density, stacking, QQ/ECDF,
LOESS, and axis-tick placement (extended-Wilkinson) — are checked against
[R ggplot2](https://ggplot2.tidyverse.org/) 4.0.3's `ggplot_build()` output, so a
histogram or a stacked bar comes out where ggplot2 puts it. See
[`validation/`](validation/).

**No polars required.** [polars](https://pola.rs/) is a convenient — and fully
optional — input adapter. The core pipeline runs on its own internal DataFrame,
so you can plot straight from plain Rust vectors, or from
[Apache Arrow](https://arrow.apache.org/) `RecordBatch`es produced by
[DuckDB](https://duckdb.org/) — with polars switched off entirely. See
[Data Input](#data-input) and [Feature Flags](#feature-flags).

## Gallery

Every image below is produced by [`examples/gallery.rs`](examples/gallery.rs) —
regenerate them all with `cargo run --example gallery`.

<table>
  <tr>
    <td align="center" width="33%">
      <img src="assets/gallery/scatter.png" width="260" alt="Grouped scatter"><br>
      <sub>Grouped scatter · <code>geom_point</code></sub>
    </td>
    <td align="center" width="33%">
      <img src="assets/gallery/smooth.png" width="260" alt="LOESS smoothing"><br>
      <sub>LOESS trend + CI · <code>geom_smooth</code></sub>
    </td>
    <td align="center" width="33%">
      <img src="assets/gallery/continuous_color.png" width="260" alt="Continuous color"><br>
      <sub>Continuous color · viridis</sub>
    </td>
  </tr>
  <tr>
    <td align="center">
      <img src="assets/gallery/histogram.png" width="260" alt="Histogram"><br>
      <sub>Histogram · <code>geom_histogram</code></sub>
    </td>
    <td align="center">
      <img src="assets/gallery/density.png" width="260" alt="Density by group"><br>
      <sub>Densities · <code>geom_density</code></sub>
    </td>
    <td align="center">
      <img src="assets/gallery/bar.png" width="260" alt="Bar chart"><br>
      <sub>Bar chart · <code>geom_bar</code></sub>
    </td>
  </tr>
  <tr>
    <td align="center">
      <img src="assets/gallery/boxplot.png" width="260" alt="Boxplot"><br>
      <sub>Boxplots · <code>geom_boxplot</code></sub>
    </td>
    <td align="center">
      <img src="assets/gallery/violin.png" width="260" alt="Violin"><br>
      <sub>Violins · <code>geom_violin</code></sub>
    </td>
    <td align="center">
      <img src="assets/gallery/jitter.png" width="260" alt="Jitter"><br>
      <sub>Jittered points · <code>geom_jitter</code></sub>
    </td>
  </tr>
  <tr>
    <td align="center">
      <img src="assets/gallery/contour_filled.png" width="260" alt="Filled contours"><br>
      <sub>Filled contours · <code>geom_contour_filled</code></sub>
    </td>
    <td align="center">
      <img src="assets/gallery/hexbin.png" width="260" alt="Hex binning"><br>
      <sub>Hex binning · <code>geom_hex</code></sub>
    </td>
    <td align="center">
      <img src="assets/gallery/heatmap.png" width="260" alt="Heatmap"><br>
      <sub>Heatmap · <code>geom_tile</code></sub>
    </td>
  </tr>
  <tr>
    <td align="center">
      <img src="assets/gallery/ribbon.png" width="260" alt="Ribbon"><br>
      <sub>Confidence band · <code>geom_ribbon</code></sub>
    </td>
    <td align="center">
      <img src="assets/gallery/area.png" width="260" alt="Stacked area"><br>
      <sub>Stacked area · <code>geom_area</code></sub>
    </td>
    <td align="center">
      <img src="assets/gallery/facet.png" width="260" alt="Facet wrap"><br>
      <sub>Small multiples · <code>facet_wrap</code></sub>
    </td>
  </tr>
  <tr>
    <td align="center">
      <img src="assets/gallery/polar.png" width="260" alt="Polar rose chart"><br>
      <sub>Rose chart · <code>coord_polar</code></sub>
    </td>
    <td align="center">
      <img src="assets/gallery/ecdf.png" width="260" alt="Empirical CDF"><br>
      <sub>Empirical CDF · <code>stat_ecdf</code></sub>
    </td>
    <td align="center">
      <img src="assets/gallery/qq.png" width="260" alt="Q-Q plot"><br>
      <sub>Q-Q plot · <code>geom_qq</code></sub>
    </td>
  </tr>
</table>

### Themes

The same plot under each built-in theme — swap with a single `.theme(theme_*())` call.

<table>
  <tr>
    <td align="center" width="33%">
      <img src="assets/gallery/theme_gray.png" width="240" alt="theme_gray"><br>
      <sub><code>theme_gray</code> (default)</sub>
    </td>
    <td align="center" width="33%">
      <img src="assets/gallery/theme_bw.png" width="240" alt="theme_bw"><br>
      <sub><code>theme_bw</code></sub>
    </td>
    <td align="center" width="33%">
      <img src="assets/gallery/theme_minimal.png" width="240" alt="theme_minimal"><br>
      <sub><code>theme_minimal</code></sub>
    </td>
  </tr>
  <tr>
    <td align="center">
      <img src="assets/gallery/theme_classic.png" width="240" alt="theme_classic"><br>
      <sub><code>theme_classic</code></sub>
    </td>
    <td align="center">
      <img src="assets/gallery/theme_light.png" width="240" alt="theme_light"><br>
      <sub><code>theme_light</code></sub>
    </td>
    <td align="center">
      <img src="assets/gallery/theme_dark.png" width="240" alt="theme_dark"><br>
      <sub><code>theme_dark</code></sub>
    </td>
  </tr>
  <tr>
    <td align="center">
      <img src="assets/gallery/theme_linedraw.png" width="240" alt="theme_linedraw"><br>
      <sub><code>theme_linedraw</code></sub>
    </td>
    <td align="center">
      <img src="assets/gallery/theme_void.png" width="240" alt="theme_void"><br>
      <sub><code>theme_void</code></sub>
    </td>
    <td align="center"></td>
  </tr>
</table>

## Quick Start

```rust
use ggplot_rs::prelude::*;
use polars::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let df = df! {
        "sepal_length" => [5.1, 4.9, 4.7, 7.0, 6.4],
        "sepal_width"  => [3.5, 3.0, 3.2, 3.2, 3.2],
        "species"      => ["setosa", "setosa", "setosa", "versicolor", "versicolor"],
    }?;

    GGPlot::new(df)
        .aes(Aes::new().x("sepal_length").y("sepal_width").color("species"))
        .geom_point()
        .save("scatter.svg")?;

    Ok(())
}
```

## Features

### Geoms

`geom_point`, `geom_line`, `geom_bar`, `geom_col`, `geom_histogram`, `geom_boxplot`, `geom_violin`, `geom_smooth`, `geom_density`, `geom_area`, `geom_ribbon`, `geom_errorbar`, `geom_segment`, `geom_rug`, `geom_text`, `geom_label`, `geom_tile`, `geom_raster`, `geom_bin2d`, `geom_hex`, `geom_contour`, `geom_contour_filled`, `geom_path`, `geom_step`, `geom_hline`, `geom_vline`, `geom_abline`, and more (40+)

### Stats

`StatIdentity`, `StatCount`, `StatBin`, `StatBoxplot`, `StatSmooth` (Lm + Loess), `StatDensity`, `StatLoess`, `StatSummary`, `StatEcdf`, `StatFunction`, `StatEllipse`, `StatContour`, `StatBin2d`, `StatBinHex`, `StatSum`, `StatYDensity`, `StatQQ`, `StatSummary2d`, `StatQuantile` (feature `regression`), and more

### Scales

- **Continuous**: linear, log10, log2, ln, sqrt, reverse, logit, probit, pseudo-log, reciprocal, exp, and Box–Cox transforms
- **Discrete**: automatic categorical mapping
- **Color**: discrete palettes (Viridis, Brewer Set1/Dark2, etc.), continuous gradients, diverging gradient2, binned/stepped scales (`scale_color_steps`/`fermenter`), manual color assignment
- **Shape & Linetype**: discrete mapping for point shapes and line styles

### Coordinates

`coord_cartesian`, `coord_flip`, `coord_fixed`, `coord_polar`, `coord_trans`

### Faceting

`facet_wrap` and `facet_grid` with free/fixed scales, proportional panel sizing
(`space = "free"` via `facet_grid_space`), and multi-variable columns
(`facet_grid_multi`, R's `rows ~ b + c`). Computed stats (density/histogram) are
estimated per panel.

### Themes

`theme_gray`, `theme_bw`, `theme_classic`, `theme_minimal`, `theme_dark`, `theme_light`, `theme_linedraw`, `theme_void` — plus full customization via `ElementText`, `ElementLine`, `ElementRect`

### Annotations

`annotate_text`, `annotate_rect`, `annotate_segment`

### Guides & axes

- **Legend inside the panel** at panel-relative coords: `legend_position_inside(x, y)` (R's `legend.position = c(x, y)`).
- **Axis label rotation**: `axis_text_x_angle(deg)` / `axis_text_y_angle(deg)` (R's `guide_axis(angle = ...)`).
- **Label dodging**: `axis_text_x_dodge(n)` staggers crowded x labels across `n` rows (`guide_axis(n.dodge)`).
- **Corner tag**: `tag("A")` for figure-panel labels (`labs(tag)`).
- **Axis position & expansion**: `ScaleContinuous::with_position_opposite()` (x-axis on top / y on the right) and `with_expand_sides(...)` for per-side expansion.

Call theme-related builders after any `theme_*()` preset.

### Computed aesthetics

An aesthetic can map an **expression** over columns, not just a bare column name:

```rust
GGPlot::new(data)
    .aes(Aes::new().x("log10(gdp)").y("pop / 1e6").color("deaths / cases"))
    .geom_point();
```

Supports `+ - * / % ^`, parentheses, and `ln`/`log`/`log10`/`log2`/`sqrt`/`exp`/`abs`/`sin`/`cos`/`tan`/`floor`/`ceil`/`round`/`sign`. A plain column name is used directly (so existing mappings are unchanged); anything else is parsed and evaluated per row. `after_scale_fill_from_color(l)` / `after_scale_color_from_fill(l)` derive one color aesthetic from another's mapped color, lightness-adjusted (`after_scale`); `Aes::stage(aes, start, after_stat)` maps an aesthetic at two pipeline stages. The same expressions work in `after_stat` mappings, plus aggregate functions (`sum`, `mean`, `max`, `min`, `count`, `median`, `prod`) that reduce over all rows — e.g. `.after_stat_y("count / sum(count)")` for proportion histograms.

## Command-line tool

A `ggplot-rs` CLI (behind the `cli` feature) plots **parquet**/**CSV** files or
**DuckDB SQL** straight from the shell — DuckDB is the query engine:

```sh
cargo install ggplot-rs --features cli

# discover columns first, then plot
ggplot-rs --parquet sales.parquet --describe
ggplot-rs --parquet sales.parquet --x month --y revenue --geom line -o rev.png

# aggregate with SQL (reads parquet globs), faceted bars
ggplot-rs --sql "SELECT region, sum(qty) q FROM 'orders/*.parquet' GROUP BY 1" \
  --x region --y q --geom col --facet-wrap region --theme minimal -o orders.svg
```

Flags: `--x/--y/--color/--fill/--size/--shape/--group`, `--geom`, `--facet-wrap/--facet-grid`,
`--log-x/--log-y/--flip`, `--title/--subtitle/--xlab/--ylab/--caption`, `-o FILE`/`--stdout`,
`--width/--height`. Run `--describe` to list a source's columns and types.

**Theming from the CLI:** `--theme <preset>` (gray/bw/minimal/…), `--palette <name>`
(Set1/Dark2/viridis/RdBu/…), `--primary "r,g,b"` (brand color), and `--theme-config <file>`
— a TOML/JSON file of element overrides for full custom theming:

```sh
ggplot-rs --parquet d.parquet --x a --y b --color g --palette Dark2 --primary "26,153,136" -o p.png
ggplot-rs --parquet d.parquet --x a --y b --color g --theme-config brand.toml -o p.png
```

```toml
# brand.toml — applied on top of the base preset
base = "minimal"
palette = "RdBu"
primary = [200, 60, 40]
[title]
size = 22
color = [40, 40, 90]
[panel_background]
fill = [248, 246, 240]
[panel_grid_major]
linetype = "dashed"
[legend]
position = "inside"
x = 0.9
y = 0.9
```

**AI-ready:** the repo ships a Claude Code skill at `.claude/skills/plot-data/` that
teaches an agent the describe-then-map-then-render workflow, so "plot this parquet"
just works.

## Data Input

`GGPlot::new` accepts anything implementing the `GGData` trait. Nothing here
requires polars — pick whichever source fits your stack.

**Plain Rust — zero optional dependencies:**

```rust
// Column-oriented
let cols: Vec<(String, Vec<Value>)> = vec![
    ("x".into(), vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)]),
    ("y".into(), vec![Value::Float(4.0), Value::Float(5.0), Value::Float(6.0)]),
];
GGPlot::new(cols)

// Row-oriented
let rows: Vec<HashMap<String, Value>> = vec![/* ... */];
GGPlot::new(rows)
```

**Apache Arrow / DuckDB** — feed a `RecordBatch` straight from a DuckDB query
result, with polars switched off:

```toml
# Cargo.toml — no polars in the dependency tree
ggplot-rs = { version = "0.9", default-features = false, features = ["arrow"] }
```

```rust
let batch: arrow::record_batch::RecordBatch = /* DuckDB query → Arrow */;
GGPlot::new(batch)
```

**polars** (optional, enabled by default) — for `df!` and polars pipelines:

```rust
let df = df! {
    "x" => [1.0, 2.0, 3.0],
    "y" => [4.0, 5.0, 6.0],
}?;
GGPlot::new(df)
```

## Rendering

Save to a file (format inferred from the extension — `svg`, `png`, `jpg`, ...):

```rust
plot.save("out.svg")?;              // 800x600 default
plot.save_with_size("out.png", 1200, 800)?;
plot.ggsave("out.png", 6.0, 4.0, 150.0)?; // width_in, height_in, dpi
```

Or render in memory — no temp files — which is what you want when serving charts
from a web/MCP service:

```rust
let svg: String   = plot.clone().render_svg()?;          // or render_svg_with_size(w, h)
let png: Vec<u8>  = plot.render_png_with_size(400, 300)?; // fully-encoded PNG bytes
```

**Headless / no system fonts.** Rendering uses plotters' `ab_glyph` text backend
with a **bundled font** (DejaVu Sans), not `font-kit`/fontconfig — so text renders
deterministically in a minimal container with no system fonts installed. Nothing
to configure; there is no dependency on the host's font stack.

## Theming & brand color

Everything about a theme is set at **runtime**, so one render process can serve
many tenants' brands without touching chart code.

Inject a **brand/primary color** — it becomes the default for any single-series
geom that has no `color`/`fill` aesthetic mapped (an explicit mapping always wins):

```rust
GGPlot::new(data)
    .aes(Aes::new().x("day").y("count"))
    .geom_col()
    .primary_color((26, 153, 136)) // DataZoo teal — no per-chart color code
    .render_svg()?;
```

Build a whole `Theme` at runtime and compose the brand into it:

```rust
let theme = theme_minimal().with_primary((26, 153, 136));
GGPlot::new(data).aes(/* … */).geom_line().theme(theme);
```

Supply an **arbitrary sequential ramp** (e.g. a green→red risk score) instead of
the built-in viridis/brewer scales — pass explicit `(offset, color)` stops:

```rust
GGPlot::new(data)
    .aes(Aes::new().x("x").y("y").color("risk"))
    .geom_point()
    .scale_color_gradientn(vec![
        (0.0, RGBAColor::new(0, 160, 80)),   // low  = green
        (0.5, RGBAColor::new(240, 200, 0)),  // mid  = amber
        (1.0, RGBAColor::new(200, 40, 40)),  // high = red
    ]);
```

## Feature Flags

| Feature      | Default | Provides                                                    |
| ------------ | :-----: | ----------------------------------------------------------- |
| `polars`     |   yes   | `impl GGData for polars::DataFrame` + `polars` re-export     |
| `arrow`      |   no    | `impl GGData for arrow::RecordBatch` (Arrow/DuckDB input)    |
| `regression` |   no    | `stat_quantile`/`geom_quantile` + `geom_smooth` glm/rlm via anofox-regression |
| `serde`      |   no    | `theme::config::ThemeConfig` — a serde-deserialisable partial theme overlay (TOML/JSON) |
| `cli`        |   no    | the `ggplot-rs` command-line tool (parquet/CSV/DuckDB → SVG/PNG), via clap + bundled DuckDB |

To skip the heavy polars dependency (e.g. an Arrow-only service), disable defaults:

```toml
ggplot-rs = { version = "0.9", default-features = false, features = ["arrow"] }
```

## Examples

Run any example with:

```sh
cargo run --example scatter
cargo run --example histogram
cargo run --example bar_chart
cargo run --example continuous_color
cargo run --example density
cargo run --example faceted
cargo run --example loess_smooth
cargo run --example annotations
cargo run --example coord_flip
cargo run --example log_scale
cargo run --example color_palettes
cargo run --example gallery            # regenerates the gallery above
cargo run --example supplier_leadtime  # polars-free; runs with --no-default-features
```

## Dependencies

- [plotters](https://crates.io/crates/plotters) 0.3 — SVG/PNG rendering (`ab_glyph` text backend; no fontconfig)
- [image](https://crates.io/crates/image) 0.24 — in-memory PNG encoding
- [indexmap](https://crates.io/crates/indexmap) 2 — ordered maps for internal data
- [rand](https://crates.io/crates/rand) 0.8 — jitter positioning
- [polars](https://crates.io/crates/polars) 0.46 — DataFrame input *(optional, default)*
- [arrow](https://crates.io/crates/arrow) 53 — Arrow `RecordBatch` input *(optional)*
- [clap](https://crates.io/crates/clap) 4 + [duckdb](https://crates.io/crates/duckdb) 1 (bundled) — the `cli` tool *(optional)*

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

### Bundled font

Bundled fonts in `assets/fonts/` — DejaVu **Sans** (+ Bold/Oblique), **Serif** (+ Bold/Italic), and **Sans Mono** (+ Bold) — give headless rendering for `family = "serif"`/`"monospace"` and bold/italic (`element_text(face=)`) with real glyphs, no fontconfig. DejaVu Sans
is distributed under a permissive, freely-redistributable license (Bitstream Vera
+ Arev) — see [`assets/fonts/LICENSE-DejaVu.txt`](assets/fonts/LICENSE-DejaVu.txt).
