import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import SettingsPage from '../SettingsPage';
import { plainUser, REDUCED_MOTION, renderSettings } from './settingsTestUtils';

vi.mock('../general/GeneralSection', () => ({ default: () => <p>general body</p> }));
vi.mock('../ocr/OcrSection', () => ({ default: () => <p>ocr body</p> }));
vi.mock('../users/UsersSection', () => ({ default: () => <p>users body</p> }));
vi.mock('../server/ServerSection', () => ({ default: () => <p>server body</p> }));
vi.mock('../apiKeys/ApiKeysSection', () => ({ default: () => <p>api keys body</p> }));
vi.mock('../labels/LabelsSection', () => ({ default: () => <p>labels body</p> }));
vi.mock('../debug/DebugSection', () => ({ default: () => <p>debug body</p> }));
vi.mock('../appearance/AppearanceSection', () => ({ default: () => <p>appearance body</p> }));

const nav = () => screen.getByRole('navigation', { name: 'Settings sections' });


describe('SettingsPage routing', () => {
  it.each([
    ['/settings', 'general body', 'General'],
    ['/settings/general', 'general body', 'General'],
    ['/settings/ocr', 'ocr body', 'OCR'],
    ['/settings/users', 'users body', 'Users'],
    ['/settings/server', 'server body', 'Server'],
    ['/settings/api-keys', 'api keys body', 'API keys'],
    ['/settings/labels', 'labels body', 'Labels'],
    ['/settings/debug', 'debug body', 'Debug'],
    ['/settings/appearance', 'appearance body', 'Appearance'],
  ])('%s renders its section and marks its link current', async (path, body, link) => {
    renderSettings(<SettingsPage />, { path });
    expect(await screen.findByText(body)).toBeInTheDocument();
    expect(within(nav()).getByRole('link', { name: link })).toHaveAttribute('aria-current', 'page');
    expect(screen.getByRole('heading', { level: 2, name: link })).toBeInTheDocument();
  });

  it('redirects an unknown section to /settings', async () => {
    renderSettings(<SettingsPage />, { path: '/settings/bogus' });
    expect(await screen.findByText('general body')).toBeInTheDocument();
    expect(screen.getByRole('status', { name: 'location' })).toHaveTextContent(/^\/settings$/);
  });

  it('has exactly one h1 and switches section from the nav', async () => {
    const user = userEvent.setup();
    renderSettings(<SettingsPage />);
    expect(screen.getAllByRole('heading', { level: 1 })).toHaveLength(1);
    await user.click(within(nav()).getByRole('link', { name: 'Debug' }));
    expect(await screen.findByText('debug body')).toBeInTheDocument();
    expect(within(nav()).getByRole('link', { name: 'Debug' })).toHaveAttribute('aria-current', 'page');
    expect(within(nav()).getByRole('link', { name: 'General' })).not.toHaveAttribute('aria-current');
  });

  it('marks only one link as current', async () => {
    renderSettings(<SettingsPage />, { path: '/settings/labels' });
    await screen.findByText('labels body');
    const current = within(nav())
      .getAllByRole('link')
      .filter((a) => a.getAttribute('aria-current') === 'page');
    expect(current).toHaveLength(1);
  });
});

describe('SettingsPage admin gating', () => {
  it('lists every section for an admin', async () => {
    renderSettings(<SettingsPage />);
    await screen.findByText('general body');
    const names = within(nav())
      .getAllByRole('link')
      .map((a) => a.textContent);
    expect(names).toEqual(['General', 'OCR', 'Users', 'Server', 'API keys', 'Labels', 'Debug', 'Appearance']);
  });

  it('hides admin-only sections from other users', async () => {
    renderSettings(<SettingsPage />, { user: plainUser });
    await screen.findByText('general body');
    const names = within(nav())
      .getAllByRole('link')
      .map((a) => a.textContent);
    expect(names).toEqual(['General', 'OCR', 'API keys', 'Labels', 'Appearance']);
  });

  it.each(['users', 'server', 'debug'])('shows "Admins only" for /settings/%s to a non-admin', async (id) => {
    renderSettings(<SettingsPage />, { path: `/settings/${id}`, user: plainUser });
    expect(await screen.findByRole('heading', { name: 'Admins only' })).toBeInTheDocument();
    expect(screen.queryByText(`${id} body`)).not.toBeInTheDocument();
  });

  it('treats the role case-insensitively', async () => {
    renderSettings(<SettingsPage />, { path: '/settings/users', user: { ...plainUser, role: 'Admin' } });
    expect(await screen.findByText('users body')).toBeInTheDocument();
  });
});

