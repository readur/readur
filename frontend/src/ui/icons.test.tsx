import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import * as shim from '../design/icons';
import * as icons from './icons';

describe('icons', () => {
  it('keeps data-testid on rendered icons', () => {
    render(<icons.Refresh />);
    expect(screen.getByTestId('RefreshIcon')).toBeInTheDocument();
  });

  it('design/icons shim re-exports exactly the same names and components', () => {
    expect(Object.keys(shim).sort()).toEqual(Object.keys(icons).sort());
    expect(Object.keys(icons).length).toBeGreaterThan(100);
    expect(shim.Close).toBe(icons.Close);
  });
});
