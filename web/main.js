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

  // Self-contained demo data. In a real app this is a file read:
  //   FROM ST_Read('countries.shp')            -- shapefile / GeoJSON / GeoPackage
  //   FROM ST_Read('https://…/countries.geojson')
  // For huge datasets, aggregate here (GROUP BY / hexbin / sample) so the SVG
  // stays light — DuckDB crunches millions of rows, ggplot-rs draws the summary.
  status("querying geometry…");
  const sql = `
    SELECT ST_AsText(geom) AS geometry, name, pop
    FROM (VALUES
      (ST_GeomFromText('POLYGON ((0 0, 3 0, 3 2, 1 2.5, 0 2, 0 0))'), 'North',  4.2),
      (ST_GeomFromText('POLYGON ((3 0, 6 0, 6 3, 3 2, 3 0))'),        'East',   7.8),
      (ST_GeomFromText('POLYGON ((0 2, 1 2.5, 3 2, 3 5, 0 5, 0 2))'), 'West',   3.1),
      (ST_GeomFromText('POLYGON ((3 2, 6 3, 6 5, 3 5, 3 2))'),        'Center', 9.5),
      (ST_GeomFromText('POLYGON ((6 0, 9 1, 8 4, 6 3, 6 0))'),        'Coast',  5.4),
      (ST_GeomFromText('POLYGON ((6 3, 8 4, 9 6, 6 5, 6 3))'),        'Cape',   2.7)
    ) t(geom, name, pop)`;
  const rows = (await conn.query(sql)).toArray().map((r) => r.toJSON());

  // Reshape rows → columnar spec for the WASM renderer.
  const spec = {
    geometry: rows.map((r) => r.geometry),
    fill: rows.map((r) => Number(r.pop)),
    label: rows.map((r) => r.name),
    title: "Population by province (DuckDB spatial → geom_sf)",
    width: 640,
    height: 480,
    // projection: "mercator",   // for lon/lat data
  };

  status("rendering…");
  document.getElementById("plot").innerHTML = render_geo(JSON.stringify(spec));
  status("done — hover a province.");
}

main().catch((e) => {
  console.error(e);
  status("error: " + e);
});
