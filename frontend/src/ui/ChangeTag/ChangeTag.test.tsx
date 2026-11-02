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

  it('is a coral dot and word from the new-signal tokens', () => {
    const css = readFileSync(resolve(__dirname, 'ChangeTag.module.css'), 'utf8');
    expect(css).toMatch(/color:\s*var\(--new\)/);
    expect(css).toMatch(/::before[^}]*background:\s*currentColor/);
  });
});
