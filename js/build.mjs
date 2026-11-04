// Build the npm package: wasm-pack → dist/, then copy the licenses and the
// agent skill in.
// SVG-only (`--no-default-features` drops the demo's raster `canvas`
// exports), which keeps the .wasm ~45% smaller.
import { execFileSync } from 'node:child_process';
import { copyFileSync, cpSync, rmSync } from 'node:fs';

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
// Ship the agent skill (an LLM how-to for this package) inside the tarball:
// node_modules/@sipemu/ggplot-rs/skills/ggplot-rs-js/SKILL.md.
rmSync('skills', { recursive: true, force: true });
cpSync('../skills/ggplot-rs-js', 'skills/ggplot-rs-js', { recursive: true });
