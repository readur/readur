import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const base = readFileSync(resolve(__dirname, '../base.css'), 'utf8');

describe('text fields on touch screens', () => {
  it('use at least 16px text, so iOS does not zoom the page when one is tapped (and leave it scrolling sideways)', () => {
    const block = base.match(/@media \(pointer: coarse\) \{([\s\S]*?)\n\}/)?.[1] ?? '';
    expect(block).toMatch(/input:not\(\[type="checkbox"\]\):not\(\[type="radio"\]\)[^{]*textarea[^{]*select[^{]*\{[^}]*font-size:\s*max\(16px, 1em\) !important/);
  });
});
