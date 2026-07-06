// DuckDB-Wasm (spatial) → WKT → ggplot-rs (WASM) → SVG (hover) + raster canvas.
//
// Build:  wasm-pack build --target web --out-dir web/pkg --no-default-features --features wasm,canvas
// Serve:  python3 -m http.server -d web 8080   → http://localhost:8080

import * as duckdb from "https://cdn.jsdelivr.net/npm/@duckdb/duckdb-wasm@1.29.0/+esm";
import init, { render_geo, render_scatter_xy } from "./pkg/ggplot_rs.js";

const set = (id, msg, busy = false) => {
  const el = document.getElementById(id);
  el.textContent = msg;
  el.classList.toggle("busy", busy);
};

// ── Shared floating tooltip ────────────────────────────────────────────────
const tip = document.getElementById("tip");
const showTip = (text, x, y) => {
  tip.textContent = text;
  tip.style.left = `${x}px`;
  tip.style.top = `${y}px`;
  tip.classList.add("on");
};
const hideTip = () => tip.classList.remove("on");

async function main() {
  set("status", "initialising ggplot-rs (wasm)…", true);
  await init();

  set("status", "starting DuckDB-Wasm…", true);
  const bundle = await duckdb.selectBundle(duckdb.getJsDelivrBundles());
  // Cross-origin `new Worker(cdnUrl)` is blocked; wrap it in a same-origin Blob
  // that importScripts the CDN worker (allowed cross-origin).
  const workerUrl = URL.createObjectURL(
    new Blob([`importScripts("${bundle.mainWorker}");`], { type: "text/javascript" }),
  );
  const worker = new Worker(workerUrl);
  const db = new duckdb.AsyncDuckDB(new duckdb.ConsoleLogger(), worker);
  await db.instantiate(bundle.mainModule, bundle.pthreadWorker);
  URL.revokeObjectURL(workerUrl);
  const conn = await db.connect();

  set("status", "loading the spatial extension…", true);
  await conn.query("INSTALL spatial; LOAD spatial;");

  const url =
    "https://cdn.jsdelivr.net/gh/nvkelso/natural-earth-vector@master/geojson/ne_110m_admin_0_countries.geojson";
  await db.registerFileURL("countries.geojson", url, duckdb.DuckDBDataProtocol.HTTP, false);

  set("status", "querying Natural Earth countries…", true);
  const sql = `
    SELECT ST_AsText(geom) AS geometry, NAME AS name, ln(POP_EST + 1) AS pop_log
    FROM ST_Read('countries.geojson')
    WHERE NAME <> 'Antarctica'`;
  const rows = (await conn.query(sql)).toArray().map((r) => r.toJSON());

  const spec = {
    geometry: rows.map((r) => r.geometry),
    fill: rows.map((r) => Number(r.pop_log)),
    label: rows.map((r) => r.name),
    fill_name: "ln(pop)",
    width: 960,
    height: 520,
  };
  document.getElementById("plot").innerHTML = render_geo(JSON.stringify(spec));
  wireSvgHover();
  set("status", `${rows.length} countries. Hover one for its name + ln(population).`);

  scatterDemo();
}

// Turn each feature's <title> into a styled tooltip (and drop the native one).
function wireSvgHover() {
  const plot = document.getElementById("plot");
  plot.querySelectorAll("title").forEach((t) => {
    t.parentNode.setAttribute("data-tip", t.textContent);
    t.remove();
  });
  plot.addEventListener("mousemove", (e) => {
    const el = e.target.closest("[data-tip]");
    if (el) showTip(el.getAttribute("data-tip"), e.clientX, e.clientY);
    else hideTip();
  });
  plot.addEventListener("mouseleave", hideTip);
}

// ── Large-N scatter (raster backend) with nearest-point hover ──────────────
function scatterDemo() {
  const n = 100_000;
  const x = new Float64Array(n), y = new Float64Array(n), group = new Array(n);
  const cx = [-2, 0, 2.5], cy = [0, 2, -1], names = ["a", "b", "c"];
  set("status2", `generating ${n.toLocaleString()} points…`, true);
  for (let i = 0; i < n; i++) {
    const k = i % 3;
    const r = Math.sqrt(-2 * Math.log(Math.random() + 1e-9));
    const t = 2 * Math.PI * Math.random();
    x[i] = cx[k] + r * Math.cos(t);
    y[i] = cy[k] + r * Math.sin(t);
    group[i] = names[k];
  }

  const canvas = document.getElementById("scatter");
  const t0 = performance.now();
  const res = render_scatter_xy(x, y, group, canvas.width, canvas.height,
    `${n.toLocaleString()} points · raster`);
  const ms = Math.round(performance.now() - t0);

  const img = new ImageData(new Uint8ClampedArray(res.rgba), canvas.width, canvas.height);
  canvas.getContext("2d").putImageData(img, 0, 0);
  set("status2", `rendered ${n.toLocaleString()} points in ${ms} ms — hover for the nearest point.`);

  // Invert canvas pixels → data using the mapping returned by the renderer.
  const [px, py, pw, ph] = res.plot;
  const [xe0, xe1] = res.xdom, [ye0, ye1] = res.ydom;
  const sx = pw / (xe1 - xe0), sy = ph / (ye1 - ye0); // data→pixel scale
  canvas.addEventListener("mousemove", (e) => {
    const rect = canvas.getBoundingClientRect();
    const mx = (e.clientX - rect.left) * (canvas.width / rect.width);
    const my = (e.clientY - rect.top) * (canvas.height / rect.height);
    if (mx < px || mx > px + pw || my < py || my > py + ph) return hideTip();
    const dx = xe0 + ((mx - px) / pw) * (xe1 - xe0);
    const dy = ye0 + (1 - (my - py) / ph) * (ye1 - ye0);
    let best = -1, bestD = Infinity;
    for (let i = 0; i < n; i++) {
      const ex = (x[i] - dx) * sx, ey = (y[i] - dy) * sy;
      const d = ex * ex + ey * ey;
      if (d < bestD) { bestD = d; best = i; }
    }
    if (best >= 0 && bestD < 18 * 18) {
      showTip(`group ${group[best]} · (${x[best].toFixed(2)}, ${y[best].toFixed(2)})`, e.clientX, e.clientY);
    } else hideTip();
  });
  canvas.addEventListener("mouseleave", hideTip);
}

main().catch((e) => {
  console.error(e);
  set("status", "error: " + e);
});
