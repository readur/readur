import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import * as icons from './icons';

describe('icons', () => {
  it('keeps data-testid on rendered icons', () => {
    render(<icons.Refresh />);
    expect(screen.getByTestId('RefreshIcon')).toBeInTheDocument();
  });

  it('exports the full icon set', () => {
    expect(Object.keys(icons).length).toBeGreaterThan(100);
  });
});
