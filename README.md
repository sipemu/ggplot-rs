# ggplot-rs

[![CI](https://github.com/sipemu/ggplot-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/sipemu/ggplot-rs/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/ggplot-rs.svg)](https://crates.io/crates/ggplot-rs)
[![Documentation](https://docs.rs/ggplot-rs/badge.svg)](https://docs.rs/ggplot-rs)
[![codecov](https://codecov.io/gh/sipemu/ggplot-rs/branch/main/graph/badge.svg)](https://codecov.io/gh/sipemu/ggplot-rs)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](#license)

A Rust implementation of ggplot2's Grammar of Graphics, built on top of [polars](https://pola.rs/) DataFrames and the [plotters](https://github.com/plotters-rs/plotters) rendering backend.

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

### Geoms (18)

`geom_point`, `geom_line`, `geom_bar`, `geom_col`, `geom_histogram`, `geom_boxplot`, `geom_smooth`, `geom_density`, `geom_area`, `geom_ribbon`, `geom_errorbar`, `geom_segment`, `geom_rug`, `geom_text`, `geom_label`, `geom_hline`, `geom_vline`, `geom_abline`

### Stats (10)

`StatIdentity`, `StatCount`, `StatBin`, `StatBoxplot`, `StatSmooth` (Lm + Loess), `StatDensity`, `StatLoess`, `StatSummary`, `StatEcdf`, `StatFunction`

### Scales

- **Continuous**: linear, log10, sqrt, reverse transforms
- **Discrete**: automatic categorical mapping
- **Color**: discrete palettes (Viridis, Brewer Set1/Dark2, etc.), continuous gradients, diverging gradient2, manual color assignment
- **Shape & Linetype**: discrete mapping for point shapes and line styles

### Coordinates

`coord_cartesian`, `coord_flip`, `coord_fixed`

### Faceting

`facet_wrap` and `facet_grid` with configurable free/fixed scales

### Themes

`theme_gray`, `theme_bw`, `theme_classic`, `theme_minimal`, `theme_dark`, `theme_light`, `theme_linedraw`, `theme_void` — plus full customization via `ElementText`, `ElementLine`, `ElementRect`

### Annotations

`annotate_text`, `annotate_rect`, `annotate_segment`

## Data Input

The primary input type is `polars::DataFrame`:

```rust
let df = df! {
    "x" => [1.0, 2.0, 3.0],
    "y" => [4.0, 5.0, 6.0],
}?;
GGPlot::new(df)
```

Row-oriented and column-oriented inputs are also supported:

```rust
// Row-oriented
let rows: Vec<HashMap<String, Value>> = vec![/* ... */];
GGPlot::new(rows)

// Column-oriented
let cols: Vec<(String, Vec<Value>)> = vec![/* ... */];
GGPlot::new(cols)
```

Arrow-native producers (e.g. DuckDB) can feed a `RecordBatch` directly with the
`arrow` feature:

```rust
// Cargo.toml: ggplot-rs = { version = "0.1", features = ["arrow"] }
let batch: arrow::record_batch::RecordBatch = /* ... */;
GGPlot::new(batch)
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

## Feature Flags

| Feature  | Default | Provides                                                        |
| -------- | :-----: | --------------------------------------------------------------- |
| `polars` |   yes   | `impl GGData for polars::DataFrame` + `polars` re-export         |
| `arrow`  |   no    | `impl GGData for arrow::RecordBatch` (Arrow/DuckDB input)        |

To skip the heavy polars dependency (e.g. an Arrow-only service), disable defaults:

```toml
ggplot-rs = { version = "0.1", default-features = false, features = ["arrow"] }
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
```

## Dependencies

- [plotters](https://crates.io/crates/plotters) 0.3 — SVG/PNG rendering
- [image](https://crates.io/crates/image) 0.24 — in-memory PNG encoding
- [indexmap](https://crates.io/crates/indexmap) 2 — ordered maps for internal data
- [rand](https://crates.io/crates/rand) 0.8 — jitter positioning
- [polars](https://crates.io/crates/polars) 0.46 — DataFrame input *(optional, default)*
- [arrow](https://crates.io/crates/arrow) 53 — Arrow `RecordBatch` input *(optional)*

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
