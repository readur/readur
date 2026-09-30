import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const css = readFileSync(resolve(__dirname, '../tokens.css'), 'utf8');

const blockFor = (selector: string): Record<string, string> => {
  const start = css.indexOf(selector);
  const open = css.indexOf('{', start);
  const close = css.indexOf('}', open);
  const vars: Record<string, string> = {};
  for (const m of css.slice(open + 1, close).matchAll(/--([a-z0-9-]+):\s*([^;]+);/g)) {
    vars[m[1]] = m[2].trim();
  }
  return vars;
};

const themes: Record<string, Record<string, string>> = {
  light: blockFor(':root,\n[data-theme="light"]'),
  dark: blockFor('[data-theme="dark"]'),
};

const channel = (v: number) => {
  const c = v / 255;
  return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
};

const luminance = (hex: string) => {
  const h = hex.replace('#', '');
  const [r, g, b] = [0, 2, 4].map((i) => channel(parseInt(h.slice(i, i + 2), 16)));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
};

const contrast = (a: string, b: string) => {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
};

describe.each(Object.keys(themes))('tokens contrast (%s)', (name) => {
  const t = themes[name];
  const grounds = ['ground', 'surface', 'surface-2'];

  it.each(
    ['fg', 'fg-2', 'fg-meta', 'danger', 'ok'].flatMap((fg) => grounds.map((bg) => [fg, bg])),
  )('%s on %s is at least 4.5:1', (fg, bg) => {
    expect(contrast(t[fg], t[bg])).toBeGreaterThanOrEqual(4.5);
  });

  it('signal-ink on signal is at least 4.5:1', () => {
    expect(contrast(t['signal-ink'], t.signal)).toBeGreaterThanOrEqual(4.5);
  });

  it('btn-fg on btn-bg is at least 4.5:1', () => {
    expect(contrast(t['btn-fg'], t['btn-bg'])).toBeGreaterThanOrEqual(4.5);
  });

  it.each(['ground', 'surface'])('focus on %s is at least 3:1', (bg) => {
    expect(contrast(t.focus, t[bg])).toBeGreaterThanOrEqual(3);
  });
});

describe('tokens before the theme is set', () => {
  it('a dark system gets the dark tokens until data-theme is set', () => {
    const media = css.indexOf('@media (prefers-color-scheme: dark)');
    expect(media).toBeGreaterThan(-1);
    const selector = css.indexOf(':root:not([data-theme])', media);
    expect(selector).toBeGreaterThan(media);
    const open = css.indexOf('{', selector);
    const close = css.indexOf('}', open);
    const vars: Record<string, string> = {};
    for (const m of css.slice(open + 1, close).matchAll(/--([a-z0-9-]+):\s*([^;]+);/g)) vars[m[1]] = m[2].trim();
    expect(vars).toEqual(themes.dark);
  });
});
