# @sipemu/ggplot-rs

ggplot2-style **grammar-of-graphics charts rendered to SVG**, in the browser or
in Node. This is the WebAssembly build of the [ggplot-rs](https://github.com/sipemu/ggplot-rs)
Rust crate. It has no runtime dependencies and no DOM requirement: you pass in
a spec and get back an SVG string.

- **Live demo:** https://sipemu.github.io/ggplot-rs/
- **Gallery & Rust docs:** https://github.com/sipemu/ggplot-rs · https://docs.rs/ggplot-rs

```sh
npm i @sipemu/ggplot-rs
```

## Quickstart

### Browser (vanilla ESM)

```html
<div id="chart"></div>
<script type="module">
  import init, { render } from 'https://cdn.jsdelivr.net/npm/@sipemu/ggplot-rs/dist/ggplot_rs.js';

  await init(); // fetches ggplot_rs_bg.wasm next to the module
  document.getElementById('chart').innerHTML = render({
    data: { wt: [2.6, 2.9, 3.2, 3.4, 3.6], mpg: [21, 22.8, 21.4, 18.7, 18.1] },
    aes: { x: 'wt', y: 'mpg' },
    layers: [{ geom: 'point' }, { geom: 'smooth', params: { method: 'lm' } }],
    labels: { title: 'Fuel economy', x: 'Weight', y: 'MPG' },
  });
</script>
```

### Vite / other bundlers

```js
import init, { render } from '@sipemu/ggplot-rs';

init().then(() => {
  document.querySelector('#chart').innerHTML = render(spec);
});
```

Vite resolves the `.wasm` file on its own. If you want top-level
`await init()` instead, set `build.target: 'es2022'` in your Vite config.

### Node (≥ 18)

```js
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import init, { render } from '@sipemu/ggplot-rs';

const require = createRequire(import.meta.url);
await init({ module_or_path: await readFile(require.resolve('@sipemu/ggplot-rs/ggplot_rs_bg.wasm')) });

const svg = render(spec); // write it to a file, serve it, embed it …
```

You can also call `initSync({ module: bytes })` to initialise synchronously.
In Observable, `await init()` works the same as in the browser.

## The spec

`render()` takes a typed `PlotSpec` object or its JSON text. The fields map
onto the ggplot2 builder:

```ts
import type { PlotSpec } from '@sipemu/ggplot-rs';

const spec: PlotSpec = {
  data: {                                   // column-oriented; equal lengths
    dose: [0.5, 1, 2, 0.5, 1, 2],
    len: [13.2, 22.7, 26.1, 7.98, 16.8, 26.1],
    supp: ['OJ', 'OJ', 'OJ', 'VC', 'VC', 'VC'],
  },
  aes: { x: 'dose', y: 'len', color: 'supp' }, // inherited by every layer
  layers: [
    { geom: 'point' },
    { geom: 'line' },
    { geom: 'hline', params: { yintercept: 20 } },
  ],
  scales: {
    x: { trans: 'log2', name: 'Dose (mg)' },
    color: { palette: 'Dark2' },            // or { values: { OJ: '#e69f00', VC: '#56b4e9' } }
  },
  facet: { wrap: 'supp' },                  // or { rows: 'a', cols: 'b', scales: 'free_y' }
  coord: 'cartesian',                       // 'flip' | 'polar' | 'fixed' | 'radar' | 'sf' | { type, xlim, ylim, ratio }
  labels: { title: 'Tooth growth', subtitle: 'by supplement', y: 'Length' },
  theme: { base: 'bw', title: { size: 16, face: 'bold' } }, // or just 'classic'
  legend: 'bottom',                         // 'none' | 'inside' | [x, y] …
  width: 640,
  height: 400,
};
```

| Field | What it does |
|---|---|
| `layers[].geom` | One of `geoms()` — every `geom_*` of the Rust crate (a test enforces this): the basic geoms (point, line, path, step, area, ribbon, col, bar, segment, curve, rect, tile, raster, polygon, text, label, text_repel, …), reference lines (hline, vline, abline), distributions (histogram, freqpoly, density, density_ridges, boxplot, violin, dotplot, smooth, count, bin2d, hex, density2d, contour, contour_filled, qq, qq_line, qq_band), intervals (errorbar, errorbarh, linerange, pointrange, crossbar, xrange), business charts (waterfall, pareto, funnel, pyramid, lollipop, dumbbell, bullet, marimekko, waffle, parliament, tilemap, wordcloud, venn, timeline, calendar, …), hierarchies and networks (treemap, sunburst, icicle, dendrogram, sankey, chord, arcdiagram, network, stream), finance (candlestick, ohlc, hlc, heikin_ashi, renko, point_figure) and spatial (sf, sf_text, sf_label, flow). |
| `layers[].aes` | A layer-level mapping, merged over the plot `aes`. |
| `layers[].data` | Layer-specific data. |
| `layers[].params` | `bins` (histogram), `method`/`span`/`se` (smooth), `yintercept`/`xintercept`/`slope`/`intercept` (reference lines), `levels` (sunburst/icicle), `rows` (waffle/parliament), `box_size`/`reversal` (renko/point_figure), `shape` (tilemap), `speed`/`direction` (windbarb), `censor` (censor_marks), `projection` (sf, sf_text, sf_label, flow), `target`/`ranges` (bullet). |
| `layers[].position` | `identity`, `stack`, `fill`, `dodge`, `dodge2`, `jitter` or `nudge`. Use an object such as `{ type: 'dodge', width: 0.8 }` to set parameters. |
| `layers[].stat` | `'identity'`, which plots values as given (e.g. `bar` with precomputed heights). |
| `theme` | A preset name (`gray`, `bw`, `minimal`, `classic`, `dark`, `light`, `void`, `linedraw`) or a partial overlay. The overlay uses the same schema as the ggplot-rs CLI's `--theme-config` (`base`, `primary`, `palette`, per-element `title`/`axis_text_x`/`panel_grid_major` …). The default is `minimal`. |

Some chart types need specific aesthetics: sankey and chord take `from`, `to`
and `weight`; treemap takes `weight`, plus optional `subgroup` and `label`;
candlestick and ohlc take `open`, `high`, `low` and `close`.

### Errors and warnings

Bad specs throw an `Error` whose message names the field at fault:

```
layers[0].aes.x: column 'wieght' not found (available: mpg, wt)
layers[1]: unknown geom 'pointy' (expected one of: point, jitter, …)
invalid plot spec: unknown field `titel`, expected one of `data`, `aes`, …
```

`renderWithWarnings(spec)` returns `{ svg, warnings }`. The warnings report
things like rows dropped for missing values, or a layer whose stat produced
no data.

### Other exports

`geoms()` lists the geom names and `version()` returns the ggplot-rs version
the package was built from. The lower-level functions used by the live demo
(`render_plot`, `render_geo`, `geo_bounds`, `render_bar`, `render_hist` and
`scatter_frame`) are exported too.

### Same spec as the CLI and Rust

The spec is the crate's `ggplot_rs::spec` module, so the `ggplot-rs` CLI
accepts the same JSON (`ggplot-rs --spec chart.json --parquet data.parquet`)
and Rust code can call `ggplot_rs::spec::render_json` (feature `spec`).

### Agent skill

The package ships an agent skill at `skills/ggplot-rs-js/SKILL.md`, a
how-to for LLM coding agents (Claude Code and others). To use it with Claude
Code, copy or link it into your project:

```sh
mkdir -p .claude/skills && cp -r node_modules/@sipemu/ggplot-rs/skills/ggplot-rs-js .claude/skills/
```

## Fidelity to ggplot2

The statistical layers (binning, density/`bw.nrd0`, loess, boxplot
quantiles, QQ, ECDF, stacking, extended-Wilkinson axis breaks) are validated
against R's `ggplot_build()` on identical inputs. The defaults follow ggplot2
4.x, so the axes, bins and smoothers should look like what you get in R. One
visible difference is that the default theme here is `minimal`, not ggplot2's
`gray`; pass `theme: 'gray'` to get the R default.

## Size

`ggplot_rs_bg.wasm` is about **1.6 MB raw, 0.59 MB gzipped** (`wasm-opt -O`).
No fonts are bundled: text is emitted as SVG `<text>` and drawn by the
browser. The package ships the SVG renderer only. The demo's raster/canvas path for
millions of points is a separate build of the same crate (`canvas` feature).

## Versioning

The npm version always matches the `ggplot-rs` crate version. Both are
published from the same GitHub release.

## License

MIT OR Apache-2.0, at your option.
