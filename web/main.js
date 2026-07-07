// DuckDB-Wasm (spatial) → WKT → ggplot-rs (WASM). Each panel is fail-isolated
// (the scatter needs no DuckDB and runs first), so one failure is reported in
// place rather than blanking the page.
//
// Build:  wasm-pack build --target web --out-dir web/pkg --no-default-features --features wasm,canvas

import * as duckdb from "https://cdn.jsdelivr.net/npm/@duckdb/duckdb-wasm@1.29.0/+esm";
import { Grid } from "https://cdn.jsdelivr.net/npm/gridjs/+esm";
import init, { render_geo, render_bar, render_hist, render_plot, render_scatter_xy, geo_bounds } from "./pkg/ggplot_rs.js";

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

// Toolbox: download a chart (M4). Works for SVG (serialize / rasterise) and the
// raster canvas. `addSaveTools` adds a hover-revealed SVG/PNG toolbar to `el`.
const download = (blob, name) => {
  const a = document.createElement("a");
  a.href = URL.createObjectURL(blob);
  a.download = name;
  document.body.appendChild(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(a.href);
};
const svgToPng = (svg, scale, cb) => {
  const xml = new XMLSerializer().serializeToString(svg);
  const img = new Image();
  img.onload = () => {
    const w = +svg.getAttribute("width") || svg.clientWidth;
    const h = +svg.getAttribute("height") || svg.clientHeight;
    const c = document.createElement("canvas");
    c.width = w * scale; c.height = h * scale;
    const ctx = c.getContext("2d");
    ctx.fillStyle = "#fff"; ctx.fillRect(0, 0, c.width, c.height);
    ctx.drawImage(img, 0, 0, c.width, c.height);
    c.toBlob(cb, "image/png");
  };
  img.src = "data:image/svg+xml;charset=utf-8," + encodeURIComponent(xml);
};
function addSaveTools(el, name) {
  el.querySelector(":scope > .savebar")?.remove(); // avoid dupes on re-render
  const bar = document.createElement("div");
  bar.className = "savebar";
  const mk = (label, fn) => {
    const btn = document.createElement("button");
    btn.type = "button"; btn.textContent = label; btn.title = `Download ${label}`;
    btn.onclick = fn; bar.appendChild(btn);
  };
  mk("SVG", () => {
    const s = el.querySelector("svg");
    if (s) download(new Blob([new XMLSerializer().serializeToString(s)], { type: "image/svg+xml;charset=utf-8" }), name + ".svg");
  });
  mk("PNG", () => {
    const canvas = el.querySelector("canvas");
    if (canvas) return canvas.toBlob((b) => download(b, name + ".png"));
    const s = el.querySelector("svg");
    if (s) svgToPng(s, 2, (b) => download(b, name + ".png"));
  });
  el.appendChild(bar);
}

// Pan/zoom (roam) for an SVG map: scroll to zoom around the cursor, drag to
// pan, double-click to reset. `getSpec()` returns the current base spec (no
// xlim/ylim) — a getter so it tracks state changes (e.g. continent drill-down);
// `rerender(spec)` redraws `el`. We feed xlim/ylim windows that render_geo clips
// to (axes update). Returns { reset }. After a real pan we set `el._panned` so a
// coexisting click handler (drill-down) can ignore the click that ends the drag.
function enableRoam(el, getSpec, rerender) {
  let view = null; // {x0,y0,x1,y1}; null = auto-fit (initial equal-aspect view)
  let raf = 0;
  const fit = () => {
    const s = getSpec();
    const aspect = s.width / s.height;
    let [x0, y0, x1, y1] = geo_bounds(JSON.stringify(s));
    const w = x1 - x0, h = y1 - y0;
    if (w / h < aspect) { const nw = h * aspect, c = (x0 + x1) / 2; x0 = c - nw / 2; x1 = c + nw / 2; }
    else { const nh = w / aspect, c = (y0 + y1) / 2; y0 = c - nh / 2; y1 = c + nh / 2; }
    return { x0, y0, x1, y1 };
  };
  const draw = () => {
    if (raf) return;
    raf = requestAnimationFrame(() => {
      raf = 0;
      const s = getSpec();
      rerender(view ? { ...s, xlim: [view.x0, view.x1], ylim: [view.y0, view.y1] } : s);
    });
  };
  el.addEventListener("wheel", (e) => {
    e.preventDefault();
    if (!view) view = fit();
    const r = el.getBoundingClientRect();
    const px = view.x0 + ((e.clientX - r.left) / r.width) * (view.x1 - view.x0);
    const py = view.y1 - ((e.clientY - r.top) / r.height) * (view.y1 - view.y0);
    const k = e.deltaY < 0 ? 0.85 : 1 / 0.85;
    view = { x0: px + (view.x0 - px) * k, x1: px + (view.x1 - px) * k, y0: py + (view.y0 - py) * k, y1: py + (view.y1 - py) * k };
    draw();
  }, { passive: false });
  let drag = null;
  el.addEventListener("mousedown", (e) => { if (!view) view = fit(); drag = { x: e.clientX, y: e.clientY, v: { ...view }, moved: false }; });
  window.addEventListener("mousemove", (e) => {
    if (!drag) return;
    if (Math.abs(e.clientX - drag.x) + Math.abs(e.clientY - drag.y) > 4) drag.moved = true;
    const r = el.getBoundingClientRect();
    const dx = ((e.clientX - drag.x) / r.width) * (drag.v.x1 - drag.v.x0);
    const dy = ((e.clientY - drag.y) / r.height) * (drag.v.y1 - drag.v.y0);
    view = { x0: drag.v.x0 - dx, x1: drag.v.x1 - dx, y0: drag.v.y0 + dy, y1: drag.v.y1 + dy };
    draw();
  });
  window.addEventListener("mouseup", () => { if (drag && drag.moved) el._panned = true; drag = null; });
  el.addEventListener("dblclick", () => { view = null; draw(); });
  return { reset: () => { view = null; draw(); }, redraw: draw };
}

// Re-render `el` at its container width when it changes (M2 responsive). Skips
// zero width (hidden tabs) and small jitters; fires when a tab becomes visible.
function responsive(el, atWidth) {
  let w = 0;
  new ResizeObserver(() => {
    const nw = Math.round(el.clientWidth);
    if (nw > 0 && Math.abs(nw - w) >= 12) { w = nw; atWidth(w); }
  }).observe(el);
}

// Tab bar: show one panel at a time. All demos still initialise on load (the
// panels are hidden, not un-rendered), so switching tabs is instant.
function setupTabs() {
  const tabs = [...document.querySelectorAll(".tab")];
  const panels = [...document.querySelectorAll(".tabpanel")];
  tabs.forEach((t) => t.addEventListener("click", () => {
    tabs.forEach((x) => x.classList.toggle("active", x === t));
    panels.forEach((p) => { p.hidden = p.id !== "tab-" + t.dataset.tab; });
  }));
}
setupTabs();

async function main() {
  set("status", "initialising ggplot-rs (wasm)…", true);
  set("status2", "initialising…", true);
  await init();

  try {
    scatterDemo();
  } catch (e) {
    console.error("scatter:", e);
    set("status2", "scatter error: " + (e.message || e));
  }

  let duck = null;
  try {
    duck = await setupDuck();
  } catch (e) {
    console.error("duckdb:", e);
    set("status", "DuckDB error: " + (e.message || e));
    set("status3", "DuckDB unavailable");
  }
  if (duck) {
    try { await mapDemo(duck); } catch (e) { console.error("map:", e); set("status", "map error: " + (e.message || e)); }
    try { await quakeDemo(duck); } catch (e) { console.error("quakes:", e); set("status3", "earthquakes error: " + (e.message || e)); }
    try { tableDemo(); } catch (e) { console.error("table:", e); set("tablecount", "table error: " + (e.message || e)); }
    try { galleryDemo(); } catch (e) { console.error("gallery:", e); set("gallerystatus", "gallery error: " + (e.message || e)); }
  }
}

// ── Gallery: many geoms (grammar of graphics) over the real earthquake data ──
function galleryDemo() {
  if (!quakeRows.length) { set("gallerystatus", "no earthquake data"); return; }
  const q = quakeRows;
  const mag = q.map((r) => Number(r.mag));
  const withDepth = q.filter((r) => r.depth != null && isFinite(Number(r.depth)));
  const dDepth = withDepth.map((r) => Number(r.depth));
  const dMag = withDepth.map((r) => Number(r.mag));

  const counts = {};
  q.forEach((r) => { const t = r.magType || "?"; counts[t] = (counts[t] || 0) + 1; });
  const topTypes = Object.entries(counts).sort((a, b) => b[1] - a[1]).slice(0, 4).map((e) => e[0]);
  const byType = q.filter((r) => topTypes.includes(r.magType));
  const tType = byType.map((r) => r.magType);
  const tMag = byType.map((r) => Number(r.mag));

  const perDay = {};
  q.forEach((r) => { const k = new Date(Number(r.time)).toISOString().slice(0, 10); perDay[k] = (perDay[k] || 0) + 1; });
  const days = Object.keys(perDay).sort();
  const dayIdx = days.map((_, i) => i);
  const dayCount = days.map((k) => perDay[k]);

  const charts = [
    ["Magnitude by type · boxplot", { geom: "boxplot", data: { type: tType, mag: tMag }, aes: { x: "type", y: "mag", fill: "type" }, legend: false }],
    ["Depth vs magnitude · loess fit", { geom: "point", smooth: 1, method: "loess", data: { depth: dDepth, mag: dMag }, aes: { x: "depth", y: "mag" } }],
    ["Depth vs magnitude · hexbin", { geom: "hex", data: { depth: dDepth, mag: dMag }, aes: { x: "depth", y: "mag" } }],
    ["Magnitude · density", { geom: "density", data: { mag }, aes: { x: "mag" } }],
    ["Magnitude by type · violin", { geom: "violin", data: { type: tType, mag: tMag }, aes: { x: "type", y: "mag", fill: "type" }, legend: false }],
    ["Earthquakes per day · area", { geom: "area", data: { day: dayIdx, n: dayCount }, aes: { x: "day", y: "n" } }],
  ];
  // M3 — interactive legend: click a chip to toggle a series (stable colours via
  // color_levels; the chart re-renders filtered).
  const SET1 = ["#e41a1c", "#377eb8", "#4daf4a", "#984ea3", "#ff7f00", "#ffff33", "#a65628", "#f781bf", "#999999"];
  const active = new Set(topTypes);
  const lchart = document.getElementById("legendchart");
  const chipsEl = document.getElementById("legendchips");
  const drawLegendChart = () => {
    const sel = q.filter((r) => active.has(r.magType));
    if (!sel.length) { lchart.innerHTML = "<p class='sub'>all series hidden — re-enable a type</p>"; return; }
    lchart.innerHTML = render_plot(JSON.stringify({
      geom: "density", width: Math.max(340, lchart.clientWidth || 720), height: 300,
      data: { mag: sel.map((r) => Number(r.mag)), type: sel.map((r) => r.magType) },
      aes: { x: "mag", color: "type" }, color_levels: topTypes, palette: "Set1", legend: false,
      title: "Magnitude density by type — click a chip to toggle",
    }));
    addSaveTools(lchart, "magnitude-density-by-type");
  };
  chipsEl.innerHTML = "";
  topTypes.forEach((t, i) => {
    const chip = document.createElement("button");
    chip.className = "chip on";
    chip.innerHTML = `<span class="dot" style="background:${SET1[i]}"></span>${t}`;
    chip.onclick = () => {
      if (active.has(t)) active.delete(t); else active.add(t);
      chip.classList.toggle("on", active.has(t));
      drawLegendChart();
    };
    chipsEl.appendChild(chip);
  });
  drawLegendChart();
  responsive(lchart, drawLegendChart);

  const grid = document.getElementById("gallery");
  grid.innerHTML = "";
  for (const [title, spec] of charts) {
    const fig = document.createElement("figure");
    fig.className = "gcell viz";
    try { fig.innerHTML = render_plot(JSON.stringify({ width: 430, height: 280, ...spec })); detitle(fig); }
    catch (e) { fig.innerHTML = `<p class="sub">${title}: ${e.message || e}</p>`; }
    const cap = document.createElement("figcaption");
    cap.textContent = title;
    fig.appendChild(cap);
    grid.appendChild(fig);
    addSaveTools(fig, "ggplot-" + title.replace(/[^\w]+/g, "-").toLowerCase());
  }
  hoverTips(grid);
  set("gallerystatus", `${q.length.toLocaleString()} earthquakes · ${charts.length} chart types, all drawn by ggplot-rs`);
}

async function setupDuck() {
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
  return { db, conn };
}

const registerUrl = async (db, name, url) => {
  const bytes = new Uint8Array(await (await fetch(url)).arrayBuffer());
  await db.registerFileBuffer(name, bytes);
};

// ── Choropleth with continent drill-down ──────────────────────────────────
let allRows = [];
let quakeRows = [];
const nameToContinent = {};

async function mapDemo({ db, conn }) {
  set("status", "downloading Natural Earth countries…", true);
  await registerUrl(db, "countries.geojson",
    "https://cdn.jsdelivr.net/gh/nvkelso/natural-earth-vector@master/geojson/ne_110m_admin_0_countries.geojson");

  set("status", "reading geometry…", true);
  const sql = `
    SELECT ST_AsText(geom) AS geometry, NAME AS name, CONTINENT AS continent,
           ln(POP_EST + 1) AS pop_log
    FROM ST_Read('countries.geojson')
    WHERE NAME <> 'Antarctica'`;
  allRows = (await conn.query(sql)).toArray().map((r) => r.toJSON());
  for (const r of allRows) nameToContinent[r.name] = r.continent;

  const plot = document.getElementById("plot");
  const WORLD = "World — hover/click a country, scroll to zoom, drag to pan";
  let curRows = allRows, curTitle = WORLD;
  let mapW = plot.clientWidth || 960, mapH = Math.round(mapW * 0.54);
  const specFor = () => ({
    geometry: curRows.map((r) => r.geometry),
    fill: curRows.map((r) => Number(r.pop_log)),
    label: curRows.map((r) => r.name),
    width: mapW, height: mapH, title: curTitle,
  });
  const rerender = (spec) => { plot.innerHTML = render_geo(JSON.stringify(spec)); detitle(plot); };
  rerender(specFor());
  set("status", `${allRows.length} countries loaded.`);

  hoverTips(plot);
  const roam = enableRoam(plot, specFor, rerender);
  responsive(plot, (w) => { mapW = w; mapH = Math.round(w * 0.54); roam.redraw(); });
  plot.addEventListener("click", (e) => {
    if (plot._panned) { plot._panned = false; return; } // this click ended a drag-pan
    const m = e.target.closest("[data-tip]");
    if (!m) return;
    const cont = nameToContinent[m.getAttribute("data-tip").replace(/: [^:]*$/, "")];
    if (!cont) return;
    curRows = allRows.filter((r) => r.continent === cont);
    curTitle = `${cont} — click ⟳ World to reset`;
    roam.reset();
    document.getElementById("reset").style.display = "";
  });
  document.getElementById("reset").onclick = () => {
    curRows = allRows; curTitle = WORLD;
    roam.reset();
    document.getElementById("reset").style.display = "none";
  };
}

// ── Live USGS earthquakes, coloured by magnitude ──────────────────────────
async function quakeDemo({ db, conn }) {
  set("status3", "downloading USGS earthquakes…", true);
  await registerUrl(db, "quakes.geojson",
    "https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/2.5_month.geojson");

  set("status3", "reading geometry…", true);
  const rows = (await conn.query(
    `SELECT ST_AsText(geom) AS geometry, mag, place, round(ST_Z(geom), 1) AS depth,
            magType, time
     FROM ST_Read('quakes.geojson') WHERE mag IS NOT NULL`,
  )).toArray().map((r) => r.toJSON());
  quakeRows = rows; // shared with the crossfilter table tab

  const eq = document.getElementById("eqplot");
  const baseSpec = {
    geometry: rows.map((r) => r.geometry),
    base: allRows.length ? allRows.map((r) => r.geometry) : undefined, // country basemap
    fill: rows.map((r) => Number(r.mag)),
    label: rows.map((r) => r.place),
    width: eq.clientWidth || 960, height: Math.round((eq.clientWidth || 960) * 0.5),
    title: `${rows.length} earthquakes (M≥2.5), past 30 days — colour = magnitude`,
  };
  const rerender = (spec) => { eq.innerHTML = render_geo(JSON.stringify(spec)); detitle(eq); };
  rerender(baseSpec);
  hoverTips(eq);
  const roam = enableRoam(eq, () => baseSpec, rerender); // scroll to zoom, drag to pan, dbl-click resets
  responsive(eq, (w) => { baseSpec.width = w; baseSpec.height = Math.round(w * 0.5); roam.redraw(); });
  set("status3", `${rows.length} earthquakes — hover, scroll to zoom, drag to pan.`);
}

// ── Combined graph + table (crossfilter): a Grid.js table beside a ggplot-rs
//    magnitude histogram, both driven by the min-magnitude and bin sliders. ──
function tableDemo() {
  if (!quakeRows.length) { set("tablecount", "no earthquake data"); return; }
  const hist = document.getElementById("eqhist");
  const minEl = document.getElementById("minmag");
  const binsEl = document.getElementById("bins");

  const grid = new Grid({
    columns: ["Place", "Mag", "Depth (km)", "Type", "Time (UTC)"],
    data: [],
    search: true,
    sort: true,
    pagination: { limit: 8 },
    style: { table: { "font-size": "13px" } },
  });
  grid.render(document.getElementById("eqtable"));

  const fmtTime = (ms) => {
    const d = new Date(Number(ms));
    return isNaN(d.getTime()) ? "" : d.toISOString().slice(0, 16).replace("T", " ");
  };

  let raf = 0;
  const update = () => {
    raf = 0;
    const minMag = parseFloat(minEl.value);
    const bins = parseInt(binsEl.value, 10);
    document.getElementById("minmagval").textContent = minMag.toFixed(1);
    document.getElementById("binsval").textContent = bins;
    const rows = quakeRows.filter((r) => Number(r.mag) >= minMag);
    const mags = Float64Array.from(rows, (r) => Number(r.mag));
    hist.innerHTML = mags.length
      ? render_hist(mags, bins, 460, 300, `Magnitude distribution (${rows.length})`)
      : "<p class='sub'>no earthquakes in range</p>";
    grid.updateConfig({
      data: rows.map((r) => [
        r.place, Number(r.mag).toFixed(1),
        r.depth == null ? "" : Number(r.depth).toFixed(1),
        r.magType || "", fmtTime(r.time),
      ]),
    }).forceRender();
    set("tablecount", `${rows.length} of ${quakeRows.length} earthquakes (M ≥ ${minMag.toFixed(1)}) — sort/search the table`);
  };
  const schedule = () => { if (!raf) raf = requestAnimationFrame(update); };
  minEl.addEventListener("input", schedule);
  binsEl.addEventListener("input", schedule);
  update();
}

// ── Linked views: raster scatter (brush highlights) → ggplot-rs bar ───────
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
  const empty = new Uint8Array(0);
  const blit = (r) => canvas.getContext("2d").putImageData(
    new ImageData(new Uint8ClampedArray(r.rgba), canvas.width, canvas.height), 0, 0);
  const draw = (sel) => blit(render_scatter_xy(x, y, gidx, names, sel, canvas.width, canvas.height, `${n} points`));

  const t0 = performance.now();
  const res = render_scatter_xy(x, y, gidx, names, empty, canvas.width, canvas.height, `${n} points`);
  blit(res);
  set("status2", `rendered ${n.toLocaleString()} points in ${Math.round(performance.now() - t0)} ms — hover, or drag to brush.`);

  const [px, py, pw, ph] = res.plot, [xe0, xe1] = res.xdom, [ye0, ye1] = res.ydom;
  const sx = pw / (xe1 - xe0), sy = ph / (ye1 - ye0);

  const countAll = () => { const c = { a: 0, b: 0, c: 0 }; for (let i = 0; i < n; i++) c[names[gidx[i]]]++; return c; };
  const renderBar = (c, title) => {
    const el = document.getElementById("scatterbar");
    el.innerHTML = render_bar(JSON.stringify({ category: names, value: names.map((g) => c[g]), width: 300, height: 300, title }));
    detitle(el);
  };
  renderBar(countAll(), `all ${n.toLocaleString()} points`);
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
    if (Math.abs(p.cx - s.cx) < 5 || Math.abs(p.cy - s.cy) < 5) {
      draw(empty);
      return renderBar(countAll(), `all ${n.toLocaleString()} points`);
    }
    const toData = (mx, my) => [xe0 + ((mx - px) / pw) * (xe1 - xe0), ye0 + (1 - (my - py) / ph) * (ye1 - ye0)];
    const [ax, ay] = toData(Math.min(s.cx, p.cx), Math.max(s.cy, p.cy));
    const [bx, by] = toData(Math.max(s.cx, p.cx), Math.min(s.cy, p.cy));
    const sel = new Uint8Array(n);
    const c = { a: 0, b: 0, c: 0 };
    let total = 0;
    for (let i = 0; i < n; i++) {
      if (x[i] >= ax && x[i] <= bx && y[i] >= ay && y[i] <= by) { sel[i] = 1; c[names[gidx[i]]]++; total++; }
    }
    draw(sel); // selected stay bright, the rest fade
    renderBar(c, `${total.toLocaleString()} selected`);
  });
}

main().catch((e) => {
  console.error(e);
  set("status", "fatal: " + (e.message || e));
});
