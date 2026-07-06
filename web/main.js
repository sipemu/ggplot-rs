// Demo: DuckDB-Wasm (spatial extension) → WKT → ggplot-rs (WASM) → SVG with hover.
//
// Build the ggplot bundle first:
//   wasm-pack build --target web --out-dir web/pkg --no-default-features --features wasm
// then serve this directory over HTTP (module workers need it), e.g.:
//   python3 -m http.server -d web 8080   # open http://localhost:8080

import * as duckdb from "https://cdn.jsdelivr.net/npm/@duckdb/duckdb-wasm@1.29.0/+esm";
import init, { render_geo, render_scatter_rgba } from "./pkg/ggplot_rs.js";

const status = (msg) => (document.getElementById("status").textContent = msg);
const status2 = (msg) => (document.getElementById("status2").textContent = msg);

async function main() {
  status("initialising ggplot-rs (wasm)…");
  await init();

  status("starting DuckDB-Wasm…");
  const bundles = duckdb.getJsDelivrBundles();
  const bundle = await duckdb.selectBundle(bundles);
  // A cross-origin `new Worker(cdnUrl)` is blocked; wrap the CDN worker in a
  // same-origin Blob that `importScripts` it (which *is* allowed cross-origin).
  const workerUrl = URL.createObjectURL(
    new Blob([`importScripts("${bundle.mainWorker}");`], { type: "text/javascript" }),
  );
  const worker = new Worker(workerUrl);
  const db = new duckdb.AsyncDuckDB(new duckdb.ConsoleLogger(), worker);
  await db.instantiate(bundle.mainModule, bundle.pthreadWorker);
  URL.revokeObjectURL(workerUrl);
  const conn = await db.connect();

  status("loading the spatial extension…");
  await conn.query("INSTALL spatial; LOAD spatial;");

  // Real data: Natural Earth 110m countries (~820 KB GeoJSON), read straight
  // from the CDN by DuckDB-Wasm. Swap the URL for a shapefile / GeoPackage /
  // your own export — ST_Read handles them all. For huge datasets, aggregate
  // here (GROUP BY / hexbin / sample) so the SVG stays light.
  const url =
    "https://cdn.jsdelivr.net/gh/nvkelso/natural-earth-vector@master/geojson/ne_110m_admin_0_countries.geojson";
  await db.registerFileURL("countries.geojson", url, duckdb.DuckDBDataProtocol.HTTP, false);

  status("querying geometry (Natural Earth countries)…");
  const sql = `
    SELECT ST_AsText(geom) AS geometry,
           NAME             AS name,
           ln(POP_EST + 1)  AS pop_log
    FROM ST_Read('countries.geojson')
    WHERE NAME <> 'Antarctica'`;
  const rows = (await conn.query(sql)).toArray().map((r) => r.toJSON());

  // Reshape rows → columnar spec for the WASM renderer.
  const spec = {
    geometry: rows.map((r) => r.geometry),
    fill: rows.map((r) => Number(r.pop_log)),
    label: rows.map((r) => r.name),
    title: "World population (log) — Natural Earth via DuckDB spatial",
    width: 1000,
    height: 560,
    // projection: "mercator",   // web-map look (clamps the poles)
  };

  status(`rendering ${rows.length} countries…`);
  document.getElementById("plot").innerHTML = render_geo(JSON.stringify(spec));
  status(`done — ${rows.length} countries. Hover one for its name + value.`);

  await scatterDemo();
}

// Large-N: 100k points → raster backend → putImageData onto a <canvas>.
async function scatterDemo() {
  const n = 100_000;
  const x = new Array(n), y = new Array(n), color = new Array(n);
  const cx = [-2, 0, 2.5], cy = [0, 2, -1], names = ["a", "b", "c"];
  status2(`generating ${n.toLocaleString()} points…`);
  for (let i = 0; i < n; i++) {
    const k = i % 3;
    // Box–Muller gaussian
    const r = Math.sqrt(-2 * Math.log(Math.random() + 1e-9));
    const t = 2 * Math.PI * Math.random();
    x[i] = cx[k] + r * Math.cos(t);
    y[i] = cy[k] + r * Math.sin(t);
    color[i] = names[k];
  }
  const canvas = document.getElementById("scatter");
  const spec = { x, y, color, width: canvas.width, height: canvas.height,
                 title: `${n.toLocaleString()} points · raster` };

  const t0 = performance.now();
  const rgba = render_scatter_rgba(JSON.stringify(spec));
  const ms = Math.round(performance.now() - t0);

  const img = new ImageData(new Uint8ClampedArray(rgba), canvas.width, canvas.height);
  canvas.getContext("2d").putImageData(img, 0, 0);
  status2(`rendered ${n.toLocaleString()} points in ${ms} ms.`);
}

main().catch((e) => {
  console.error(e);
  status("error: " + e);
});
