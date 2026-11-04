// The agent skill shipped in the package (skills/ggplot-rs-js/SKILL.md) must
// match the build: every geom `geoms()` returns is documented, and every
// import-free ```js example in it runs without throwing.
import { beforeAll, describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { geoms, render, renderWithWarnings, version } from '../dist/ggplot_rs.js';
import { init } from './setup';

beforeAll(init);

const md = readFileSync(new URL('../../skills/ggplot-rs-js/SKILL.md', import.meta.url), 'utf8');
const examples = [...md.matchAll(/```js\n([\s\S]*?)```/g)]
  .map((m) => m[1])
  .filter((code) => !/^\s*import /m.test(code));

describe('ggplot-rs-js skill', () => {
  it('documents every geom', () => {
    const geomSection = md.split('\n## Geoms\n')[1].split('\n## ')[0];
    const missing = geoms().filter((g) => !new RegExp(`[\\s\`]${g}[\\s\`]`).test(geomSection));
    expect(missing).toEqual([]);
  });

  it('has runnable examples', () => {
    expect(examples.length).toBeGreaterThanOrEqual(3);
  });

  it.each(examples.map((code, i) => [i, code]))('example %i runs', (_i, code) => {
    const run = new Function('render', 'renderWithWarnings', 'geoms', 'version', code);
    expect(() => run(render, renderWithWarnings, geoms, version)).not.toThrow();
  });
});
