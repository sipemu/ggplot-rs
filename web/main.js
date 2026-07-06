// DuckDB-Wasm (spatial) → WKT → ggplot-rs (WASM). Two interactive dashboards.
// The scatter needs no DuckDB, so it runs first and proves the wasm renderer;
// the map (which needs DuckDB) runs independently, so a failure in one is
// isolated and reported in place.
//
// Build:  wasm-pack build --target web --out-dir web/pkg --no-default-features --features wasm,canvas

import * as duckdb from "https://cdn.jsdelivr.net/npm/@duckdb/duckdb-wasm@1.29.0/+esm";
import init, { render_geo, render_bar, render_scatter_xy } from "./pkg/ggplot_rs.js";

const set = (id, msg, busy = false) => {
  const el = document.getElementById(id);
  if (!el) return;
  el.textContent = msg;
  el.classList.toggle("busy", busy);
};

const tip = document.getElementById("tip");
const showTip = (text, x, y) => {
  tip.textContent = text;
  tip.style.left = `${x}px`;
  tip.style.top = `${y}px`;
  tip.classList.add("on");
};
const hideTip = () => tip.classList.remove("on");

const detitle = (el) => {
  el.querySelectorAll("title").forEach((t) => {
    t.parentNode.setAttribute("data-tip", t.textContent);
    t.remove();
  });
  return el;
};
const hoverTips = (el) => {
  el.addEventListener("mousemove", (e) => {
    const m = e.target.closest("[data-tip]");
    if (m) showTip(m.getAttribute("data-tip"), e.clientX, e.clientY);
    else hideTip();
  });
  el.addEventListener("mouseleave", hideTip);
};

async function main() {
  set("status", "initialising ggplot-rs (wasm)…", true);
  set("status2", "initialising…", true);
  await init();

  // 1. Scatter first — pure ggplot-rs, no data source.
  try {
    scatterDemo();
  } catch (e) {
    console.error("scatter:", e);
    set("status2", "scatter error: " + (e.message || e));
  }

  // 2. Map — needs DuckDB-Wasm + the spatial extension.
  try {
    await mapDemo();
  } catch (e) {
    console.error("map:", e);
    set("status", "map error: " + (e.message || e));
  }
}

// ── Map: DuckDB spatial → choropleth, click a country to drill into continent ─
let allRows = [];
const nameToContinent = {};

async function mapDemo() {
  set("status", "starting DuckDB-Wasm…", true);
  const bundle = await duckdb.selectBundle(duckdb.getJsDelivrBundles());
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

  // Fetch the GeoJSON bytes and hand them to DuckDB (more robust for ST_Read
  // than a lazy HTTP file).
  set("status", "downloading Natural Earth countries…", true);
  const url =
    "https://cdn.jsdelivr.net/gh/nvkelso/natural-earth-vector@master/geojson/ne_110m_admin_0_countries.geojson";
  const bytes = new Uint8Array(await (await fetch(url)).arrayBuffer());
  await db.registerFileBuffer("countries.geojson", bytes);

  set("status", "reading geometry…", true);
  const sql = `
    SELECT ST_AsText(geom) AS geometry, NAME AS name, CONTINENT AS continent,
           ln(POP_EST + 1) AS pop_log
    FROM ST_Read('countries.geojson')
    WHERE NAME <> 'Antarctica'`;
  allRows = (await conn.query(sql)).toArray().map((r) => r.toJSON());
  for (const r of allRows) nameToContinent[r.name] = r.continent;

  renderMap(allRows, "World — hover a country, or click to zoom to its continent");
  set("status", `${allRows.length} countries loaded.`);

  const plot = document.getElementById("plot");
  hoverTips(plot);
  plot.addEventListener("click", (e) => {
    const m = e.target.closest("[data-tip]");
    if (!m) return;
    const name = m.getAttribute("data-tip").replace(/: [^:]*$/, "");
    const cont = nameToContinent[name];
    if (!cont) return;
    renderMap(allRows.filter((r) => r.continent === cont), `${cont} — click ⟳ World to reset`);
    document.getElementById("reset").style.display = "";
  });
  document.getElementById("reset").onclick = () => {
    renderMap(allRows, "World — hover a country, or click to zoom to its continent");
    document.getElementById("reset").style.display = "none";
  };
}

function renderMap(rows, title) {
  const svg = render_geo(JSON.stringify({
    geometry: rows.map((r) => r.geometry),
    fill: rows.map((r) => Number(r.pop_log)),
    label: rows.map((r) => r.name),
    width: 960, height: 520, title,
  }));
  const plot = document.getElementById("plot");
  plot.innerHTML = svg;
  detitle(plot);
}

