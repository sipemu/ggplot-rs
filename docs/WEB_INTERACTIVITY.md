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

## Mark attribute contract (0.21.0)

Every drawn mark on the native SVG backend carries a small, predictable set of
`data-*` attributes, a `class`, and (usually) a `<title>` hover tooltip — the
substrate the JS runtime above hit-tests and reads from, without per-chart
glue code.

**Built-in, set by the geom:**

- `class="gg-mark gg-<kind>"` — every element a layer draws, classed by
  primitive: `gg-point` (circle/point shapes), `gg-line` (polyline, also
  `draw_quadratic`'s native `<path>`), `gg-band` (filled polygon — area,
  ribbon, density, hex…), `gg-bar` (rect), `gg-text`. A decorative sub-mark
  (below) is classed by its *part* instead, e.g. `gg-arrow`. Grid lines, axes,
  legends, strips and backgrounds are drawn outside a layer and stay
  unclassed — `[class^="gg-mark"]` (or the tag) selects data only.
- `data-geom="<geom name>"` — the layer's `Geom::name()`, e.g. `line`,
  `smooth`, `col`, or `hline`/`vline`/`abline` for a reference line.
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
styling or excluding these sub-marks. Its `gg-mark` class is `gg-arrow` (the
part), not its parent's `gg-line`/`gg-band`.

**Non-finite guard.** `draw_line` splits a polyline at a non-finite vertex
into separate marks instead of silently dropping the bad point and joining
its finite neighbours across the gap — `geom_line`/`geom_path`/`geom_step`
treat a non-finite row as a break, matching ggplot2's NA-break handling for
connected lines. A lone finite vertex between two breaks is still its own
(single-point, invisible) mark rather than vanishing, so hover/data
attributes stay consistent across runs.

**`data-fvalue` / `data-fx` — formatted values (opt-in via
`GGPlot::svg_formatted_values(true)`, default off).** `data-fvalue` is the
value aesthetic's raw value (y for most geoms, fill for hex/tile/bin2d)
formatted through that aesthetic's scale (`Scale::format_value`) — a de-DE
currency string, a locale date, whatever the scale's own formatter produces
— instead of the plain display string `data-value` already carries.
`data-fx` is the x value formatted the same way, written only when it
differs from the plain `data-x` (e.g. a datetime x shows its ISO/locale
date). A vline/hline's own `data-x` and tooltip switch to the formatted
value directly when the flag is on, since a raw epoch-seconds `data-x` on a
date reference line isn't useful even unformatted.

**`Aes::tooltip(col)` — override the tooltip per row.** Maps a column to the
mark's `<title>`, replacing whatever the geom would have computed. Escaped
like any other text. Dropped by a computing stat (e.g. `stat_bin`) unless
the stat is `StatIdentity`, since the column isn't part of its output — the
same rule as `linetype`/`alpha`.

**`GGPlot::tooltip(template)` — a `{column}` tooltip template for a whole
layer.** Applies to the most recently added layer. `{column}` is filled from
the *built* (post-stat) data at that row, formatted through the column's
scale when it has one — so `tooltip("{label}: {y}")` on a currency-formatted
y scale renders `<title>Berlin: 1.234,50 €</title>`, and a hex's
`tooltip("{count} points")` reads the stat's own output column. An absent
column renders empty, not literally `{column}`; `{{`/`}}` are literal
braces; the result is escaped the same way any other tooltip is.

**`Aes::data_attr(name, col)` — a custom `data-*` attribute.** Maps a column
to `data-<name>` on the mark. `name` must match `[a-z0-9][a-z0-9-]*`
(lowercase alphanumerics and hyphens, like a real HTML dataset key); an
invalid name — including the reserved `part`, `geom`, `fvalue` and `fx`,
which ggplot-rs itself writes — is skipped with a build warning rather than
failing the render or duplicating the attribute. A `data_attr` with the same
name as a built-in (`data-x`, `data-series`, `data-value`) **replaces** it —
an element never carries the same attribute name twice.

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
data-row="42" class="gg-mark gg-line" data-geom="curve"><title>Berlin →
Warsaw</title></path>`, with its arrowhead as `<polygon data-orig="Berlin"
data-row="42" data-part="arrow" aria-hidden="true" class="gg-mark
gg-arrow"/>` (no `<title>`).

**Layers that map neither `tooltip` nor any `data_attr` render byte-identical
output** — the mark-attribute machinery (`geom::set_row_extras`) is a no-op
when the built data has neither column.

## Root attributes and page-level styling

**`GGPlot::svg_attr(name, value)`/`svg_attrs(iter)` — and the same API on
`PlotGrid`.** Add caller attributes to the root `<svg>` element: a host id,
an ARIA label, a CSS hook. Allow-listed the same way as `data_attr`'s name
(`data-*`/`aria-*`), plus the fixed names `role`, `id`, `class`, `lang`.
Anything else, or a name ggplot-rs already writes to that root (`data-plot`,
`data-domain`/…, `data-grid`, or `data-warnings` when `svg_warnings_attr` is
also on), is dropped with a build warning instead of silently overwriting or
duplicating it.

**`GGPlot::svg_warnings_attr(bool)` (and on `PlotGrid`).** Emits
`data-warnings` on the root `<svg>` as a JSON array of the build warnings
(rows dropped for non-finite positions, a rejected `svg_attr`/`data_attr`
name, …) — `[]` when there are none. Default off, and omitted entirely when
off.

**Theme ink/paper/accent (`Theme.geom`, ggplot2 4.0's `element_geom`).** For
a host that recolours a whole dashboard for a dark theme or a brand palette:
`Theme::with_ink_paper(ink, paper)` (or the finer-grained `set_geom
(ElementGeom::default().with_ink(..).with_paper(..).with_accent(..))`)
recolours a geom's *built-in default* colour — never a mapped or explicit
one — without touching every geom's own colour field. `ink` replaces a
built-in black/`(50, 50, 50)` line, text, outline or legend-key colour;
`paper` replaces a built-in white fill; `accent` replaces the smooth line's
default blue. `theme.geom.family` sits between a geom's own `fontfamily` and
`theme.text.family` in the font-resolution order. Every field `None` (the
default) changes nothing.
