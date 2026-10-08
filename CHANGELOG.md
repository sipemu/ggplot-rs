# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased] — 0.16.0

### Breaking changes

- **The library is now an `rlib` only** (was `["cdylib", "rlib"]`), which
  unblocks static / emscripten side-module consumers. The wasm-bindgen browser
  exports (`ggplot_rs::wasm::*`: `render_geo`, `render_plot`, `render_bar`,
  `render_hist`, `geo_bounds`, `scatter_frame`, `render_scatter_rgba`,
  `render_scatter_xy`) moved to the new, unpublished workspace crate
  `crates/ggplot-rs-wasm` (`cdylib`). Build the browser bundle with
  `wasm-pack build crates/ggplot-rs-wasm --target web --out-dir ../../web/pkg --out-name ggplot_rs`.
  The library no longer depends on `wasm-bindgen`.
- The `wasm` feature is now a **deprecated alias for `sf`** (kept so
  `features = ["wasm"]` dependents keep compiling); it no longer adds
  `wasm-bindgen`/`serde_json`.
- New default-on feature **`plotters`** (implies the new `png` feature) gates
  the plotters/`image`-backed APIs: `GGPlot::{render_svg, render_svg_with_size,
  render_png, render_png_with_size, save, save_with_size, ggsave}`,
  `ggpubr::{ggarrange_png, ggarrange_save_png}` and
  `render::plotters_backend`. Crates using `default-features = false` that call
  these must add `features = ["plotters"]`; the self-contained
  `render_svg_native*` path is always available. `canvas` now implies `png`.
  `cli` implies `plotters`.
- `DataFrame::add_column` no longer panics on a length mismatch: it pads the
  shorter side with `Value::Na` and records an issue
  (`DataFrame::issues()` / `validate()`); `GGPlot::try_build` and every
  `render_*`/`save*` then return `GGError::ValidationError`. This is reachable
  from `GGPlot::new(Vec<(String, Vec<Value>)>)` with ragged columns, which used
  to panic. New `DataFrame::try_add_column` rejects the column instead.
- `DataFrame::group_by` / `unique_values` keep `Value::Na` distinct from the
  literal string `"NA"` (they used to merge into one group).
- `label_number(…, prefix, …)` is now sign-aware: `-5` with prefix `"€"`
  renders `"-€5"` (was `"€-5"`), and a value rounding to zero drops its sign.
- `PositionJitter` gained a `seed: u64` field, and `width`/`height` are now
  `Option<f64>` (`None` = 0.4 × the data's resolution, as in ggplot2). Build it
  with `PositionJitter::new(w, h)`, `PositionJitter::default()`, `.with_width()`,
  `.with_height()` and `.with_seed()` instead of a struct literal.
  `GeomJitter::{width, height}` are likewise `Option<f64>` (default `None`).
- Jitter and `mean_cl_boot` now draw from an internal SplitMix64 stream instead
  of `rand::StdRng`: output is still deterministic, but the exact jittered
  positions / bootstrap intervals differ from 0.15.

### Added

- `ggplot_rs::format::format_value(&Value) -> String` — the public tooltip /
  data-label value formatter (previously the crate-private `geom::tip_value`);
  `ggplot_rs::format` also re-exports the `label_*` formatters.
- `label_currency(prefix, suffix, big_mark, accuracy)` — sign-aware currency
  labels with a configurable thousands mark (`"-€1.234.567"`). `label_dollar`
  is now implemented on top of it (output unchanged).
- `tests/native_svg.rs`: smoke coverage of the plotters-free SVG path across
  geoms/coords/facets/themes, run in every feature configuration.

- `Value::group_key() -> GroupKey` (injective w.r.t. missingness, borrows
  strings) and `Value::key_str() -> Cow<str>` (allocation-free form of
  `to_group_key`).
- `DataFrame::try_add_column`, `DataFrame::issues`, `DataFrame::validate`.

### Changed

- Minimal dependency tree (`default-features = false, features = ["sf"]`):
  38 crates → 3 (`indexmap`, `equivalent`, `hashbrown`). No plotters, image,
  ab_glyph, chrono, rand or getrandom.
- Removed the `rand` dependency (and the `wasm32`-only `getrandom` dependency)
  from the library; randomness comes from a small internal seeded PRNG.
- Default jitter amount is resolution-based (ggplot2's `0.4 * resolution(x)`):
  integer-spaced data keeps ±0.4, finely spaced continuous data is no longer
  smeared by a fixed ±0.4 data units.

### Fixed

- Empty manual scales (`scale_color_manual(vec![])`, `scale_fill_manual`,
  `scale_shape_manual`, `scale_linetype_manual`) no longer panic with a
  division by zero; the geom falls back to its default colour/shape/linetype.
- `format_epoch_secs` is total over `i64` (it overflowed for `i64::MIN`).
- `stat_binhex` / `geom_hex` output order is deterministic (row-major) instead
  of following `HashMap` iteration order, so identical data renders identical
  SVG.
- Discrete scale training/lookups no longer allocate a `String` per value.

- `label_number` / `label_currency` with an `accuracy` that keeps decimals no
  longer inserts thousands marks into the fractional part
  (`1234.5` @ 0.01 → `"1,234.50"`, was `"1,23,4.50"`-style garbage).
