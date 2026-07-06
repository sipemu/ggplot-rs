// Demo: DuckDB-Wasm (spatial extension) → WKT → ggplot-rs (WASM) → SVG with hover.
//
// Build the ggplot bundle first:
//   wasm-pack build --target web --out-dir web/pkg --no-default-features --features wasm
// then serve this directory over HTTP (module workers need it), e.g.:
//   python3 -m http.server -d web 8080   # open http://localhost:8080

import * as duckdb from "https://cdn.jsdelivr.net/npm/@duckdb/duckdb-wasm@1.29.0/+esm";
import init, { render_geo } from "./pkg/ggplot_rs.js";

const status = (msg) => (document.getElementById("status").textContent = msg);

async function main() {
  status("initialising ggplot-rs (wasm)…");
  await init();

  status("starting DuckDB-Wasm…");
  const bundles = duckdb.getJsDelivrBundles();
  const bundle = await duckdb.selectBundle(bundles);
  const worker = new Worker(bundle.mainWorker);
  const db = new duckdb.AsyncDuckDB(new duckdb.ConsoleLogger(), worker);
  await db.instantiate(bundle.mainModule, bundle.pthreadWorker);
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
}

main().catch((e) => {
  console.error(e);
  status("error: " + e);
});
