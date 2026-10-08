# Changelog

## 0.16.0 (unreleased)

### Rendering robustness

- An empty plot (zero-row data, no columns, or no layers) renders an empty
  panel with its title/axes instead of failing with
  `ValidationError("geom_… requires aesthetic …")`.
- A layer whose stat produces no rows (a density/violin of one value, loess on
  two points, …) draws nothing; the other layers still render.
- Rows with non-finite position values (`NaN`, `±Inf`, or a transform that is
  undefined such as `log10(0)`) are dropped before stats, like ggplot2's
  "Removed n rows containing non-finite values". Stat output is filtered the
  same way.
- Build warnings are exposed: `BuiltPlot::warnings()` and
  `GGPlot::render_svg_native_with_warnings(w, h) -> (String, Vec<String>)`.
- The native SVG backend never writes `NaN`/`inf` into an attribute: marks with
  non-finite geometry are dropped and non-finite style values fall back to
  defaults.
- Density/violin bandwidth follows R's `bw.nrd0`, including its fallbacks for
  zero-spread samples (previously a constant sample produced NaN).
- A position scale's transform now applies to its whole column family
  (`ymin`/`ymax`/`yend`, …), not only `x`/`y`.
- Faceted panels keep per-mark hover metadata (tooltips / `data-x`).

### SVG host integration

- Hoverable marks carry `data-series` (colour/fill/group level) and
  `data-value` (the raw measured value; for stacked/filled bars the segment's
  own value) next to `data-x`. Stacked-bar tooltips now report the segment
  value rather than the cumulative stack top.
- The root `<svg>` carries the trained, expanded position domains:
  `data-domain="x0 x1 y0 y1"` (both axes continuous), `data-xdomain`,
  `data-ydomain`, `data-xlevels`/`data-ylevels` (JSON arrays for discrete axes)
  and `data-flip="true"` under `coord_flip`.
- `GGPlot::render_svg_native_at(x, y, w, h)` renders a nested, positioned
  `<svg x y width height viewBox>` fragment without `xmlns`, for composing
  dashboards without string surgery (`SvgBackend::finish_fragment`).

### Rect bounds

- `-Inf`/`Inf` (`f64::NEG_INFINITY`/`f64::INFINITY`) in `geom_rect` /
  `annotate("rect")` `xmin`/`xmax`/`ymin`/`ymax` extend the rect to the panel
  edge (ggplot2) and do not train scales — also on discrete axes. New
  `Geom::allows_infinite()` hook; other geoms still drop infinite positions.

### Theme / colour

- Theme presets (`theme_minimal()`, `theme_void()`, …) and `theme(t)` keep a
  previously set `primary_color` (unless the new theme sets its own), so call
  order no longer matters.
- `primary_color` no longer overrides colours of geoms added with
  `geom_*_with(...)` (`Layer::explicit_style`); default-configured geoms still
  take the brand colour.
- `ScaleColorDiscrete::sorted()`, `GGPlot::scale_fill_discrete_sorted()` and
  `scale_color_discrete_sorted()`: discrete colour levels in sorted order, so a
  level keeps its palette colour across charts regardless of data order.

### Tiles

- `geom_tile` on a continuous axis is sized by the data resolution (smallest
  gap between distinct values, like ggplot2's `resolution()`); `width`/`height`
  are now multiples of it (default 1 = abutting tiles). The tile extents
  (`xmin`/`xmax`/`ymin`/`ymax`) train the scales, so edge tiles no longer spill
  past the axes and no `with_expand(…)` workaround is needed. New
  `Geom::setup_data()` hook (ggplot2's `setup_data`). Free facet scales also
  train on extent columns.
