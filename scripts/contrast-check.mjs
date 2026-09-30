#!/usr/bin/env node
// Checks every text/background and mark/background pair in frontend/src/styles/tokens.css
// against WCAG 2.2 AA, in both themes. Run: node scripts/contrast-check.mjs
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const TOKENS_PATH = resolve(dirname(fileURLToPath(import.meta.url)), '../frontend/src/styles/tokens.css');

/** The `--name: value` pairs declared directly inside the first block that follows `selector`. */
export function blockVars(css, selector) {
  const start = css.indexOf(selector);
  if (start < 0) throw new Error(`selector not found: ${selector}`);
  const open = css.indexOf('{', start);
  const close = css.indexOf('}', open);
  const vars = {};
  for (const m of css.slice(open + 1, close).matchAll(/--([a-z0-9-]+):\s*([^;]+);/g)) vars[m[1]] = m[2].trim();
  return vars;
}

export function parseThemes(css) {
  return {
    light: blockVars(css, ':root,\n[data-theme="light"]'),
    dark: blockVars(css, '[data-theme="dark"]'),
  };
}

const channel = (v) => {
  const c = v / 255;
  return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
};

export function luminance(hex) {
  const h = hex.replace('#', '');
  const [r, g, b] = [0, 2, 4].map((i) => channel(parseInt(h.slice(i, i + 2), 16)));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

export function contrast(a, b) {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

const GROUNDS = ['bg', 'surface', 'surface-2'];
const SOURCES = [1, 2, 3, 4, 5, 6, 7, 8];

/** [foreground, background, minimum ratio] triples every theme must meet. */
export const PAIRS = [
  // Body and secondary text on every page surface.
  ...['fg', 'fg-2', 'fg-meta', 'accent', 'new', 'ok', 'warn', 'danger'].flatMap((fg) =>
    GROUNDS.map((bg) => [fg, bg, 4.5]),
  ),
  ['fg', 'surface-sunken', 4.5],
  ['fg-2', 'surface-sunken', 4.5],
  // Text on filled controls and tinted pills.
  ['accent-fg', 'accent', 4.5],
  ['accent-fg', 'accent-hover', 4.5],
  ['new-fg', 'new', 4.5],
  ['danger-fg', 'danger', 4.5],
  ['fg', 'selection', 4.5],
  ['accent', 'accent-soft', 4.5],
  ['new', 'new-soft', 4.5],
  ['ok', 'ok-soft', 4.5],
  ['warn', 'warn-soft', 4.5],
  ['danger', 'danger-soft', 4.5],
  ['fg', 'accent-soft', 4.5],
  // Source hues: text on surfaces and on their own tint.
  ...SOURCES.flatMap((n) => [
    ...GROUNDS.map((bg) => [`src-${n}`, bg, 4.5]),
    [`src-${n}`, `src-${n}-soft`, 4.5],
    [`fg`, `src-${n}-soft`, 4.5],
  ]),
  // UI marks: focus ring, control borders, the new edge bar. (--warn-fill is decorative only;
  // a warning always carries its word in --warn.)
  ...['accent', 'line-strong', 'new-fill'].flatMap((fg) => ['bg', 'surface'].map((bg) => [fg, bg, 3])),
];

/** Every failing pair, per theme. */
export function check(css = readFileSync(TOKENS_PATH, 'utf8')) {
  const themes = parseThemes(css);
  const failures = [];
  for (const [name, t] of Object.entries(themes)) {
    for (const [fg, bg, min] of PAIRS) {
      if (!t[fg] || !t[bg]) {
        failures.push({ theme: name, fg, bg, min, ratio: NaN, missing: true });
        continue;
      }
      const ratio = contrast(t[fg], t[bg]);
      if (ratio < min) failures.push({ theme: name, fg, bg, min, ratio });
    }
  }
  return failures;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const failures = check();
  if (failures.length === 0) {
    console.log(`contrast-check: ${PAIRS.length} pairs pass in light and dark`);
  } else {
    for (const f of failures) {
      console.error(
        f.missing
          ? `${f.theme}: --${f.fg} or --${f.bg} is missing`
          : `${f.theme}: --${f.fg} on --${f.bg} is ${f.ratio.toFixed(2)}:1 (needs ${f.min}:1)`,
      );
    }
    process.exit(1);
  }
}
