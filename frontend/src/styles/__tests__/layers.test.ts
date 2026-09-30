import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const src = resolve(__dirname, '../..');
const read = (p: string) => readFileSync(resolve(src, p), 'utf8');

const tokens = read('styles/tokens.css');
const scale = (name: string) => {
  const m = tokens.match(new RegExp(`--${name}:\\s*(\\d+)`));
  if (!m) throw new Error(`--${name} missing from tokens.css`);
  return Number(m[1]);
};

/** The z-index a stylesheet declares for `selector`. */
const layerOf = (file: string, selector: string): string => {
  const css = read(file);
  const block = css.match(new RegExp(`(?:^|\\n)${selector.replace('.', '\\.')}\\s*\\{([^}]*)\\}`));
  const z = block?.[1].match(/z-index:\s*([^;]+);/);
  if (!z) throw new Error(`${selector} in ${file} has no z-index`);
  return z[1].trim();
};

describe('layer scale', () => {
  it('orders shell < overlay < toast < palette', () => {
    expect(scale('z-shell')).toBeLessThan(scale('z-overlay'));
    expect(scale('z-overlay')).toBeLessThan(scale('z-toast'));
    expect(scale('z-toast')).toBeLessThan(scale('z-palette'));
  });

  it('puts the app bar and tab bar on the shell layer', () => {
    expect(layerOf('features/shell/AppShell.module.css', '.topBar')).toBe('var(--z-shell)');
    expect(layerOf('features/shell/AppShell.module.css', '.bottomBar')).toBe('var(--z-shell)');
  });

  it('puts SlideOver and Dialog above the shell, Toast above them, the palette on top', () => {
    expect(layerOf('ui/SlideOver/SlideOver.module.css', '.overlay')).toBe('var(--z-overlay)');
    expect(layerOf('ui/Dialog/Dialog.module.css', '.overlay')).toBe('var(--z-overlay)');
    expect(layerOf('ui/Toast/Toast.module.css', '.region')).toBe('var(--z-toast)');
    expect(layerOf('ui/CommandPalette/CommandPalette.module.css', '.overlay')).toBe('var(--z-palette)');
  });

  it('seats the BulkActionBar dock above the shell bars and below every overlay', () => {
    expect(layerOf('ui/BulkActionBar/BulkActionBar.module.css', '.dock')).toBe('var(--z-dock)');
    expect(scale('z-dock')).toBeGreaterThan(scale('z-shell'));
    expect(scale('z-dock')).toBeLessThan(scale('z-overlay'));
  });

  /*
   * The one exception to the scale. React Aria positions every Popover (menus, selects, combo
   * boxes, tooltips' popovers) with an inline `z-index: 100000`, which beats any stylesheet value,
   * so Popover declares no z-index: a token there would be dead code that only looks like it
   * follows the scale. 100000 sits above every token layer, which is what an anchored popup needs.
   */
  it('leaves Popover off the token scale: React Aria sets its z-index inline', () => {
    expect(read('ui/Popover/Popover.module.css')).not.toMatch(/z-index/);
    expect(scale('z-palette')).toBeLessThan(100000);
  });

  it('keeps raw z-index numbers out of the layered stylesheets', () => {
    for (const f of [
      'features/shell/AppShell.module.css',
      'ui/SlideOver/SlideOver.module.css',
      'ui/Dialog/Dialog.module.css',
      'ui/Toast/Toast.module.css',
      'ui/BulkActionBar/BulkActionBar.module.css',
      'ui/CommandPalette/CommandPalette.module.css',
    ]) {
      expect(read(f), f).not.toMatch(/z-index:\s*\d/);
    }
  });
});

describe('backdrops', () => {
  it('every overlay backdrop uses the --scrim token', () => {
    for (const f of ['ui/Dialog/Dialog.module.css', 'ui/SlideOver/SlideOver.module.css', 'ui/CommandPalette/CommandPalette.module.css']) {
      const block = read(f).match(/(?:^|\n)\.overlay\s*\{([^}]*)\}/)?.[1] ?? '';
      expect(block, f).toMatch(/background:\s*var\(--scrim\);/);
    }
  });
});