describe('SettingsPage search', () => {
  it('filters sections by title', async () => {
    const user = userEvent.setup();
    renderSettings(<SettingsPage />);
    await screen.findByText('general body');
    await user.type(screen.getByRole('searchbox', { name: 'Search settings' }), 'appear');
    const links = within(nav()).getAllByRole('link');
    expect(links.map((a) => a.textContent)).toEqual(['Appearance']);
  });

  it('matches individual settings and links to their group', async () => {
    const user = userEvent.setup();
    renderSettings(<SettingsPage />);
    await screen.findByText('general body');
    await user.type(screen.getByRole('searchbox', { name: 'Search settings' }), 'threshold');
    const hit = within(nav()).getByRole('link', { name: 'Brightness Threshold' });
    expect(hit).toHaveAttribute('href', '/settings/ocr#ocr-thresholds');
    expect(within(nav()).getByRole('link', { name: 'Fuzzy Search Threshold' })).toHaveAttribute(
      'href',
      '/settings#search',
    );
    expect(within(nav()).queryByRole('link', { name: 'Appearance' })).not.toBeInTheDocument();
    await user.click(hit);
    expect(await screen.findByText('ocr body')).toBeInTheDocument();
    expect(screen.getByRole('status', { name: 'location' })).toHaveTextContent('/settings/ocr#ocr-thresholds');
  });

  it('is case-insensitive and reports no matches', async () => {
    const user = userEvent.setup();
    renderSettings(<SettingsPage />);
    await screen.findByText('general body');
    const field = screen.getByRole('searchbox', { name: 'Search settings' });
    await user.type(field, 'CPU');
    expect(within(nav()).getByRole('link', { name: 'CPU Priority' })).toBeInTheDocument();
    await user.clear(field);
    await user.type(field, 'zzzz');
    expect(within(nav()).queryAllByRole('link')).toHaveLength(0);
    expect(screen.getByText('No settings match “zzzz”')).toBeInTheDocument();
  });

  it('does not surface admin-only settings to other users', async () => {
    const user = userEvent.setup();
    renderSettings(<SettingsPage />, { user: plainUser });
    await screen.findByText('general body');
    await user.type(screen.getByRole('searchbox', { name: 'Search settings' }), 'version');
    expect(within(nav()).queryByRole('link', { name: 'Version' })).not.toBeInTheDocument();
  });

  it('clears the filter with Escape', async () => {
    const user = userEvent.setup();
    renderSettings(<SettingsPage />);
    await screen.findByText('general body');
    const field = screen.getByRole('searchbox', { name: 'Search settings' });
    await user.type(field, 'appear');
    await user.keyboard('{Escape}');
    expect(within(nav()).getAllByRole('link')).toHaveLength(8);
  });
});

describe('SettingsPage on a narrow screen', () => {
  it('replaces the link list with a section select that navigates', async () => {
    const user = userEvent.setup();
    renderSettings(<SettingsPage />, { media: { ...REDUCED_MOTION, '(max-width: 719px)': true } });
    await screen.findByText('general body');
    expect(within(nav()).queryAllByRole('link')).toHaveLength(0);
    await user.click(screen.getByRole('button', { name: /Section/ }));
    await user.click(await screen.findByRole('option', { name: 'Labels' }));
    expect(await screen.findByText('labels body')).toBeInTheDocument();
  });
});
