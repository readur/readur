import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
// @ts-expect-error: a plain .mjs helper shared with the CLI check, without type declarations.
import { ACCENT_HUE_GAP, PAIRS, blockVars, check, contrast, hueFailures, hueOf, parseThemes } from '../../../../scripts/contrast-check.mjs';

const css = readFileSync(resolve(__dirname, '../tokens.css'), 'utf8');
const themes = parseThemes(css) as Record<'light' | 'dark', Record<string, string>>;

type Pair = [string, string, number];

describe.each(['light', 'dark'] as const)('tokens contrast (%s)', (name) => {
  const t = themes[name];

  it.each(PAIRS as Pair[])('%s on %s is at least %s:1', (fg, bg, min) => {
    expect(t[fg], `--${fg}`).toMatch(/^#[0-9A-Fa-f]{6}$/);
    expect(t[bg], `--${bg}`).toMatch(/^#[0-9A-Fa-f]{6}$/);
    expect(contrast(t[fg], t[bg])).toBeGreaterThanOrEqual(min);
  });

  it('defines eight source hues, each with a tint', () => {
    for (let n = 1; n <= 8; n += 1) {
      expect(t[`src-${n}`]).toBeDefined();
      expect(t[`src-${n}-soft`]).toBeDefined();
    }
  });

  it('declares the same token names as the other theme', () => {
    expect(Object.keys(t).sort()).toEqual(Object.keys(themes.light).sort());
  });
});

describe('contrast-check script', () => {
  it('reports no failures for the shipped tokens', () => {
    expect(check(css)).toEqual([]);
  });

  it('keeps every source hue away from the accent', () => {
    expect(hueFailures(css)).toEqual([]);
    expect(ACCENT_HUE_GAP).toBeGreaterThanOrEqual(25);
  });

  it('reports a source hue that sits on the accent', () => {
    const blueSource = css.replace('--src-1: #107064;', '--src-1: #3767AD;');
    expect(hueFailures(blueSource)).toEqual([expect.objectContaining({ theme: 'light', token: 'src-1' })]);
  });

  it('measures hue in degrees', () => {
    expect(hueOf('#FF0000')).toBeCloseTo(0);
    expect(hueOf('#00FF00')).toBeCloseTo(120);
    expect(hueOf('#0000FF')).toBeCloseTo(240);
    expect(hueOf('#808080')).toBe(0);
  });

  it('reports a failing pair', () => {
    const broken = css.replace('--fg-meta: #5A6472;', '--fg-meta: #C0C4C8;');
    const failures = check(broken) as { theme: string; fg: string }[];
    expect(failures.some((f) => f.theme === 'light' && f.fg === 'fg-meta')).toBe(true);
  });
});

describe('older token names', () => {
  it('stay defined as aliases of the new ones', () => {
    const aliases = blockVars(css, '/* Older names') as Record<string, string>;
    expect(aliases).toMatchObject({
      ground: 'var(--bg)',
      signal: 'var(--new)',
      'signal-ink': 'var(--new-fg)',
      'danger-bg': 'var(--danger-soft)',
      focus: 'var(--accent)',
      'btn-bg': 'var(--accent)',
      'btn-fg': 'var(--accent-fg)',
    });
  });
});

describe('tokens before the theme is set', () => {
  it('a dark system gets the dark tokens until data-theme is set', () => {
    const media = css.indexOf('@media (prefers-color-scheme: dark)');
    expect(media).toBeGreaterThan(-1);
    const selector = css.indexOf(':root:not([data-theme])', media);
    expect(selector).toBeGreaterThan(media);
    expect(blockVars(css.slice(media), ':root:not([data-theme])')).toEqual(themes.dark);
  });
});

describe('shape scale', () => {
  it('uses 6px controls, 8px panels and 12px overlays', () => {
    expect(css).toMatch(/--radius-sm:\s*6px;/);
    expect(css).toMatch(/--radius:\s*8px;/);
    expect(css).toMatch(/--radius-lg:\s*12px;/);
  });
});
