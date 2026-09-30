import { render, screen } from '@testing-library/react';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { ChangeTag } from './ChangeTag';

describe('ChangeTag', () => {
  it('renders the word with the shared tag class and any extra class', () => {
    render(<ChangeTag className="extra">New</ChangeTag>);
    const tag = screen.getByText('New');
    expect(tag.className).toMatch(/tag/);
    expect(tag).toHaveClass('extra');
  });

  it('uses the tight 2px corner and signal fill from tokens', () => {
    const css = readFileSync(resolve(__dirname, 'ChangeTag.module.css'), 'utf8');
    expect(css).toMatch(/border-radius:\s*2px/);
    expect(css).toMatch(/background:\s*var\(--signal\)/);
    expect(css).toMatch(/color:\s*var\(--signal-ink\)/);
  });
});
