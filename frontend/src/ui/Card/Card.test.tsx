import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { Card } from './Card';

describe('Card', () => {
  it('renders the requested element with padding data attr', () => {
    render(<Card as="section" aria-label="Sources" padding="md">x</Card>);
    const el = screen.getByRole('region', { name: 'Sources' });
    expect(el.tagName).toBe('SECTION');
    expect(el).toHaveAttribute('data-padding', 'md');
  });
});
