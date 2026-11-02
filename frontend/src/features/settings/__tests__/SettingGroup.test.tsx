import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it } from 'vitest';
import { createResponsiveMatchMediaMock } from '../../../test/pwa-test-utils';
import { SettingGroup } from '../fold/SettingGroup';
import { renderSettings } from './settingsTestUtils';

const group = (
  <SettingGroup id="ocr-processing" title="Processing" summary="4 jobs · 300s">
    <label>
      Jobs <input defaultValue="4" />
    </label>
  </SettingGroup>
);

function setReducedMotion(reduced: boolean) {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    configurable: true,
    value: createResponsiveMatchMediaMock({ '(prefers-reduced-motion: reduce)': reduced }),
  });
}

beforeEach(() => setReducedMotion(false));

describe('SettingGroup', () => {
  it('starts as a summary row with the name, current values and an Edit button', () => {
    renderSettings(group, { media: null });
    expect(screen.getByRole('heading', { level: 3, name: 'Processing' })).toBeInTheDocument();
    expect(screen.getByText('4 jobs · 300s')).toBeInTheDocument();
    const toggle = screen.getByRole('button', { name: 'Edit Processing' });
    expect(toggle).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByRole('textbox')).not.toBeInTheDocument();
  });

  it('points aria-controls at a labelled region that is hidden while folded', () => {
    renderSettings(group, { media: null });
    const toggle = screen.getByRole('button', { name: 'Edit Processing' });
    const region = document.getElementById(toggle.getAttribute('aria-controls') ?? '');
    expect(region).not.toBeNull();
    expect(region).toHaveAttribute('hidden');
    expect(region).toHaveAttribute('role', 'region');
  });

  it('unfolds on click and folds again', async () => {
    const user = userEvent.setup();
    renderSettings(group, { media: null });
    await user.click(screen.getByRole('button', { name: 'Edit Processing' }));
    const close = screen.getByRole('button', { name: 'Close Processing' });
    expect(close).toHaveAttribute('aria-expanded', 'true');
    const region = screen.getByRole('region', { name: 'Processing' });
    expect(region).toBeVisible();
    // The height animates from the next frame.
    await waitFor(() => expect(region).toHaveAttribute('data-open'));
    expect(screen.getByRole('textbox', { name: 'Jobs' })).toBeInTheDocument();
    await user.click(close);
    expect(screen.getByRole('button', { name: 'Edit Processing' })).toHaveAttribute('aria-expanded', 'false');
    await waitFor(() => expect(screen.queryByRole('textbox')).not.toBeInTheDocument());
  });

  it('unfolds and folds from the keyboard', async () => {
    setReducedMotion(true);
    const user = userEvent.setup();
    renderSettings(group, { media: null });
    await user.tab();
    expect(screen.getByRole('button', { name: 'Edit Processing' })).toHaveFocus();
    await user.keyboard('{Enter}');
    expect(screen.getByRole('button', { name: 'Close Processing' })).toHaveAttribute('aria-expanded', 'true');
    await user.tab();
    expect(screen.getByRole('textbox', { name: 'Jobs' })).toHaveFocus();
    await user.tab({ shift: true });
    await user.keyboard(' ');
    expect(screen.getByRole('button', { name: 'Edit Processing' })).toHaveAttribute('aria-expanded', 'false');
  });

  it('opens the group named in the URL hash', () => {
    renderSettings(group, { path: '/settings/ocr#ocr-processing', media: null });
    expect(screen.getByRole('button', { name: 'Close Processing' })).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByRole('textbox', { name: 'Jobs' })).toBeInTheDocument();
  });

  it('folds instantly under reduced motion', async () => {
    setReducedMotion(true);
    const user = userEvent.setup();
    renderSettings(group, { media: null });
    await user.click(screen.getByRole('button', { name: 'Edit Processing' }));
    const region = screen.getByRole('region', { name: 'Processing' });
    expect(region).toHaveAttribute('data-open');
    await user.click(screen.getByRole('button', { name: 'Close Processing' }));
    // No transition to wait for: the region is hidden in the same update.
    expect(region).toHaveAttribute('hidden');
  });
});
