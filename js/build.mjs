// Build the npm package: wasm-pack → dist/, then copy the licenses in.
// SVG-only (`--no-default-features` drops the demo's raster `canvas`
// exports), which keeps the .wasm ~45% smaller.
import { execFileSync } from 'node:child_process';
import { copyFileSync, rmSync } from 'node:fs';

execFileSync(
  'wasm-pack',
  [
    'build', '../crates/ggplot-rs-wasm',
    '--release', '--target', 'web',
    '--out-dir', '../../js/dist', '--out-name', 'ggplot_rs',
    '--no-pack', '--no-default-features',
  ],
  { stdio: 'inherit' },
);
// wasm-pack drops a `*` .gitignore into dist/, which npm would honour and
// exclude the whole build from the tarball.
rmSync('dist/.gitignore', { force: true });
for (const f of ['LICENSE-MIT', 'LICENSE-APACHE']) copyFileSync(`../${f}`, f);
