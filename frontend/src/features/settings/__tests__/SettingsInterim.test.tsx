import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes, useLocation } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import SettingsInterim from '../SettingsInterim';

vi.mock('../../../pages/SettingsPage', () => ({ default: () => <p>settings page</p> }));
vi.mock('../../../pages/LabelsPage', () => ({ default: () => <p>labels page</p> }));
vi.mock('../../../pages/DebugPage', () => ({ default: () => <p>debug page</p> }));

function Where() {
  return <output aria-label="location">{useLocation().pathname}</output>;
}

const renderAt = (path: string) =>
  render(
    <MemoryRouter initialEntries={[path]}>
      <Routes>
        <Route
          path="/settings/:section?"
          element={
            <>
              <SettingsInterim />
              <Where />
            </>
          }
        />
      </Routes>
    </MemoryRouter>,
  );

describe('SettingsInterim', () => {
  it.each([
    ['/settings', 'settings page', 'General'],
    ['/settings/general', 'settings page', 'General'],
    ['/settings/labels', 'labels page', 'Labels'],
    ['/settings/debug', 'debug page', 'Debug'],
  ])('%s renders the right page and marks its tab', (path, page, tab) => {
    renderAt(path);
    expect(screen.getByText(page)).toBeInTheDocument();
    const nav = screen.getByRole('navigation', { name: 'Settings sections' });
    expect(within(nav).getByRole('link', { name: tab })).toHaveAttribute('aria-current', 'page');
  });

  it('redirects an unknown section to /settings', () => {
    renderAt('/settings/bogus');
    expect(screen.getByText('settings page')).toBeInTheDocument();
    expect(screen.getByRole('status', { name: 'location' })).toHaveTextContent('/settings');
  });

  it('has one h1 and switches section from its nav', async () => {
    const user = userEvent.setup();
    renderAt('/settings');
    expect(screen.getAllByRole('heading', { level: 1 })).toHaveLength(1);
    await user.click(screen.getByRole('link', { name: 'Debug' }));
    expect(screen.getByText('debug page')).toBeInTheDocument();
  });
});
