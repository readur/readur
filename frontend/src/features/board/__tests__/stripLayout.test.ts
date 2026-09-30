import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const css = readFileSync(resolve(__dirname, '../Board.module.css'), 'utf8');
const block = (query: string) => {
  const start = css.indexOf(query);
  if (start < 0) throw new Error(`${query} missing`);
  return css.slice(start, css.indexOf('\n}\n', start));
};

describe('Board pass strip on phones', () => {
  it('stacks each segment: head above the body', () => {
    const b = block('@media (max-width: 599px)');
    expect(b).toMatch(/\.segment\s*\{[^}]*flex-direction:\s*column/);
  });

  it('lays the cells out in a two-column grid under 480px', () => {
    const b = block('@media (max-width: 479px)');
    expect(b).toMatch(/\.figures\s*\{[^}]*display:\s*grid/);
    expect(b).toMatch(/grid-template-columns:\s*repeat\(2, minmax\(0, 1fr\)\)/);
  });

  it('never wraps a label or value mid-figure', () => {
    expect(block('.figureLabel {')).toMatch(/white-space:\s*nowrap/);
    expect(block('.figureValue {')).toMatch(/white-space:\s*nowrap/);
  });
});
