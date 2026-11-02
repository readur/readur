import { screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { renderWithProviders } from '../../../test/test-utils';
import { ThemeModeProvider } from '../../../theme/ThemeProvider';
import { ToastProvider } from '../../../ui';
import Gallery from '../Gallery';

describe('primitive gallery', () => {
  it('shows every group of primitives', () => {
    renderWithProviders(
      <ThemeModeProvider>
        <ToastProvider>
          <Gallery />
        </ToastProvider>
      </ThemeModeProvider>,
    );
    for (const name of ['Actions', 'Inputs', 'Status', 'Data', 'Overlays']) {
      expect(screen.getByRole('heading', { name, level: 2 })).toBeInTheDocument();
    }
    expect(screen.getByRole('heading', { name: 'Studio primitives', level: 1 })).toBeInTheDocument();
  });
});
