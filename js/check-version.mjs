// The npm package, the ggplot-rs crate and the wasm crate share one version.
// Usage: node js/check-version.mjs [tag]   (tag like v0.35.0, optional)
import { readFileSync } from 'node:fs';

const root = new URL('..', import.meta.url);
const cargoVersion = (path) =>
  readFileSync(new URL(path, root), 'utf8').match(/^version\s*=\s*"([^"]+)"/m)[1];

const versions = {
  'Cargo.toml': cargoVersion('Cargo.toml'),
  'crates/ggplot-rs-wasm/Cargo.toml': cargoVersion('crates/ggplot-rs-wasm/Cargo.toml'),
  'js/package.json': JSON.parse(readFileSync(new URL('js/package.json', root), 'utf8')).version,
};
const tag = process.argv[2];
if (tag) versions[`tag ${tag}`] = tag.replace(/^v/, '');

console.log(versions);
if (new Set(Object.values(versions)).size !== 1) {
  console.error('::error::version mismatch — bump all of these together');
  process.exit(1);
}