// ── Linked views: raster scatter (brush) → ggplot-rs bar (counts) ──────────
function scatterDemo() {
  const n = 100_000;
  const x = new Float64Array(n), y = new Float64Array(n), gidx = new Uint32Array(n);
  const cx = [-2, 0, 2.5], cy = [0, 2, -1], names = ["a", "b", "c"];
  set("status2", `generating ${n.toLocaleString()} points…`, true);
  for (let i = 0; i < n; i++) {
    const k = i % 3;
    const r = Math.sqrt(-2 * Math.log(Math.random() + 1e-9));
    const t = 2 * Math.PI * Math.random();
    x[i] = cx[k] + r * Math.cos(t);
    y[i] = cy[k] + r * Math.sin(t);
    gidx[i] = k;
  }

  const canvas = document.getElementById("scatter");
  const t0 = performance.now();
  const res = render_scatter_xy(x, y, gidx, names, canvas.width, canvas.height, `${n.toLocaleString()} points`);
  const ms = Math.round(performance.now() - t0);
  canvas.getContext("2d").putImageData(
    new ImageData(new Uint8ClampedArray(res.rgba), canvas.width, canvas.height), 0, 0);
  set("status2", `rendered ${n.toLocaleString()} points in ${ms} ms — hover, or drag to brush.`);

  const plot = res.plot, xdom = res.xdom, ydom = res.ydom;
  const px = plot[0], py = plot[1], pw = plot[2], ph = plot[3];
  const xe0 = xdom[0], xe1 = xdom[1], ye0 = ydom[0], ye1 = ydom[1];
  const sx = pw / (xe1 - xe0), sy = ph / (ye1 - ye0);

  const counts = (pred) => {
    const c = { a: 0, b: 0, c: 0 };
    for (let i = 0; i < n; i++) if (pred(i)) c[names[gidx[i]]]++;
    return c;
  };
  const renderBar = (c, title) => {
    const el = document.getElementById("scatterbar");
    el.innerHTML = render_bar(JSON.stringify({
      category: names, value: names.map((g) => c[g]), width: 300, height: 300, title,
    }));
    detitle(el);
  };
  renderBar(counts(() => true), `all ${n.toLocaleString()} points`);
  hoverTips(document.getElementById("scatterbar"));

  const px2 = (e) => {
    const r = canvas.getBoundingClientRect();
    return {
      cx: (e.clientX - r.left) * (canvas.width / r.width),
      cy: (e.clientY - r.top) * (canvas.height / r.height),
      ox: e.clientX - r.left, oy: e.clientY - r.top,
    };
  };

  let brushing = null;
  const brush = document.getElementById("brush");

  canvas.addEventListener("mousemove", (e) => {
    if (brushing) return;
    const p = px2(e);
    if (p.cx < px || p.cx > px + pw || p.cy < py || p.cy > py + ph) return hideTip();
    const dx = xe0 + ((p.cx - px) / pw) * (xe1 - xe0);
    const dy = ye0 + (1 - (p.cy - py) / ph) * (ye1 - ye0);
    let best = -1, bestD = Infinity;
    for (let i = 0; i < n; i++) {
      const ex = (x[i] - dx) * sx, ey = (y[i] - dy) * sy, d = ex * ex + ey * ey;
      if (d < bestD) { bestD = d; best = i; }
    }
    if (best >= 0 && bestD < 18 * 18)
      showTip(`group ${names[gidx[best]]} · (${x[best].toFixed(2)}, ${y[best].toFixed(2)})`, e.clientX, e.clientY);
    else hideTip();
  });
  canvas.addEventListener("mouseleave", () => { if (!brushing) hideTip(); });

  canvas.addEventListener("mousedown", (e) => {
    const p = px2(e);
    brushing = p;
    hideTip();
    Object.assign(brush.style, { display: "block", left: `${p.ox}px`, top: `${p.oy}px`, width: "0px", height: "0px" });
  });
  window.addEventListener("mousemove", (e) => {
    if (!brushing) return;
    const p = px2(e);
    Object.assign(brush.style, {
      left: `${Math.min(brushing.ox, p.ox)}px`, top: `${Math.min(brushing.oy, p.oy)}px`,
      width: `${Math.abs(p.ox - brushing.ox)}px`, height: `${Math.abs(p.oy - brushing.oy)}px`,
    });
  });
  window.addEventListener("mouseup", (e) => {
    if (!brushing) return;
    const p = px2(e), s = brushing;
    brushing = null;
    brush.style.display = "none";
    if (Math.abs(p.cx - s.cx) < 5 || Math.abs(p.cy - s.cy) < 5)
      return renderBar(counts(() => true), `all ${n.toLocaleString()} points`);
    const toData = (mx, my) => [xe0 + ((mx - px) / pw) * (xe1 - xe0), ye0 + (1 - (my - py) / ph) * (ye1 - ye0)];
    const [ax, ay] = toData(Math.min(s.cx, p.cx), Math.max(s.cy, p.cy));
    const [bx, by] = toData(Math.max(s.cx, p.cx), Math.min(s.cy, p.cy));
    const c = counts((i) => x[i] >= ax && x[i] <= bx && y[i] >= ay && y[i] <= by);
    renderBar(c, `${(c.a + c.b + c.c).toLocaleString()} selected`);
  });
}

main().catch((e) => {
  console.error(e);
  set("status", "fatal: " + (e.message || e));
});
