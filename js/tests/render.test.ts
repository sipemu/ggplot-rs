import { beforeAll, describe, expect, it } from 'vitest';
import { geoms, render, renderWithWarnings, version, type PlotSpec } from '../dist/ggplot_rs.js';
import { readFileSync } from 'node:fs';
import { init } from './setup';

beforeAll(init);

const data = {
  x: [1, 2, 3, 4, 5, 6],
  y: [2.1, 3.9, 6.2, 7.8, 10.1, 12.2],
  g: ['a', 'a', 'a', 'b', 'b', 'b'],
};

describe('render', () => {
  it('renders a scatter from a typed spec object', () => {
    const spec: PlotSpec = {
      data,
      aes: { x: 'x', y: 'y', color: 'g' },
      layers: [{ geom: 'point' }],
      labels: { title: 'Hello npm' },
    };
    const svg = render(spec);
    expect(svg.startsWith('<svg')).toBe(true);
    expect(svg).toContain('Hello npm');
    expect(svg.match(/<circle/g)?.length).toBeGreaterThanOrEqual(6);
  });

  it('accepts JSON text too', () => {
    const svg = render(JSON.stringify({ data, aes: { x: 'x', y: 'y' }, layers: [{ geom: 'line' }] }));
    expect(svg).toContain('<svg');
  });

  it('honours width/height', () => {
    const svg = render({ data, aes: { x: 'x', y: 'y' }, layers: [{ geom: 'point' }], width: 320, height: 200 });
    expect(svg).toContain('width="320"');
    expect(svg).toContain('height="200"');
  });

  it('renders multiple layers, facets, scales, theme and legend', () => {
    const svg = render({
      data,
      aes: { x: 'x', y: 'y', color: 'g' },
      layers: [
        { geom: 'point' },
        { geom: 'smooth', params: { method: 'lm', se: false } },
        { geom: 'hline', params: { yintercept: 6 } },
      ],
      facet: { wrap: 'g', ncol: 2 },
      scales: { color: { palette: 'Dark2' }, y: { limits: [0, 14] } },
      theme: { base: 'bw', title: { size: 18, face: 'bold' } },
      legend: 'bottom',
      labels: { title: 'Layered', x: 'Dose', y: 'Response' },
    });
    expect(svg).toContain('Layered');
    expect(svg).toContain('Dose');
  });

  it('renders the newer chart types', () => {
    const treemap = render({
      data: { k: ['a', 'b', 'c'], v: [5, 3, 1] },
      aes: { weight: 'v', label: 'k', fill: 'k' },
      layers: [{ geom: 'treemap' }],
    });
    expect(treemap).toContain('<svg');
    const sankey = render({
      data: { s: ['a', 'a', 'b'], t: ['b', 'c', 'c'], w: [3, 2, 1] },
      aes: { from: 's', to: 't', weight: 'w' },
      layers: [{ geom: 'sankey' }],
    });
    expect(sankey).toContain('<svg');
    const waterfall = render({
      data: { step: ['start', 'sales', 'costs'], delta: [10, 5, -3] },
      aes: { x: 'step', y: 'delta' },
      layers: [{ geom: 'waterfall' }],
    });
    expect(waterfall).toContain('<svg');
  });

  it('supports bars with position and coord_flip', () => {
    const svg = render({
      data: { cat: ['a', 'b', 'a', 'b'], sub: ['x', 'x', 'y', 'y'], n: [3, 4, 2, 5] },
      aes: { x: 'cat', y: 'n', fill: 'sub' },
      layers: [{ geom: 'col', position: 'dodge' }],
      coord: 'flip',
      scales: { fill: { values: { x: '#1b9e77', y: '#d95f02' } } },
    });
    expect(svg).toContain('<svg');
  });
});

describe('errors', () => {
  it('names an unknown column and lists the available ones', () => {
    expect(() => render({ data, aes: { x: 'nope', y: 'y' }, layers: [{ geom: 'point' }] })).toThrow(
      /column 'nope' not found \(available: g, x, y\)/,
    );
  });

  it('throws real Error objects', () => {
    try {
      render({ data, layers: [{ geom: 'pointy' as never }] });
      expect.unreachable();
    } catch (e) {
      expect(e).toBeInstanceOf(Error);
      expect((e as Error).message).toMatch(/unknown geom 'pointy'/);
    }
  });

  it('rejects unknown spec fields', () => {
    expect(() => render({ data, layers: [{ geom: 'point' }], titel: 'x' } as unknown as PlotSpec)).toThrow(
      /invalid plot spec/,
    );
  });

  it('rejects unequal column lengths', () => {
    expect(() => render({ data: { x: [1, 2], y: [1] }, aes: { x: 'x', y: 'y' }, layers: [{ geom: 'point' }] })).toThrow(
      /equal length/,
    );
  });
});

describe('warnings + metadata', () => {
  it('returns warnings alongside the SVG', () => {
    const r = renderWithWarnings({
      data: { x: [1, 2, null, 4], y: [1, 2, 3, 4] },
      aes: { x: 'x', y: 'y' },
      layers: [{ geom: 'point' }],
    });
    expect(r.svg).toContain('<svg');
    expect(Array.isArray(r.warnings)).toBe(true);
  });

  it('lists geoms and reports the crate version (= package version)', () => {
    expect(geoms()).toContain('point');
    expect(geoms()).toContain('sankey');
    const pkg = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8'));
    expect(version()).toBe(pkg.version);
  });
});
