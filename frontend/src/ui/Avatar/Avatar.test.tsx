import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { Avatar } from './Avatar';

describe('Avatar', () => {
  it('shows two initials and names the person', () => {
    render(<Avatar name="Alex Lindqvist" />);
    expect(screen.getByRole('img', { name: 'Alex Lindqvist' })).toHaveTextContent('AL');
  });
  it('uses the first two letters for a single name', () => {
    render(<Avatar name="alex" />);
    expect(screen.getByRole('img', { name: 'alex' })).toHaveTextContent('AL');
  });
});
