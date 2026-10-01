import { act, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import api, { labelService } from '../../../services/api';
import { LABELS_CHANGED_EVENT, topCollections } from '../useCollections';
import { SOURCES_CHANGED_EVENT } from '../useLastSynced';
import { sourceHealth } from '../SidebarLists';
import { NARROW, installStorage, renderShell, setMedia } from './shellTestUtils';

vi.mock('../../../services/api', () => ({
  default: { get: vi.fn() },
  api: { defaults: { headers: { common: {} } } },
  documentService: { enhancedSearch: vi.fn() },
  labelService: { list: vi.fn() },
}));

const mockedGet = vi.mocked(api.get);
const mockedLabels = vi.mocked(labelService.list);
const location = () => screen.getByRole('status', { name: 'location' });

const label = (id: string, name: string, document_count: number, color = '#4878B8') => ({
  id,
  name,
  color,
  document_count,
  user_id: null,
  description: null,
  background_color: null,
  icon: null,
  is_system: false,
  created_at: '',
  updated_at: '',
  source_count: 0,
});

const source = (id: string, name: string, extra: Record<string, unknown> = {}) => ({
  id,
  name,
  source_type: 'webdav',
  enabled: true,
  status: 'idle',
  last_sync_at: null,
  ...extra,
});

beforeEach(() => {
  setMedia();
  installStorage();
  mockedGet.mockReset();
  mockedLabels.mockReset();
  mockedGet.mockResolvedValue({ data: [] } as never);
  mockedLabels.mockResolvedValue({ data: [] } as never);
});

const collections = () => screen.getByRole('navigation', { name: 'Collections' });
const sources = () => screen.getByRole('navigation', { name: 'Sources' });

describe('collections', () => {
  it('asks for counts and lists the most-used collections with their counts', async () => {
    mockedLabels.mockResolvedValue({
      data: [label('a', 'Receipts', 3), label('b', 'Medical', 12), label('c', 'Taxes', 7)],
    } as never);
    renderShell();
    const link = await within(collections()).findByRole('link', { name: /Medical/ });
    expect(mockedLabels).toHaveBeenCalledWith(true);
    expect(link).toHaveAttribute('href', '/documents?label=b');
    expect(link).toHaveTextContent('12');
    const names = within(collections())
      .getAllByRole('link')
      .map((l) => l.textContent);
    expect(names).toEqual(['Medical12', 'Taxes7', 'Receipts3', 'All collections']);
  });

  it('links to label management and says when there are none', async () => {
    renderShell();
    expect(await within(collections()).findByText('No collections yet')).toBeInTheDocument();
    expect(within(collections()).getByRole('link', { name: 'All collections' })).toHaveAttribute(
      'href',
      '/settings/labels',
    );
  });

  it('marks the collection being viewed', async () => {
    mockedLabels.mockResolvedValue({ data: [label('a', 'Receipts', 3), label('b', 'Medical', 1)] } as never);
    renderShell({ path: '/documents?label=b' });
    expect(await within(collections()).findByRole('link', { name: /Medical/ })).toHaveAttribute('aria-current', 'page');
    expect(within(collections()).getByRole('link', { name: /Receipts/ })).not.toHaveAttribute('aria-current');
  });

  it('reloads when a screen announces a label change', async () => {
    renderShell();
    await within(collections()).findByText('No collections yet');
    mockedLabels.mockResolvedValue({ data: [label('n', 'Shoulder injury', 9)] } as never);
    await act(async () => {
      window.dispatchEvent(new CustomEvent(LABELS_CHANGED_EVENT));
    });
    expect(await within(collections()).findByRole('link', { name: /Shoulder injury/ })).toBeInTheDocument();
    expect(mockedLabels).toHaveBeenCalledTimes(2);
  });

  it('keeps the list when a reload fails', async () => {
    mockedLabels.mockResolvedValueOnce({ data: [label('a', 'Receipts', 3)] } as never);
    renderShell();
    await within(collections()).findByRole('link', { name: /Receipts/ });
    mockedLabels.mockRejectedValueOnce(new Error('down'));
    await act(async () => {
      window.dispatchEvent(new CustomEvent(LABELS_CHANGED_EVENT));
    });
    expect(within(collections()).getByRole('link', { name: /Receipts/ })).toBeInTheDocument();
  });

  it('caps the list at eight, most documents first then by name', () => {
    const many = Array.from({ length: 11 }, (_, i) => label(String(i), `L${i}`, i % 3));
    const top = topCollections(many);
    expect(top).toHaveLength(8);
    expect(top[0].document_count).toBe(2);
    expect(top.slice(0, 4).map((l) => l.name)).toEqual(['L2', 'L5', 'L8', 'L1']);
  });
});

describe('sources', () => {
  it('lists uploads, the watch folder and each source by name with a health word', async () => {
    mockedGet.mockResolvedValue({
      data: [
        source('s2', 'Scanner inbox', { source_type: 'local_folder', validation_status: 'warning' }),
        source('s1', 'Nextcloud', { status: 'error' }),
        source('s3', 'Archive bucket', { source_type: 's3', enabled: false }),
      ],
    } as never);
    renderShell();
    await within(sources()).findByRole('link', { name: /Nextcloud/ });
    const links = within(sources()).getAllByRole('link');
    expect(links.map((l) => l.textContent)).toEqual([
      'Uploads',
      'Watch folder',
      'Archive bucketOff',
      'NextcloudError',
      'Scanner inboxCheck',
    ]);
    expect(links[0]).toHaveAttribute('href', '/intake?section=upload');
    expect(links[1]).toHaveAttribute('href', '/intake?section=watch');
    expect(links[3]).toHaveAttribute('href', '/intake?section=connections&source=s1');
  });

  it('marks the open source and section', async () => {
    mockedGet.mockResolvedValue({ data: [source('s1', 'Nextcloud')] } as never);
    renderShell({ path: '/intake?section=connections&source=s1' });
    expect(await within(sources()).findByRole('link', { name: /Nextcloud/ })).toHaveAttribute('aria-current', 'page');
    expect(within(sources()).getByRole('link', { name: 'Uploads' })).not.toHaveAttribute('aria-current');
  });

  it('marks the watch folder when that section is open', async () => {
    renderShell({ path: '/intake?section=watch' });
    await waitFor(() =>
      expect(within(sources()).getByRole('link', { name: 'Watch folder' })).toHaveAttribute('aria-current', 'page'),
    );
  });

  it('reloads when a screen announces a source change', async () => {
    renderShell();
    await waitFor(() => expect(mockedGet).toHaveBeenCalledTimes(1));
    mockedGet.mockResolvedValue({ data: [source('s9', 'New share')] } as never);
    await act(async () => {
      window.dispatchEvent(new CustomEvent(SOURCES_CHANGED_EVENT));
    });
    expect(await within(sources()).findByRole('link', { name: /New share/ })).toBeInTheDocument();
  });

  it('names health only when something is off', () => {
    expect(sourceHealth({ enabled: true, status: 'idle' })).toBeNull();
    expect(sourceHealth({ enabled: true, status: 'syncing' })).toBe('syncing');
    expect(sourceHealth({ enabled: true, status: 'error' })).toBe('error');
    expect(sourceHealth({ enabled: false, status: 'error' })).toBe('off');
    expect(sourceHealth({ enabled: true, status: 'idle', validation_status: 'warning' })).toBe('check');
    expect(sourceHealth({ enabled: true, status: 'idle', validation_status: 'critical' })).toBe('error');
    expect(sourceHealth({ enabled: true, status: 'syncing', validation_status: 'critical' })).toBe('syncing');
  });
});

describe('phone drawer', () => {
  beforeEach(() => setMedia(NARROW));

  it('hides the sidebar behind a menu button and opens it as a dialog', async () => {
    const user = userEvent.setup();
    renderShell();
    expect(screen.queryByRole('navigation', { name: 'Collections' })).not.toBeInTheDocument();
    const open = screen.getByRole('button', { name: 'Open menu' });
    expect(within(screen.getByRole('banner')).getByRole('link', { name: 'Readur home' })).toBeInTheDocument();
    await user.click(open);
    const drawer = await screen.findByRole('dialog', { name: 'Menu' });
    expect(within(drawer).getByRole('navigation', { name: 'Collections' })).toBeInTheDocument();
    expect(within(drawer).getByRole('button', { name: 'Switch to dark mode' })).toBeInTheDocument();
  });

  it('closes after following a link', async () => {
    const user = userEvent.setup();
    renderShell();
    await user.click(screen.getByRole('button', { name: 'Open menu' }));
    const drawer = await screen.findByRole('dialog', { name: 'Menu' });
    await user.click(within(drawer).getByRole('link', { name: 'Settings' }));
    expect(location()).toHaveTextContent('/settings');
    await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Menu' })).not.toBeInTheDocument());
  });

  it('closes with the close button', async () => {
    const user = userEvent.setup();
    renderShell();
    await user.click(screen.getByRole('button', { name: 'Open menu' }));
    await user.click(await screen.findByRole('button', { name: 'Close menu' }));
    await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Menu' })).not.toBeInTheDocument());
  });
});
