// Headless smoke test for the ggplot-rs web demo (Playwright + Chromium).
//
//   npm i && npx playwright install chromium
//   node smoke.mjs                         # tests the live demo
//   DEMO_URL=http://localhost:8080 node smoke.mjs   # a local build
//
// Verifies every panel actually renders (DuckDB-Wasm + spatial + the WASM
// renderer), that there are no console/page errors, exercises the map roam
// gesture, and writes a full-page screenshot. Exits non-zero on any failure —
// suitable for gating a deploy.

import { chromium } from "playwright";

const TARGET = process.env.DEMO_URL || "https://sipemu.github.io/ggplot-rs/";
const OUT = process.env.SHOT || new URL("out.png", import.meta.url).pathname;
const TIMEOUT = Number(process.env.TIMEOUT || 60000);

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1100, height: 2400 } });
const errors = [];
page.on("console", (m) => m.type() === "error" && errors.push("console: " + m.text()));
page.on("pageerror", (e) => errors.push("pageerror: " + e.message));

const failed = [];
const results = {};
try {
  await page.goto(TARGET, { waitUntil: "load", timeout: TIMEOUT });

  // Panels live in tabs (hidden ≠ removed), so wait for "attached", not "visible".
  const panels = {
    choropleth: "#plot svg", earthquakes: "#eqplot svg", linkedBar: "#scatterbar svg",
    histogram: "#eqhist svg", dataTable: "#eqtable table", gallery: "#gallery svg",
  };
  for (const [name, sel] of Object.entries(panels)) {
    try { await page.waitForSelector(sel, { state: "attached", timeout: TIMEOUT }); results[name] = "ok"; }
    catch { results[name] = "MISSING"; failed.push(name); }
  }

  // Tab switching: each tab reveals its panel and hides the others.
  results.tabs = {};
  for (const [tab, panel] of [["quakes", "#tab-quakes"], ["linked", "#tab-linked"], ["table", "#tab-table"], ["gallery", "#tab-gallery"], ["map", "#tab-map"]]) {
    await page.click(`.tab[data-tab="${tab}"]`);
    const shown = await page.isVisible(panel);
    results.tabs[tab] = shown ? "ok" : "HIDDEN";
    if (!shown) failed.push("tab:" + tab);
  }

  // The scatter is a raster canvas — assert it has non-white pixels.
  results.scatter = await page.evaluate(() => {
    const c = document.getElementById("scatter");
    if (!c) return "MISSING";
    const d = c.getContext("2d").getImageData(0, 0, c.width, c.height).data;
    let n = 0;
    for (let i = 0; i < d.length; i += 4000) if (d[i] < 250 || d[i + 1] < 250 || d[i + 2] < 250) n++;
    return n > 5 ? "ok" : "blank";
  });
  if (results.scatter !== "ok") failed.push("scatter");

  results.statuses = await page.evaluate(() =>
    ["status", "status2", "status3"].map((id) => document.getElementById(id)?.textContent || ""));
  for (const s of results.statuses) if (/error|fatal|unavailable/i.test(s)) failed.push("status: " + s);

  // Exercise the earthquake-map roam (scroll-to-zoom) — must not throw.
  await page.click('.tab[data-tab="quakes"]');
  const b = await (await page.$("#eqplot")).boundingBox();
  if (b) {
    await page.mouse.move(b.x + b.width / 2, b.y + b.height / 2);
    await page.mouse.wheel(0, -500);
    await page.waitForTimeout(400);
  }

  await page.screenshot({ path: OUT, fullPage: true });
} catch (e) {
  errors.push("fatal: " + e.message);
}
await browser.close();

console.log(JSON.stringify({ url: TARGET, results, errors }, null, 2));
if (errors.length || failed.length) {
  console.error("FAIL:", { failed, errors });
  process.exit(1);
}
console.log("PASS — all panels rendered, no console errors. Screenshot:", OUT);
