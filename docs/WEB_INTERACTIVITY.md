# Web interactivity roadmap — toward ECharts-class dashboards

ggplot-rs already renders in the browser (WASM): vector **SVG** for normal charts
(via the plotters-free `SvgBackend`) and a **raster canvas** for large-N. Data is
fed by DuckDB-Wasm. What ECharts has that we don't is an **interaction runtime**:
tooltips, hover-highlight, zoom/pan, brush+link, legend toggling, animation,
responsive resize — available on every chart out of the box.

We won't clone ECharts feature-for-feature (it's a charting widget; ggplot-rs is a
grammar-of-graphics compiler). The goal is **interaction parity on the 20% of
features that cover 80% of use**, while keeping our differentiators: the grammar
(composability + correctness), one Rust codebase for native + web + server, and
DuckDB for data.

The key architectural lever is to **expose the scene model** from WASM — the panel
rect(s), each trained scale's invertible domain↔range mapping, the legend
geometry, and per-mark metadata — so JS can hit-test, invert coordinates, and
highlight *generically* instead of via bespoke per-demo code.

## Status

| Capability | ECharts | ggplot-rs web | Notes |
|---|---|---|---|
| Tooltips (hover) | ✅ | ✅ | `<title>` → styled floating tooltip |
| Hover highlight | ✅ | ⚠️ | CSS on SVG marks; not generalized |
| Zoom / pan (roam) | ✅ | ✅ (maps) | re-render with `xlim`/`ylim`; axes update |
| Brush select | ✅ | ✅ (scatter) | rectangular brush → selection alpha |
| Linked views | ✅ | ✅ | brush → linked bar; map click → drill-down |
| Legend toggle | ✅ | ✅ | clickable chips; stable colours via `color_levels` |
| Responsive resize | ✅ | ✅ (maps) | `ResizeObserver` → re-render at container width |
| Animation / transitions | ✅ | ❌ | hardest for a re-render model |
| Large-N (WebGL) | ✅ (echarts-gl) | ✅ | 1M points on the GPU; Rust axes frame + JS shader |
| Toolbox (export/reset) | ✅ | ⚠️ | save-as-SVG/PNG on gallery charts; reset via dbl-click |
| Declarative `setOption` | ✅ | ⚠️ | per-function JSON specs; unify later |

## Milestones

- **M0 — Scene model (the lever).** After build+layout, return panel rect(s),
  scale domain↔range (invertible), legend boxes, and per-mark metadata (id, geom,
  data row, bbox, color). Unlocks generic hit-testing/highlight and replaces
  bespoke JS. *Partly done*: the raster scatter already returns the pixel↔data
  mapping; `geo_bounds` exposes map extent.
- **M1 — Navigation (roam). _Done for maps._** Scroll-to-zoom + drag-to-pan +
  double-click reset; `render_geo` clips to an `xlim`/`ylim` window and axes
  update. Next: roam on the cartesian charts (scatter) and cursor-accurate zoom
  using the panel rect from M0.
- **M2 — Responsive resize. _Done for maps._** `ResizeObserver` re-renders the
  maps at their container width (and when a tab becomes visible). Next: the
  raster scatter (re-raster on resize, rebinding the brush mapping).
- **M3 — Legend interactivity. _Done._** Clickable legend chips toggle series
  (re-render filtered); the new `render_plot` `color_levels` option keeps each
  series' colour stable as others are toggled. `ScaleColorDiscrete::with_levels`
  backs it. Next: a toolbox (reset-view, save-PNG/SVG).
- **M4 — Unified interaction runtime (`GGView`).** One JS wrapper standardizing
  tooltip, highlight, brush, roam, and `on(event)` across all charts, driven by
  the M0 scene model — so new charts get interactions for free.
- **M5 — Transitions.** Stable mark identity across renders (keyed by data id) →
  tween position/color/opacity between states (enter/update/exit).
- **M6 — Scale + breadth.** WebGL `DrawBackend` for >1M interactive points;
  `visualMap`-style interactive color legend; fill chart-type gaps ECharts is
  known for (candlestick, calendar heatmap, parallel coordinates, sankey, graph,
  treemap/sunburst, gauge, radar).

## Principles

- **Re-render, don't fake it.** Zoom/brush/filter re-run the Rust pipeline (a map
  is ~16 ms) so axes/scales/legends stay correct — the grammar advantage. Reserve
  DOM/canvas tricks for animation only.
- **Fail-isolated panels.** Each chart initialises independently; one failure is
  reported in place, never blanks the page.
- **Verify natively.** Every rendering change is checked with a native
  `render_svg_native`/raster snapshot before it ships to the browser.

## Mark attribute contract (0.20.0)

Every drawn mark on the native SVG backend carries a small, predictable set of
`data-*` attributes and (usually) a `<title>` hover tooltip — the substrate the
JS runtime above hit-tests and reads from, without per-chart glue code.

**Built-in, set by the geom:**

- `data-x` — the row's x-axis key (for axis-pointer-style grouping).
- `data-series` — the row's colour/fill/group level.
- `data-value` — the row's raw (unformatted) measured value.
- `<title>` — a geom-computed hover tooltip (e.g. `"(2.1, 4.3)"`, `"group:
  12.4"`).

**`data-part` — sub-marks of a larger geom.** A decorative sub-mark (so far,
only an arrowhead on segment/curve/path/line — `arrow()`) carries
`data-part="arrow"` and `aria-hidden="true"`, and never a `<title>`: the
parent mark already carries the semantic tooltip, and assistive tech should
skip the decoration. `[data-part="arrow"]` is a stable CSS/JS selector for
styling or excluding these sub-marks.

**`Aes::tooltip(col)` — override the tooltip.** Maps a column to the mark's
`<title>`, replacing whatever the geom would have computed. Escaped like any
other text. Dropped by a computing stat (e.g. `stat_bin`) unless the stat is
`StatIdentity`, since the column isn't part of its output — the same rule as
`linetype`/`alpha`.

**`Aes::data_attr(name, col)` — a custom `data-*` attribute.** Maps a column
to `data-<name>` on the mark. `name` must match `[a-z0-9][a-z0-9-]*`
(lowercase alphanumerics and hyphens, like a real HTML dataset key); an
invalid name is skipped with a build warning rather than failing the render.
A `data_attr` with the same name as a built-in (`data-x`, `data-series`,
`data-value`) **replaces** it — an element never carries the same attribute
name twice.

```rust
GGPlot::new(flows)
    .aes(Aes::new().x("x").y("y").xend("xend").yend("yend")
        .tooltip("label")
        .data_attr("orig", "origin_city")
        .data_attr("row", "id"))
    .geom_curve_with(GeomCurve::default().arrow(arrow().closed()))
    .render_svg_native_with_size(800, 600)?;
```

renders e.g. `<path d="M.. Q.." data-x="…" data-series="…" data-orig="Berlin"
data-row="42"><title>Berlin → Warsaw</title></path>`, with its arrowhead as
`<polygon data-orig="Berlin" data-row="42" data-part="arrow"
aria-hidden="true"/>` (no `<title>`).

**Layers that map neither `tooltip` nor any `data_attr` render byte-identical
output** — the mark-attribute machinery (`geom::set_row_extras`) is a no-op
when the built data has neither column.
