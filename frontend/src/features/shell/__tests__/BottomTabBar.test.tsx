import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import api from '../../../services/api';
import { resetPWAMocks, setupIOSPWAMode } from '../../../test/pwa-test-utils';
import { NARROW, STANDALONE, installStorage, renderShell, setMedia } from './shellTestUtils';

vi.mock('../../../services/api', () => ({
  default: { get: vi.fn() },
  api: { defaults: { headers: { common: {} } } },
  documentService: { enhancedSearch: vi.fn() },
}));

const location = () => screen.getByRole('status', { name: 'location' });
const DESTINATIONS = [
  ['Board', '/board'],
  ['Library', '/documents'],
  ['Intake', '/intake'],
  ['Settings', '/settings'],
] as const;

beforeEach(() => {
  resetPWAMocks();
  installStorage();
  vi.mocked(api.get).mockReturnValue(new Promise(() => {}) as never);
});

describe('bottom tab bar visibility', () => {
  it('is not rendered on a wide screen outside the installed app', () => {
    setMedia();
    renderShell();
    expect(screen.queryByRole('navigation', { name: 'Tab bar' })).not.toBeInTheDocument();
    expect(screen.getAllByRole('navigation')).toHaveLength(1);
  });

  it('replaces the top-bar nav on a narrow screen', () => {
    setMedia(NARROW);
    renderShell();
    const navs = screen.getAllByRole('navigation');
    expect(navs).toHaveLength(1);
    expect(within(screen.getByRole('banner')).queryByRole('navigation')).not.toBeInTheDocument();
    expect(screen.getByRole('navigation', { name: 'Main' })).toBeInTheDocument();
  });

  it('is rendered in installed-app (standalone) mode', () => {
    setMedia(STANDALONE);
    renderShell();
    expect(screen.getByRole('navigation', { name: 'Tab bar' })).toBeInTheDocument();
  });

  it('is rendered for an iOS home-screen app', () => {
    setMedia();
    setupIOSPWAMode(true);
    renderShell();
    expect(screen.getByRole('navigation', { name: 'Tab bar' })).toBeInTheDocument();
  });

  it('sits outside the header and main landmarks', () => {
    setMedia(NARROW);
    renderShell();
    const bar = screen.getByRole('navigation', { name: 'Main' });
    expect(screen.getByRole('banner')).not.toContainElement(bar);
    expect(screen.getByRole('main')).not.toContainElement(bar);
  });
});

describe('bottom tab bar items', () => {
  beforeEach(() => setMedia(NARROW));

  it('renders the four destinations with visible text labels', () => {
    renderShell();
    const bar = screen.getByRole('navigation', { name: 'Main' });
    const links = within(bar).getAllByRole('link');
    expect(links).toHaveLength(4);
    DESTINATIONS.forEach(([name, href], i) => {
      expect(links[i]).toHaveAccessibleName(name);
      expect(within(links[i]).getByText(name)).toBeVisible();
      expect(links[i]).toHaveAttribute('href', href);
    });
  });

  it.each(DESTINATIONS)('navigates to %s when pressed', async (name, href) => {
    const user = userEvent.setup();
    renderShell({ path: name === 'Board' ? '/settings' : '/board' });
    const bar = screen.getByRole('navigation', { name: 'Main' });
    await user.click(within(bar).getByRole('link', { name }));
    expect(location()).toHaveTextContent(href);
    expect(within(bar).getByRole('link', { name })).toHaveAttribute('aria-current', 'page');
  });

  it('marks the item for the current location', () => {
    renderShell({ path: '/intake?section=watch' });
    const bar = screen.getByRole('navigation', { name: 'Main' });
    expect(within(bar).getByRole('link', { name: 'Intake' })).toHaveAttribute('aria-current', 'page');
    ['Board', 'Library', 'Settings'].forEach((name) =>
      expect(within(bar).getByRole('link', { name })).not.toHaveAttribute('aria-current'),
    );
  });

  it('reaches every item with the keyboard', async () => {
    const user = userEvent.setup();
    renderShell();
    const bar = screen.getByRole('navigation', { name: 'Main' });
    const links = within(bar).getAllByRole('link');
    links[0].focus();
    for (const link of links.slice(1)) {
      await user.tab();
      expect(link).toHaveFocus();
    }
  });

  it('opens a link with Enter', async () => {
    const user = userEvent.setup();
    renderShell({ path: '/board' });
    within(screen.getByRole('navigation', { name: 'Main' }))
      .getByRole('link', { name: 'Library' })
      .focus();
    await user.keyboard('{Enter}');
    expect(location()).toHaveTextContent('/documents');
  });

  it('keeps the same four items across re-renders', () => {
    const { rerenderShell } = renderShell();
    expect(within(screen.getByRole('navigation', { name: 'Main' })).getAllByRole('link')).toHaveLength(4);
    rerenderShell();
    expect(within(screen.getByRole('navigation', { name: 'Main' })).getAllByRole('link')).toHaveLength(4);
  });

  it('turns the search field into an icon button that still opens the palette', async () => {
    const user = userEvent.setup();
    renderShell();
    const search = screen.getByRole('button', { name: 'Search documents' });
    expect(search).not.toHaveTextContent('Search documents…');
    await user.click(search);
    expect(await screen.findByRole('dialog', { name: 'Command palette' })).toBeInTheDocument();
  });
});
