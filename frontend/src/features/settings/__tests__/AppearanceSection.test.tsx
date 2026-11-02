import { act, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import i18n from 'i18next';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import AppearanceSection from '../appearance/AppearanceSection';
import { installStorage, REDUCED_MOTION, renderSettings } from './settingsTestUtils';

const DARK_SYSTEM = { ...REDUCED_MOTION, '(prefers-color-scheme: dark)': true };

let storage: Storage;

beforeEach(() => {
  storage = installStorage();
});

afterEach(async () => {
  document.documentElement.removeAttribute('data-theme');
  await act(async () => {
    await i18n.changeLanguage('en');
  });
});

async function openTheme(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole('button', { name: 'Edit Theme' }));
  return screen.getByRole('radiogroup', { name: 'Theme' });
}

describe('AppearanceSection', () => {
  it('summarises the current theme and language', () => {
    storage.setItem('themeMode', 'light');
    renderSettings(<AppearanceSection />, { path: '/settings/appearance', media: DARK_SYSTEM });
    expect(screen.getByText('Light', { selector: 'p' })).toBeInTheDocument();
    expect(screen.getByText('English', { selector: 'p' })).toBeInTheDocument();
  });

  it('switches to dark and applies it to the document', async () => {
    storage.setItem('themeMode', 'light');
    const user = userEvent.setup();
    renderSettings(<AppearanceSection />, { path: '/settings/appearance', media: DARK_SYSTEM });
    await openTheme(user);
    expect(screen.getByRole('radio', { name: 'Light' })).toBeChecked();
    await user.click(screen.getByRole('radio', { name: 'Dark' }));
    expect(screen.getByRole('radio', { name: 'Dark' })).toBeChecked();
    await waitFor(() => expect(document.documentElement).toHaveAttribute('data-theme', 'dark'));
    expect(storage.getItem('themeMode')).toBe('dark');
  });

  it('follows the system when "System" is chosen', async () => {
    storage.setItem('themeMode', 'light');
    const user = userEvent.setup();
    renderSettings(<AppearanceSection />, { path: '/settings/appearance', media: DARK_SYSTEM });
    await openTheme(user);
    await user.click(screen.getByRole('radio', { name: /System/ }));
    await waitFor(() => expect(document.documentElement).toHaveAttribute('data-theme', 'dark'));
    expect(storage.getItem('themeMode')).toBeNull();
    expect(screen.getByText('System · Dark')).toBeInTheDocument();
  });

  it('starts on "System" when nothing is saved', async () => {
    const user = userEvent.setup();
    renderSettings(<AppearanceSection />, { path: '/settings/appearance', media: DARK_SYSTEM });
    await openTheme(user);
    expect(screen.getByRole('radio', { name: /System/ })).toBeChecked();
  });

  it('changes the interface language', async () => {
    const user = userEvent.setup();
    renderSettings(<AppearanceSection />, { path: '/settings/appearance', media: DARK_SYSTEM });
    await user.click(screen.getByRole('button', { name: 'Edit Interface language' }));
    expect(screen.getByRole('radio', { name: 'English' })).toBeChecked();
    await user.click(screen.getByRole('radio', { name: 'Deutsch' }));
    await waitFor(() => expect(i18n.language).toBe('de'));
  });
});
