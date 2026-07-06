// DuckDB-Wasm (spatial) → WKT → ggplot-rs (WASM). Two interactive dashboards:
//   1. World choropleth — click a country to re-query + zoom to its continent.
//   2. Linked views — brush the raster scatter, a ggplot-rs bar chart reacts.
//
// Build:  wasm-pack build --target web --out-dir web/pkg --no-default-features --features wasm,canvas

import * as duckdb from "https://cdn.jsdelivr.net/npm/@duckdb/duckdb-wasm@1.29.0/+esm";
import init, { render_geo, render_bar, render_scatter_xy } from "./pkg/ggplot_rs.js";

const set = (id, msg, busy = false) => {
  const el = document.getElementById(id);
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

// Replace each SVG <title> with a data-tip attribute + return the container.
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

let allRows = [];
const nameToContinent = {};

async function main() {
  set("status", "initialising ggplot-rs (wasm)…", true);
  await init();

  set("status", "starting DuckDB-Wasm…", true);
  const bundle = await duckdb.selectBundle(duckdb.getJsDelivrBundles());
  // Cross-origin `new Worker(cdnUrl)` is blocked; wrap it in a same-origin Blob.
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
    SELECT ST_AsText(geom) AS geometry, NAME AS name, CONTINENT AS continent,
           ln(POP_EST + 1) AS pop_log
    FROM ST_Read('countries.geojson')
    WHERE NAME <> 'Antarctica'`;
  allRows = (await conn.query(sql)).toArray().map((r) => r.toJSON());
  for (const r of allRows) nameToContinent[r.name] = r.continent;

  renderMap(allRows, "World — hover a country, or click to zoom to its continent");
  set("status", `${allRows.length} countries loaded.`);

  // Delegated interactions on the persistent #plot container.
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

  scatterDemo();
}

function renderMap(rows, title) {
  const spec = {
    geometry: rows.map((r) => r.geometry),
    fill: rows.map((r) => Number(r.pop_log)),
    label: rows.map((r) => r.name),
    width: 960,
    height: 520,
    title,
  };
  detitle(Object.assign(document.getElementById("plot"), { innerHTML: render_geo(JSON.stringify(spec)) }));
}

// ── Linked views: raster scatter (brush) → ggplot-rs bar (counts) ──────────
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
  const res = render_scatter_xy(x, y, group, canvas.width, canvas.height, `${n.toLocaleString()} points`);
  const ms = Math.round(performance.now() - t0);
  canvas.getContext("2d").putImageData(
    new ImageData(new Uint8ClampedArray(res.rgba), canvas.width, canvas.height), 0, 0);
  set("status2", `rendered ${n.toLocaleString()} points in ${ms} ms — hover, or drag to brush.`);

  const [px, py, pw, ph] = res.plot, [xe0, xe1] = res.xdom, [ye0, ye1] = res.ydom;
  const sx = pw / (xe1 - xe0), sy = ph / (ye1 - ye0);

  const counts = (pred) => {
    const c = { a: 0, b: 0, c: 0 };
    for (let i = 0; i < n; i++) if (pred(i)) c[group[i]]++;
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

  // Canvas-internal + CSS pixel coords for an event.
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

  // Hover: nearest point (suppressed while brushing).
  canvas.addEventListener("mousemove", (e) => {
    if (brushing) return;
    const { cx: mx, cy: my } = px2(e);
    if (mx < px || mx > px + pw || my < py || my > py + ph) return hideTip();
    const dx = xe0 + ((mx - px) / pw) * (xe1 - xe0);
    const dy = ye0 + (1 - (my - py) / ph) * (ye1 - ye0);
    let best = -1, bestD = Infinity;
    for (let i = 0; i < n; i++) {
      const ex = (x[i] - dx) * sx, ey = (y[i] - dy) * sy, d = ex * ex + ey * ey;
      if (d < bestD) { bestD = d; best = i; }
    }
    if (best >= 0 && bestD < 18 * 18)
      showTip(`group ${group[best]} · (${x[best].toFixed(2)}, ${y[best].toFixed(2)})`, e.clientX, e.clientY);
    else hideTip();
  });
  canvas.addEventListener("mouseleave", () => !brushing && hideTip());

  // Brush → per-group counts in the bar.
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
    if (Math.abs(p.cx - s.cx) < 5 || Math.abs(p.cy - s.cy) < 5) {
      return renderBar(counts(() => true), `all ${n.toLocaleString()} points`);
    }
    const toData = (mx, my) => [xe0 + ((mx - px) / pw) * (xe1 - xe0), ye0 + (1 - (my - py) / ph) * (ye1 - ye0)];
    const [ax, ay] = toData(Math.min(s.cx, p.cx), Math.max(s.cy, p.cy));
    const [bx, by] = toData(Math.max(s.cx, p.cx), Math.min(s.cy, p.cy));
    const c = counts((i) => x[i] >= ax && x[i] <= bx && y[i] >= ay && y[i] <= by);
    renderBar(c, `${(c.a + c.b + c.c).toLocaleString()} selected`);
  });
}

main().catch((e) => {
  console.error(e);
  set("status", "error: " + e);
});
