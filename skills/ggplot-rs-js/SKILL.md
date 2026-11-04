---
name: ggplot-rs-js
description: Render ggplot2-style (Grammar of Graphics) charts to SVG in JavaScript/TypeScript — browser or Node — with the `@sipemu/ggplot-rs` npm package (WebAssembly build of the ggplot-rs crate). Use when writing JS/TS that charts data with `@sipemu/ggplot-rs`, when package.json depends on it, or when asked for ggplot-style charts in a web page, Observable, or a Node script without a DOM.
---

# Charts in JS/TS with `@sipemu/ggplot-rs`

A typed, layered **plot spec in → SVG string out**. No runtime dependencies,
no DOM needed (works in Node), the same engine as the `ggplot-rs` Rust crate
and CLI (the npm version always equals the crate version). Statistical layers
match R ggplot2 4.x.

```sh
npm i @sipemu/ggplot-rs
```

## Initialise once, then render

```js
// Browser / Vite / any bundler
import init, { render } from "@sipemu/ggplot-rs";
await init();                       // fetches ggplot_rs_bg.wasm next to the module
document.querySelector("#chart").innerHTML = render(spec);
```

```js
// Node ≥ 18 (no fetch from file: hand the bytes over)
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { initSync, render } from "@sipemu/ggplot-rs";
const require = createRequire(import.meta.url);
initSync({ module: await readFile(require.resolve("@sipemu/ggplot-rs/ggplot_rs_bg.wasm")) });
```

Exports: `render(spec)` → SVG string (throws `Error` naming the bad field),
`renderWithWarnings(spec)` → `{ svg, warnings }`, `geoms()` → accepted geom
names, `version()`. Types: `PlotSpec`, `Layer`, `GeomName`, `LayerParams`, …

## The spec

```js
const svg = render({
  data: { dose: [0.5, 1, 2, 0.5, 1, 2],          // column-oriented, equal lengths
          len: [13.2, 22.7, 26.1, 8.0, 16.8, 26.1],
          supp: ["OJ", "OJ", "OJ", "VC", "VC", "VC"] },
  aes: { x: "dose", y: "len", color: "supp" },   // inherited by every layer
  layers: [
    { geom: "point" },
    { geom: "line" },
    { geom: "hline", params: { yintercept: 20 } },
  ],
  scales: { x: { trans: "log2", name: "Dose (mg)" }, color: { palette: "Dark2" } },
  facet: { wrap: "supp" },
  labels: { title: "Tooth growth", y: "Length" },
  theme: { base: "bw", title: { size: 16, face: "bold" } },
  legend: "bottom",
  width: 640, height: 400,
});
if (!svg.startsWith("<svg")) throw new Error("render failed");
```

