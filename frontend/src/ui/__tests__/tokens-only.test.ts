import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const root = resolve(__dirname, '..');
const cssFiles = (dir: string): string[] =>
  readdirSync(dir).flatMap((name) => {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) return cssFiles(p);
    return p.endsWith('.module.css') ? [p] : [];
  });

describe('ui primitives use tokens only', () => {
  const files = cssFiles(root);
  it('finds the stylesheets', () => expect(files.length).toBeGreaterThan(20));
  it.each(files.map((f) => [f.slice(root.length + 1), f]))('%s has no raw colours, radii, shadows or z-index', (_, f) => {
    const css = readFileSync(f, 'utf8').replace(/\/\*[\s\S]*?\*\//g, '');
    expect(css, 'hex colour').not.toMatch(/#[0-9a-fA-F]{3,8}\b/);
    expect(css, 'rgb()/hsl()').not.toMatch(/\b(rgba?|hsla?)\(/);
    expect(css, 'px border-radius').not.toMatch(/border-radius:\s*\d+px/);
    expect(css, 'literal box-shadow').not.toMatch(/box-shadow:\s*(?!none|var\(|inset 0 0 0 \d+px var\(|0 0 0 \d+px var\()[^;]*\d+px[^;]*\brgba?\(/);
    expect(css, 'raw z-index').not.toMatch(/z-index:\s*\d/);
  });
});
