import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { initSync } from '../dist/ggplot_rs.js';

// Node has no fetch-from-file for `init()`; hand the bytes to `initSync`.
let done = false;
export function init(): void {
  if (done) return;
  const wasm = readFileSync(fileURLToPath(new URL('../dist/ggplot_rs_bg.wasm', import.meta.url)));
  initSync({ module: wasm });
  done = true;
}