| Field | Values |
|---|---|
| `data` | `{ col: (number \| string \| boolean \| null)[] }` — numbers continuous, strings/booleans categorical |
| `aes` / `layers[].aes` | aesthetic → column: `x y color fill size shape alpha group linetype linewidth label weight xmin xmax ymin ymax xend yend angle radius tooltip from to subgroup open high low close lower middle upper fontface class` |
| `layers[].geom` | see Geoms |
| `layers[].params` | `bins` · `method` (`"lm"`/`"loess"`) `span` `se` · `yintercept` `xintercept` `slope` `intercept` · `levels` · `rows` · `box_size` `reversal` · `shape` · `speed` `direction` · `censor` · `projection` · `target` `ranges` · boxplot `notch` `varwidth` `coef` `outliers` `staplewidth` · violin `trim` `scale` (`"area"`/`"count"`/`"width"`) `quantiles` `bw` `adjust` · stat connect `connection` · stat summary_hex `fun` |
| `layers[].position` | `identity stack fill dodge dodge2 jitter nudge`, or `{ type: "dodge", width: 0.8, preserve: "single" }`. boxplot dodges (`dodge2`) and violin (`dodge`) by default, so grouped boxes sit side by side |
| `layers[].stat` | `"identity"` (plot precomputed values, e.g. bar heights), `"unique"` (drop duplicate rows), `"connect"` (steps; `params.connection` `hv`/`vh`/`mid`/`linear`), `"summary_hex"` (`params.fun` of a `z` column per hexagon), `"sf_coordinates"` (a point per WKT `geometry` — text on a map) |
| `layers[].data` | layer-specific data |
| `layers[].key_glyph` | legend key picture: `point path rect polygon boxplot crossbar pointrange linerange smooth text label vline vpath abline timeseries dotplot blank` (default per geom; layers on one mapping share a key) |
| `layers[].show_legend` | `false` hides the layer from legends; `{ color: false }` per aesthetic |
| `scales.x/y` | `{ trans, limits: [lo, hi], breaks: [...], name }`; trans `log10 log2 ln sqrt reverse logit probit pseudo_log reciprocal exp` |
| `scales.color/fill` | `{ palette }` (`Set1 Dark2 Paired npg …`, `hue` = ggplot2's default hues, `viridis:A`…`viridis:H` for categorical; `viridis magma plasma inferno` or any Brewer name (a distiller gradient) for numeric), `{ values: { level: "#rrggbb" } }`, `{ gradient: ["#lo", "#hi"] }`, `{ levels: [...] }` |
| `facet` | `{ wrap, ncol?, scales? }` or `{ rows?, cols?, scales? }`; scales `fixed free free_x free_y` |
| `coord` | `cartesian flip polar radial fixed radar quickmap map sf`, or `{ type, xlim, ylim, ratio, reverse: "x"/"y"/"xy", start, end, inner_radius }` (`quickmap` / `map` = plain lon/lat data; `radial` partial circles / donuts) |
| `labels` | `title subtitle caption tag x y` (axis titles are not auto-filled from column names) |
| `theme` | preset `gray bw minimal classic dark light void linedraw` (default `minimal`), or an overlay `{ base, primary: [r,g,b], palette, title: {...}, panel_grid_major: {...}, … }` |
| `legend` | `right left top bottom none inside` or `[x, y]` inside the panel |

## Geoms

- basic: `point jitter line path step area ribbon stepribbon col bar segment curve
  spoke rect tile raster polygon text label text_repel label_repel rug blank`
- reference lines: `hline vline abline` (params `yintercept` / `xintercept` / `slope`)
- distributions: `histogram freqpoly density density_ridges boxplot violin dotplot
  smooth count bin2d hex density2d contour contour_filled qq qq_line qq_band
  censor_marks`
- intervals: `errorbar errorbarh linerange pointrange crossbar xrange`
- business: `waterfall pareto funnel pyramid lollipop dumbbell bullet
  bullet_vertical marimekko variwide waffle parliament tilemap packed_bubble
  variable_pie wordcloud venn timeline calendar flags windbarb`
- hierarchies & networks: `treemap sunburst icicle dendrogram tree sankey chord
  arcdiagram network stream`
- finance: `candlestick ohlc hlc heikin_ashi renko point_figure`
- spatial: `sf sf_text sf_label flow` (a WKT `geometry` column + `params.projection`)

Special aesthetics: sankey/chord `from`/`to`/`weight`; treemap `weight`
(+ `subgroup`, `label`); sunburst/icicle `params.levels`; candlestick/ohlc
`open`/`high`/`low`/`close`; intervals `ymin`/`ymax`.

```js
const svg = render({
  data: { src: ["a", "a", "b"], dst: ["b", "c", "c"], n: [3, 2, 1] },
  aes: { from: "src", to: "dst", weight: "n" },
  layers: [{ geom: "sankey" }],
});
if (!svg.includes("<svg")) throw new Error("sankey failed");
```

```js
const { svg, warnings } = renderWithWarnings({
  data: { x: [1, 2, 3, 4], y: [1, null, 3, 4], g: ["a", "a", "b", "b"] },
  aes: { x: "x", y: "y", fill: "g" },
  layers: [{ geom: "col", position: { type: "dodge", width: 0.8 } }],
  scales: { fill: { values: { a: "#e69f00", b: "#56b4e9" } } },
  coord: "flip",
});
if (!Array.isArray(warnings) || !svg) throw new Error("bad result");
```

## Workflow & pitfalls

1. Shape data column-wise (`{ col: [...] }`); convert row arrays with
   `Object.fromEntries(keys.map(k => [k, rows.map(r => r[k])]))`.
2. Use numbers for continuous and strings for categorical columns — a numeric
   `color` column gets a gradient, and a discrete palette name then errors.
3. Develop with `renderWithWarnings` and read the warnings.
4. On a thrown `Error`, read the message: it names the field
   (`layers[0].aes.x: column 'wieght' not found (available: …)`) and lists the
   valid choices.
5. For huge data, aggregate first (e.g. DuckDB-Wasm) — SVG becomes slow past
   ~50k marks.
