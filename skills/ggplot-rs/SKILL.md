---
name: ggplot-rs
description: Build ggplot2-style charts in Rust with the `ggplot-rs` crate (Grammar of Graphics) — data → aes → geoms → scales/facets/coords/theme → SVG/PNG. Use when writing or fixing Rust code that plots data, when a Cargo.toml depends on `ggplot-rs`, or when asked to "make a chart in Rust", port ggplot2/R plotting code to Rust, or render charts server-side from Rust.
---

# Plotting in Rust with `ggplot-rs`

`ggplot-rs` is ggplot2's Grammar of Graphics as a Rust builder:
`GGPlot::new(data).aes(..).geom_*()..` then render. Computed layers (bins,
densities, loess, boxplot stats, stacking, axis breaks) are validated against R
ggplot2 4.x, so translate R code one-to-one.

API reference: <https://docs.rs/ggplot-rs>. Every example below is compiled by
the crate's test suite, so it matches the published API.

## Cargo.toml

```toml
[dependencies]
ggplot-rs = "0.40"            # defaults: polars input + plotters PNG/SVG files
# Lean, no polars/plotters (self-contained SVG only, also builds for wasm32):
# ggplot-rs = { version = "0.40", default-features = false, features = ["sf"] }
```

| Feature | Adds |
|---|---|
| `polars` (default) | `GGPlot::new(polars_df)` |
| `plotters` (default) | `save`, `save_with_size`, `ggsave`, `render_png*`, `render_svg*` |
| `arrow` | `GGPlot::new(record_batch)` (DuckDB/Arrow results) |
| `sf` | `geom_sf`/`coord_sf` maps from WKT, projections |
| `geojson` | `spatial::geojson::read_geojson_*` |
| `regression` | `geom_quantile`, `geom_smooth` glm/rlm |
| `ggpubr` | `stat_cor`, `stat_compare_means` annotations |
| `canvas` | raster RGBA backend for very large N (`render_rgba_with_size`) |
| `spec` | `ggplot_rs::spec` — build a plot from the JSON spec the CLI/npm package use |
| `cli` | the `ggplot-rs` binary (see the `ggplot-rs-cli` skill) |

## Data

`GGPlot::new` takes anything implementing `GGData`: column vectors, row maps,
a polars `DataFrame`, or an Arrow `RecordBatch`. `Value` is
`Float | Integer | Str | Bool | DateTime(secs) | Na`. Strings/bools are
categorical (discrete scales); numbers are continuous.

```rust
use ggplot_rs::prelude::*;

let f = |v: &[f64]| v.iter().map(|x| Value::Float(*x)).collect::<Vec<_>>();
let s = |v: &[&str]| v.iter().map(|x| Value::Str(x.to_string())).collect::<Vec<_>>();
let data = vec![
    ("wt".to_string(), f(&[2.6, 2.9, 3.2, 3.4, 3.6, 4.1])),
    ("mpg".to_string(), f(&[21.0, 22.8, 21.4, 18.7, 18.1, 14.3])),
    ("cyl".to_string(), s(&["4", "4", "6", "6", "8", "8"])),
];

let svg: String = GGPlot::new(data)
    .aes(Aes::new().x("wt").y("mpg").color("cyl"))
    .geom_point()
    .geom_smooth()
    .title("Fuel economy")
    .xlab("Weight")
    .ylab("MPG")
    .theme_minimal()
    .render_svg_native_with_size(640, 400)?;
assert!(svg.starts_with("<svg"));
# Ok::<(), ggplot_rs::GGError>(())
```

With polars (default feature): `GGPlot::new(df!{ "x" => [1.0, 2.0], "y" => [3.0, 4.0] }?)`.

## The grammar

- **Aesthetics:** `Aes::new().x(..).y(..).color(..).fill(..).size(..).shape(..)
  .alpha(..).group(..).linetype(..).label(..).weight(..)`, intervals
  `ymin/ymax/xmin/xmax/xend/yend`, finance `open/high/low/close`, graphs
  `from/to`, hover `tooltip`. Plot-level `.aes()` is inherited; `.layer_aes(..)`
  right after a geom overrides it for that layer; `.layer_data(..)` gives a
  layer its own data.
