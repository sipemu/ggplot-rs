# ggplot-rs in the browser (WASM) + DuckDB-Wasm spatial

A demo of plotting **spatial data in the browser**: DuckDB-Wasm (with the
`spatial` extension) reads geometry and `ST_AsText`s it to WKT; ggplot-rs —
compiled to WebAssembly — renders an interactive SVG with **hover tooltips**.

## Build & run

```sh
# 1. Build the ggplot-rs WASM bundle (needs wasm-pack + the wasm32 target)
wasm-pack build --target web --out-dir web/pkg --no-default-features --features wasm

# 2. Serve this directory over HTTP (module workers require it)
python3 -m http.server -d web 8080
# open http://localhost:8080
```

The bundle is ~310 KB of `.wasm` (no polars, no plotters — it uses the
plotters-free `SvgBackend`). `web/pkg/` is a build artifact and is git-ignored.

## How it fits together

```
DuckDB-Wasm (data + spatial)              ggplot-rs WASM (grammar + SVG)
  ST_Read(...) / ST_AsText(geom)  ──►  render_geo({ geometry, fill, label, … })
  (shapefile, GeoJSON, GeoPackage)         → <svg> with <title> hover tooltips
```

`render_geo(specJson) -> String` (see `src/wasm.rs`) takes columnar geometry +
options and returns an SVG document. Every mark is a real DOM element, so hover
works via the native `<title>` tooltip plus a little CSS.

## Scaling to many points

SVG is great up to ~10k–50k elements. For larger data, **aggregate in DuckDB**
(`GROUP BY`, hex-binning, `ST_` clustering, sampling) before rendering — DuckDB
crunches millions of rows client-side and ggplot-rs draws the compact summary.
For raw million-point scatter, a canvas/WebGL `DrawBackend` would be the next
step (the backend trait is designed for it).

## Real data

Swap the demo query's `VALUES` for a file read:

```sql
SELECT ST_AsText(geom) AS geometry, name, pop_est
FROM ST_Read('https://.../ne_110m_admin_0_countries.geojson')
```
