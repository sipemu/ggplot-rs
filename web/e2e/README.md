# Web demo — headless smoke test

Drives the demo in headless Chromium (Playwright): loads it, waits for every
panel to render (DuckDB-Wasm + spatial + the WASM renderer), asserts there are
no console/page errors, exercises the map roam gesture, and screenshots.

```sh
npm install
npx playwright install chromium      # one-time browser download
npm test                             # tests the live demo (sipemu.github.io)

# against a local build:
#   (from repo root) wasm-pack build --target web --out-dir web/pkg --no-default-features --features wasm,canvas
#   python3 -m http.server -d web 8080
DEMO_URL=http://localhost:8080 npm test
```

Exits non-zero on any failure, so it can gate a deploy. Env: `DEMO_URL`,
`TIMEOUT` (ms), `SHOT` (screenshot path).