- **Geoms:** `geom_point line path step area ribbon col bar histogram density
  boxplot violin smooth text label errorbar pointrange segment rect tile raster
  hex bin2d contour hline(y) vline(x) abline(slope, icpt)` and ~60 more
  (waterfall, sankey, treemap, sunburst(levels), candlestick, sf, …). Each has a
  `geom_*_with(GeomX::default()...)` form for parameters:
  `geom_histogram_with(GeomHistogram::default().with_bins(20))`,
  `geom_smooth_with(GeomSmooth::default().loess(0.75))`,
  `geom_boxplot_with(GeomBoxplot::default().notched().varwidth().with_coef(1.0))`,
  `geom_violin_with(GeomViolin::default().with_scale(ViolinScale::Count)
  .untrimmed().with_quantiles([0.25, 0.5, 0.75]))`.
- **Function / extra stats:** `geom_function(|x| x.sin())`,
  `geom_function_xlim(f, lo, hi)` (no data needed), `stat_connect(Connection::Mid)`,
  `stat_unique()`, `stat_manual(|df| …)`, `stat_summary_hex(SummaryFun::Mean)`
  (raw `z` column), `stat_sf_coordinates()` (feature `sf`).
- **Stat / position overrides** apply to the last layer: `.stat(StatIdentity)`,
  `.position(PositionDodge::new(0.9))` (`.with_preserve_single()` keeps
  widths in partial slots), `PositionDodge2`, `PositionStack`, `PositionFill`,
  `PositionJitter::new(w, h)`, `PositionNudge::new(x, y)`.
- **Scales:** `scale_x_log10()`, `scale_x_continuous(ScaleContinuous::new()
  .with_limits(0.0, 10.0).with_breaks(vec![..]).with_transform(ScaleTransform::Sqrt)
  .with_label_formatter(label_percent(None)))` (`.with_n_breaks(10)` = `n.breaks`),
  `scale_x_time()` (seconds → `HH:MM:SS`), `scale_color_brewer(PaletteName::Dark2)`,
  `scale_fill_manual(vec![("a", RGBAColor::new(230, 159, 0))])`,
  `scale_color_gradient(lo, hi)`, `scale_color_gradientn(stops)`,
  `scale_color_hue(HuePalette::default())` (ggplot2's default colours for any
  number of groups), `scale_color_viridis_d('D', 0.0, 1.0, 1)` (options A–H),
  `scale_color_distiller(PaletteName::Blues, -1)`; `scale_x_binned(10)` (bin x before the stat —
  `geom_bar()` → histogram), `scale_size_binned(5)` / `_alpha_` / `_shape_` / `_linetype_` / `_linewidth_binned`; binned `scale_fill_steps2(lo, mid,
  hi, midpoint, n_bins)`, `scale_color_viridis_b('C', n_bins)`; `scale_size_manual(vec![("a",
  2.0)])` / `scale_alpha_manual` / `scale_linewidth_manual`, `scale_size_identity()` /
  `_alpha_` / `_linewidth_` / `scale_shape_identity()` (pch codes), `scale_radius(1.0, 8.0)`;
  `expand_limits(&[], &[0.0])`.
- **Secondary axes:** `scale_x_continuous(ScaleContinuous::new().with_sec_axis(SecAxis::new(|c|
  c * 1.8 + 32.0).with_name("°F")))` (x: on top, y: on the right; `SecAxis::dup()` =
  `dup_axis()`, `SecAxis::linear(scale, offset)` own nice ticks).
