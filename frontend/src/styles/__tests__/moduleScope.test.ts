import { readFileSync, readdirSync } from 'node:fs';
import { join, relative, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const src = resolve(__dirname, '../..');

function modules(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) return entry.name === 'node_modules' ? [] : modules(path);
    return entry.name.endsWith('.module.css') ? [path] : [];
  });
}

/** Every selector in a stylesheet, comments and at-rule preludes left out. */
function selectors(css: string): string[] {
  const plain = css.replace(/\/\*[\s\S]*?\*\//g, '');
  return Array.from(plain.matchAll(/([^{}]+)\{/g))
    .map((m) => m[1].trim())
    .filter((prelude) => prelude && !prelude.startsWith('@'))
    .flatMap((prelude) => prelude.split(',').map((s) => s.trim()));
}

/*
 * CSS Modules rename classes, not attributes: a rule like `[data-tone='neutral']` in a module is
 * global, and reaches every other component that uses the same attribute.
 */
describe('CSS modules stay scoped', () => {
  it('has no rule that is only an attribute selector', () => {
    const leaks = modules(src).flatMap((file) =>
      selectors(readFileSync(file, 'utf8'))
        .filter((s) => /^\[[^\]]+\]$/.test(s))
        .map((s) => `${relative(src, file)}: ${s}`),
    );
    expect(leaks).toEqual([]);
  });
});
