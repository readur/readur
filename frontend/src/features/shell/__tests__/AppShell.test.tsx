import { act, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import i18n from 'i18next';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import api, { documentService, labelService } from '../../../services/api';
import { installStorage, renderShell, setMedia } from './shellTestUtils';

vi.mock('../../../services/api', () => ({
  default: { get: vi.fn() },
  api: { defaults: { headers: { common: {} } } },
  documentService: { enhancedSearch: vi.fn() },
  labelService: { list: vi.fn() },
}));

vi.mock('../../document/drawer/DocumentDrawer', () => ({
  default: ({ id, isOpen }: { id: string; isOpen: boolean }) =>
    isOpen ? <div role="dialog" aria-label={`Document ${id}`} /> : null,
}));

const mockedGet = vi.mocked(api.get);
const mockedLabels = vi.mocked(labelService.list);
const sourcesCalls = () => mockedGet.mock.calls.filter(([url]) => url === '/sources').length;
const mockedSearch = vi.mocked(documentService.enhancedSearch);

const location = () => screen.getByRole('status', { name: 'location' });

beforeEach(() => {
  setMedia();
  installStorage();
  document.documentElement.removeAttribute('data-theme');
  // Unresolved by default so the sync readout doesn't update outside act() in unrelated tests.
  mockedGet.mockReturnValue(new Promise(() => {}) as never);
  mockedSearch.mockResolvedValue({ data: { documents: [], total: 0 } } as never);
  mockedLabels.mockReturnValue(new Promise(() => {}) as never);
});

afterEach(() => {
  vi.useRealTimers();
});

describe('document drawer', () => {
  it('opens over whatever page names a document in ?document=', async () => {
    renderShell({ path: '/home?document=d1', page: <p>Home page</p> });
    expect(await screen.findByRole('dialog', { name: 'Document d1' })).toBeInTheDocument();
    expect(screen.getByText('Home page')).toBeInTheDocument();
  });

  it('shows no drawer without ?document=', async () => {
    renderShell({ path: '/home', page: <p>Home page</p> });
    await screen.findByText('Home page');
    expect(screen.queryByRole('dialog', { name: /Document/ })).not.toBeInTheDocument();
  });
});

describe('AppShell layout', () => {
  it('has header, main and navigation landmarks and a skip link to main', () => {
    renderShell();
    expect(screen.getByRole('banner')).toBeInTheDocument();
    const main = screen.getByRole('main');
    expect(main).toHaveAttribute('id', 'main');
    expect(screen.getByRole('navigation', { name: 'Main' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Skip to content' })).toHaveAttribute('href', '#main');
  });

  it('renders the wordmark as a link home', () => {
    renderShell({ path: '/settings' });
    expect(screen.getByRole('link', { name: 'Readur home' })).toHaveAttribute('href', '/home');
  });

  it('puts the destinations, collections and sources in the sidebar', () => {
    renderShell();
    const banner = screen.getByRole('banner');
    expect(within(banner).getByRole('navigation', { name: 'Main' })).toBeInTheDocument();
    expect(within(banner).getByRole('navigation', { name: 'Collections' })).toBeInTheDocument();
    expect(within(banner).getByRole('navigation', { name: 'Sources' })).toBeInTheDocument();
  });

  it('renders the matched page inside main', () => {
    renderShell({ path: '/documents', page: <p>Page body</p> });
    expect(within(screen.getByRole('main')).getByText('Page body')).toBeInTheDocument();
  });
});

describe('primary navigation', () => {
  it('shows the six destinations in order', () => {
    renderShell();
    const nav = screen.getByRole('navigation', { name: 'Main' });
    const links = within(nav).getAllByRole('link');
    expect(links.map((l) => l.textContent)).toEqual(['Home', 'Search', 'Library', 'Intake', 'Sources', 'Settings']);
    expect(links.map((l) => l.getAttribute('href'))).toEqual(['/home', '/search', '/documents', '/intake', '/sources', '/settings']);
  });

  it.each([
    ['/home', 'Home'],
    ['/documents', 'Library'],
    ['/documents/abc', 'Library'],
    ['/search', 'Search'],
    ['/intake', 'Intake'],
    ['/settings/labels', 'Settings'],
  ])('marks the current destination for %s', (path, name) => {
    renderShell({ path });
    const nav = screen.getByRole('navigation', { name: 'Main' });
    const current = within(nav)
      .getAllByRole('link')
      .filter((l) => l.getAttribute('aria-current') === 'page');
    expect(current).toHaveLength(1);
    expect(current[0]).toHaveTextContent(name);
  });

  it('moves aria-current when navigating', async () => {
    const user = userEvent.setup();
    renderShell({ path: '/home' });
    const nav = screen.getByRole('navigation', { name: 'Main' });
    await user.click(within(nav).getByRole('link', { name: 'Intake' }));
    expect(location()).toHaveTextContent('/intake');
    expect(within(nav).getByRole('link', { name: 'Intake' })).toHaveAttribute('aria-current', 'page');
    expect(within(nav).getByRole('link', { name: 'Home' })).not.toHaveAttribute('aria-current');
  });
});

describe('command palette', () => {
  it('opens with Meta+K and Ctrl+K', async () => {
    const user = userEvent.setup();
    renderShell();
    expect(screen.queryByRole('dialog', { name: 'Command palette' })).not.toBeInTheDocument();
    await user.keyboard('{Meta>}k{/Meta}');
    expect(await screen.findByRole('dialog', { name: 'Command palette' })).toBeInTheDocument();
    await user.keyboard('{Escape}');
    await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Command palette' })).not.toBeInTheDocument());
    await user.keyboard('{Control>}k{/Control}');
    expect(await screen.findByRole('dialog', { name: 'Command palette' })).toBeInTheDocument();
  });

  it('opens from the search trigger, which shows a shortcut hint', async () => {
    const user = userEvent.setup();
    renderShell();
    const trigger = screen.getByRole('button', { name: 'Search documents' });
    expect(trigger).toHaveAttribute('aria-keyshortcuts', 'Meta+K Control+K');
    expect(within(trigger).getByText(/K$/)).toBeInTheDocument();
    await user.click(trigger);
    expect(await screen.findByRole('dialog', { name: 'Command palette' })).toBeInTheDocument();
  });

  it('lists destinations on open and navigates to the chosen one', async () => {
    const user = userEvent.setup();
    renderShell({ path: '/home' });
    await user.click(screen.getByRole('button', { name: 'Search documents' }));
    const intake = await screen.findByRole('menuitem', { name: 'Intake' });
    await user.click(intake);
    expect(location()).toHaveTextContent('/intake');
    await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Command palette' })).not.toBeInTheDocument());
  });

  it('offers intake and settings sections', async () => {
    const user = userEvent.setup();
    renderShell();
    await user.click(screen.getByRole('button', { name: 'Search documents' }));
    await user.keyboard('attention');
    await user.click(await screen.findByRole('menuitem', { name: /Needs attention/ }));
    expect(location()).toHaveTextContent('/intake?section=attention');

    await user.click(screen.getByRole('button', { name: 'Search documents' }));
    await user.keyboard('labels');
    await user.click(await screen.findByRole('menuitem', { name: /^Labels/ }));
    expect(location()).toHaveTextContent('/settings/labels');
  });

  it('searches documents with a limit of 8 and opens the chosen one', async () => {
    mockedSearch.mockResolvedValue({
      data: {
        documents: [
          { id: 'd1', filename: 'inv.pdf', original_filename: 'Invoice March.pdf', snippets: [{ text: 'total due' }] },
        ],
        total: 1,
      },
    } as never);
    const storage = installStorage();
    const user = userEvent.setup();
    renderShell();
    await user.click(screen.getByRole('button', { name: 'Search documents' }));
    await user.keyboard('invoice');
    const doc = await screen.findByRole('menuitem', { name: /Invoice March\.pdf/ });
    expect(mockedSearch).toHaveBeenCalledWith(expect.objectContaining({ query: 'invoice', limit: 8 }));
    await user.click(doc);
    expect(location()).toHaveTextContent('/home?document=d1');
    expect(JSON.parse(storage.getItem('recentSearches') ?? '[]')).toEqual(['invoice']);
  });

  it('offers the full results page even when document search fails', async () => {
    mockedSearch.mockRejectedValue(new Error('down'));
    const user = userEvent.setup();
    renderShell();
    await user.click(screen.getByRole('button', { name: 'Search documents' }));
    await user.keyboard('tax');
    await user.click(await screen.findByRole('menuitem', { name: /Show all results for “tax”/ }));
    expect(location()).toHaveTextContent('/search?q=tax');
  });

  it('shows recent searches from the shared storage key', async () => {
    const storage = installStorage();
    storage.setItem('recentSearches', JSON.stringify(['contract', 'receipt']));
    const user = userEvent.setup();
    renderShell();
    await user.click(screen.getByRole('button', { name: 'Search documents' }));
    const recent = await screen.findByRole('menuitem', { name: 'receipt' });
    expect(mockedSearch).not.toHaveBeenCalled();
    await user.click(recent);
    expect(location()).toHaveTextContent('/search?q=receipt');
    expect(JSON.parse(storage.getItem('recentSearches') ?? '[]')).toEqual(['receipt', 'contract']);
  });
});

describe('theme toggle', () => {
  it('flips data-theme and names the target mode', async () => {
    const user = userEvent.setup();
    renderShell();
    expect(document.documentElement.dataset.theme).toBe('light');
    await user.click(screen.getByRole('button', { name: 'Switch to dark mode' }));
    expect(document.documentElement.dataset.theme).toBe('dark');
    await user.click(screen.getByRole('button', { name: 'Switch to light mode' }));
    expect(document.documentElement.dataset.theme).toBe('light');
  });
});

describe('language menu', () => {
  it('lists the supported languages with the current one checked, and switches language', async () => {
    const change = vi.spyOn(i18n, 'changeLanguage').mockResolvedValue((() => '') as never);
    const user = userEvent.setup();
    renderShell();
    await user.click(screen.getByRole('button', { name: 'Change language' }));
    const menu = await screen.findByRole('menu');
    const items = within(menu).getAllByRole('menuitemradio');
    expect(items.map((i) => i.textContent)).toEqual(['English', 'Español', 'Deutsch', 'Français']);
    expect(within(menu).getByRole('menuitemradio', { name: 'English' })).toHaveAttribute('aria-checked', 'true');
    await user.click(within(menu).getByRole('menuitemradio', { name: 'Deutsch' }));
    expect(change).toHaveBeenCalledWith('de');
    change.mockRestore();
  });
});

describe('user menu', () => {
  it('shows who is signed in and logs out', async () => {
    const user = userEvent.setup();
    const { auth } = renderShell({ path: '/documents' });
    await user.click(screen.getByRole('button', { name: /^Account menu/ }));
    const menu = await screen.findByRole('menu');
    expect(within(menu).getByText('ada')).toBeInTheDocument();
    expect(within(menu).getByText('ada@example.com')).toBeInTheDocument();
    await user.click(within(menu).getByRole('menuitem', { name: 'Log out' }));
    expect(auth.logout).toHaveBeenCalledTimes(1);
    expect(location()).toHaveTextContent('/login');
  });

  it('opens settings and the API docs in a new tab', async () => {
    const open = vi.spyOn(window, 'open').mockReturnValue(null);
    const user = userEvent.setup();
    renderShell({ path: '/home' });
    await user.click(screen.getByRole('button', { name: /^Account menu/ }));
    await user.click(await screen.findByRole('menuitem', { name: 'API documentation' }));
    expect(open).toHaveBeenCalledWith('/swagger-ui', '_blank', 'noopener,noreferrer');

    await user.click(screen.getByRole('button', { name: /^Account menu/ }));
    await user.click(await screen.findByRole('menuitem', { name: 'Settings' }));
    expect(location()).toHaveTextContent('/settings');
    open.mockRestore();
  });
});

describe('synced readout', () => {
  it('shows the most recent sync across sources', async () => {
    const now = Date.now();
    mockedGet.mockResolvedValue({
      data: [
        { id: 'a', last_sync_at: new Date(now - 5 * 60_000).toISOString() },
        { id: 'b', last_sync_at: new Date(now - 2 * 60_000 - 5_000).toISOString() },
        { id: 'c', last_sync_at: null },
      ],
    } as never);
    renderShell();
    expect(await screen.findByText('synced 2 min. ago')).toBeInTheDocument();
    expect(mockedGet).toHaveBeenCalledWith('/sources');
    expect(sourcesCalls()).toBe(1);
  });

  it('renders nothing when there are no sources', async () => {
    mockedGet.mockResolvedValue({ data: [] } as never);
    renderShell();
    await waitFor(() => expect(mockedGet).toHaveBeenCalledWith('/sources'));
    await act(async () => {});
    expect(screen.queryByText(/synced/)).not.toBeInTheDocument();
  });

  it('polls the sources list once a minute', async () => {
    mockedGet.mockResolvedValue({ data: [] } as never);
    vi.useFakeTimers({ shouldAdvanceTime: true });
    renderShell();
    await act(async () => {});
    expect(sourcesCalls()).toBe(1);
    await act(async () => {
      vi.advanceTimersByTime(60_000);
    });
    expect(sourcesCalls()).toBe(2);
  });
});