- **Facets:** `facet_wrap("col", Some(ncol))`, `facet_grid(Some("row"), Some("col"))`,
  `facet_wrap_free(col, ncol, FacetScales::FreeY)`, `.facet_wrap_dir(vertical,
  as_table)` (ggplot2's `dir = "v"`, `as.table`),
  `facet_wrap_labeller(col, ncol, FacetLabeller::Wrap(12))` (`label_wrap_gen`),
  `.facet_labeller(FacetLabeller::per_var(vec![("am", FacetLabeller::lookup(vec![("0", "auto")]))]))`
  / `FacetLabeller::Template("n = {value}".into())`,
  `.facet_strip_position(StripPosition::Bottom)` (wrap), `.facet_switch(FacetSwitch::Y)`
  (grid: row strips on the left; by default column strips are on top, row strips right).
  `.facet_axes(FacetAxes::all().margin_labels())` (axes on every panel; ggplot2's
  `axes = "all", axis.labels = "margins"`).
  `.facet_grid_levels(Some(&["b", "a", "z"]), None::<&[&str]>)` (order, keep empty),
  `.facet_margins(true)` ("(all)" panels; ggplot2's `margins = TRUE`).
- **Coords:** `coord_flip()`, `coord_polar()`, `coord_radial(start, end,
  inner_radius)`, `coord_fixed(ratio)`, `coord_cartesian_zoom(Some((lo, hi)), None)`,
  `coord_reverse(x, y)`, `coord_cartesian_with(CoordCartesian::new().expand(t, r, b, l))`, `coord_clip(false)` (`clip = "off"`; layers are clipped to the panel by default)
  (`expand = FALSE` per side), `coord_polar_with(CoordPolar::new().reverse(theta, r)
  .rotate_angle(true).r_axis_inside(true))`, `coord_trans`/`coord_transform(x, y)`, `coord_quickmap()`,
  `coord_map()` (Mercator, plain lon/lat), `coord_radar()`, `coord_sf()`.
- **Annotations:** `annotate_text/rect/segment`, `annotation_logticks("bl")` (or on the axis:
  `.scale_x_log10().guide_axis_logticks(Aesthetic::X)`)
  (with `scale_*_log10()`), `annotation_raster(png_bytes, xmin, xmax, ymin, ymax)`,
  `annotation_custom(inset_plot, xmin, xmax, ymin, ymax)` (a plot inside a
  plot; `f64::INFINITY` bounds = panel edges).
- **Legend keys:** each layer draws its own glyph (points, lines, boxes …)
  and layers on one mapping share a key; `.key_glyph(KeyGlyph::Timeseries)`
  and `.show_legend_for(Aesthetic::Fill, false)` modify the last layer.
- **Labels:** `title subtitle caption tag xlab ylab`. Axis titles are **not**
  taken from column names automatically — set `xlab`/`ylab` when you want them.
- **Theme:** `.theme_minimal()` / `.theme_bw()` / `.theme_classic()` / `.theme_test()` … or
  `.theme(theme_minimal().with_primary((26, 153, 136)))`; tweak one property with
  `.theme_with(|t| t.axis_text_x.size = 14.0)` (ggplot2's `+ theme()`), whole elements via
  `.theme_update(theme_sub_axis_x(Some(el), None, None, None))` (`theme_sub_*`, `%+replace%`); legend via
  `.legend_position(LegendPosition::Bottom)` (call **after** a theme preset —
  presets replace the whole theme); per aesthetic `.guide_none(Aesthetic::Fill)`
  and `.guide_title(Aesthetic::Color, "Group")`, `.guide_reverse(Aesthetic::Color, true)`,
  `.guide_override(Aesthetic::Color, KeyOverride::new().size(4.0).alpha(1.0))` (`override.aes`);
  a continuous colour bar: `.guide_coloursteps(Aesthetic::Fill)` or
  `.guide_colourbar(Aesthetic::Fill, GuideColourbar::new().with_bar_height(150.0)
  .with_nbin(10).with_display(ColourbarDisplay::Rectangles).with_ticks(false))`;
  a binned scale (`scale_size_binned(5)` …) draws `guide_bins` — keys labelled at the
  bin edges — tune with `.guide_bins(Aesthetic::Size, GuideBins::new().with_show_limits(true))`
  or get one `lo–hi` key per bin with `.guide_legend_keys(Aesthetic::Size)`.
  One legend at its own side: `.guide_position(Aesthetic::Shape, LegendPosition::Bottom)`.
  Your own legend entry: `.guide_custom(GuideCustom::new(w, h, |backend, rect| { … }).with_title("…"))`.

```rust
use ggplot_rs::prelude::*;

let data = vec![
    ("q".to_string(), ["Q1", "Q1", "Q2", "Q2"].map(|s| Value::Str(s.into())).to_vec()),
    ("seg".to_string(), ["a", "b", "a", "b"].map(|s| Value::Str(s.into())).to_vec()),
    ("rev".to_string(), [3.0, 2.0, 4.0, 2.5].map(Value::Float).to_vec()),
];
let svg = GGPlot::new(data)
    .aes(Aes::new().x("q").y("rev").fill("seg"))
    .geom_col()
    .position(PositionDodge::new(0.9))
    .scale_fill_brewer(PaletteName::Dark2)
    .coord_flip()
    .theme_bw()
    .legend_position(LegendPosition::Bottom)
    .render_svg_native()?;
assert!(svg.contains("<rect"));
# Ok::<(), ggplot_rs::GGError>(())
```

## Helpers

`GGPlot::qplot(data, "x", Some("y"))` (points; histogram / bars with `None`),
`cut_interval(&x, n)` / `cut_interval_length(&x, len)` / `cut_number(&x, n)` /
`cut_width(&x, w, CutWidth::default())` → `Cut { values, levels }` (ggplot2's
`[a,b]` / `(a,b]` labels; use `values` as a discrete column), `resolution(&x,
true)`, and the summary functions `mean_se(&y, 1.0)`, `mean_cl_normal(&y, 0.95)`,
`mean_cl_boot(&y, 0.95, 1000)`, `mean_sdl(&y, 2.0)`, `median_hilow(&y, 0.95)` →
`(y, ymin, ymax)`.

## Rendering

| Call | Needs | Output |
|---|---|---|
| `render_svg_native()` / `_with_size(w, h)` | nothing | SVG `String` (self-contained backend; wasm-ok) |
| `render_svg_native_with_warnings(w, h)` | nothing | `(svg, Vec<String>)` — dropped rows, empty stats |
| `save("p.png")` / `save_with_size(path, w, h)` | `plotters` | file; format from extension (svg/png/jpg/…) |
| `ggsave(path, w_in, h_in, dpi)` | `plotters` | file at physical size |
| `render_png_with_size(w, h)` | `plotters` | PNG bytes (serve over HTTP, no temp file) |
| `build()` | nothing | computed layer data, like R's `ggplot_build()` |

Multi-panel layouts: `PlotGrid::new().add(p1).add(p2).ncol(2).render_svg_native()`.

## From a JSON spec (feature `spec`)

The same JSON spec the CLI (`--spec`) and the npm package (`render()`) accept:

```rust,ignore
let (svg, warnings) = ggplot_rs::spec::render_json(r#"{
  "data": {"x": [1, 2, 3], "y": [2, 4, 3]},
  "aes": {"x": "x", "y": "y"},
  "layers": [{"geom": "point"}, {"geom": "smooth", "params": {"method": "loess"}}]
}"#)?;
```

`spec::geoms()` lists the accepted geom names; `spec::build_plot_with_data`
builds a `GGPlot` over your own columns.

## Workflow & pitfalls

1. Inspect the data first: which columns are numeric vs categorical. A numeric
   column mapped to `color` gets a continuous gradient; convert to `Value::Str`
   for discrete colors.
2. Start from the R/ggplot2 idea, translate each `+ layer` into a builder call.
3. Render with `render_svg_native_with_warnings` while developing and read the
   warnings (they explain empty layers / dropped rows).
4. Errors (`GGError`) name the missing column or aesthetic — fix the mapping,
   don't guess.
- Grouped `geom_boxplot` (dodge2) and `geom_violin` (dodge) sit side by side
  by default; crossbar, pointrange, linerange, errorbar, dotplot, candlestick
  dodge with an explicit `.position(PositionDodge::new(w))`.
- Required aesthetics per geom follow ggplot2 (`geom_bar` needs only `x`;
  `geom_col` needs `x` and `y`; `geom_boxplot` a discrete `x` and numeric `y`).
- Fonts are bundled; rendering needs no system fonts (works in containers).
