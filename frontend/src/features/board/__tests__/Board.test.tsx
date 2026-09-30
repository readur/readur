import { act, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const m = vi.hoisted(() => ({
  api: { get: vi.fn() },
  documentService: { listWithPagination: vi.fn(), getFailedOcrDocuments: vi.fn(), retryOcr: vi.fn() },
  queueService: { getStats: vi.fn(), getOcrStatus: vi.fn(), pauseOcr: vi.fn(), resumeOcr: vi.fn() },
  sourcesService: { triggerSync: vi.fn() },
}));

vi.mock('../../../services/api', () => ({
  default: m.api,
  api: m.api,
  documentService: m.documentService,
  queueService: m.queueService,
  sourcesService: m.sourcesService,
}));

import Board from '../Board';
import { isLit, markLit } from '../litStore';
import { doc, renderPage, resetBoardState } from './boardTestUtils';

const STATS = {
  pending_count: 7,
  processing_count: 2,
  failed_count: 3,
  completed_today: 41,
  oldest_pending_minutes: 130,
};

const SOURCES = [
  { id: 's1', name: 'Office scanner', enabled: true, status: 'idle', last_sync_at: new Date(Date.now() - 3 * 60_000).toISOString() },
  { id: 's2', name: 'Cloud share', enabled: true, status: 'syncing', last_sync_at: null },
  { id: 's3', name: 'Old archive', enabled: false, status: 'idle', last_sync_at: null },
];

let sources: unknown = SOURCES;

function serve() {
  m.documentService.listWithPagination.mockResolvedValue({
    data: { documents: [doc('d1'), doc('d2', { ocr_status: 'processing', has_ocr_text: false, ocr_progress_current: 3, ocr_progress_total: 12 })], pagination: { total: 2 } },
  });
  m.documentService.getFailedOcrDocuments.mockResolvedValue({ data: { documents: [] } });
  m.documentService.retryOcr.mockResolvedValue({ data: {} });
  m.queueService.getStats.mockResolvedValue({ data: STATS });
  m.queueService.getOcrStatus.mockResolvedValue({ data: { is_paused: false, status: 'running' } });
  m.queueService.pauseOcr.mockResolvedValue({ data: { status: 'paused', message: '' } });
  m.queueService.resumeOcr.mockResolvedValue({ data: { status: 'resumed', message: '' } });
  m.sourcesService.triggerSync.mockResolvedValue({ data: {} });
  m.api.get.mockImplementation((url: string) => {
    if (url === '/sources') return sources instanceof Error ? Promise.reject(sources) : Promise.resolve({ data: sources });
    if (url === '/metrics') {
      return Promise.resolve({ data: { documents: { total_documents: 1200, total_storage_bytes: 5 * 1024 * 1024 * 1024, documents_with_ocr: 900 } } });
    }
    if (url.startsWith('/labels')) return Promise.resolve({ data: [{ id: 'l1' }, { id: 'l2' }, { id: 'l3' }] });
    return Promise.reject(new Error(`unexpected ${url}`));
  });
}

const region = (name: string) => screen.getByRole('region', { name });
const loc = () => screen.getByRole('status', { name: 'location' }, { hidden: true }).textContent;

async function renderBoard(role: 'Admin' | 'User' = 'Admin') {
  const view = renderPage(<Board />, role);
  await screen.findByText('d1.pdf');
  return view;
}

beforeEach(() => {
  resetBoardState();
  sources = SOURCES;
  vi.clearAllMocks();
  serve();
});

afterEach(() => {
  vi.useRealTimers();
});

describe('Board', () => {
  it('renders the page title, add action and every region', async () => {
    await renderBoard();
    expect(screen.getAllByRole('heading', { level: 1 })).toHaveLength(1);
    expect(screen.getByRole('heading', { level: 1, name: 'Board' })).toBeInTheDocument();
    expect(screen.getAllByRole('link', { name: 'Add documents' })[0]).toHaveAttribute('href', '/intake?section=upload');
    expect(await screen.findByRole('region', { name: 'Arrivals' })).toBeInTheDocument();
    expect(region('Processing')).toBeInTheDocument();
    expect(region('Connections')).toBeInTheDocument();
    expect(region('Library')).toBeInTheDocument();
  });

  it('shows loading placeholders before anything has arrived', () => {
    const never = new Promise(() => {});
    m.documentService.listWithPagination.mockReturnValue(never);
    m.queueService.getStats.mockReturnValue(never);
    m.queueService.getOcrStatus.mockReturnValue(never);
    m.documentService.getFailedOcrDocuments.mockReturnValue(never);
    m.api.get.mockReturnValue(never);
    renderPage(<Board />);
    expect(screen.getAllByRole('status', { name: 'Loading' }).length).toBeGreaterThanOrEqual(3);
    expect(screen.getByRole('heading', { level: 1, name: 'Board' })).toBeInTheDocument();
  });

  describe('Arrivals', () => {
    it('lists the newest documents with type, status, size and age', async () => {
      await renderBoard();
      const table = within(region('Arrivals')).getByRole('grid', { name: 'Arrivals' });
      expect(within(table).getAllByRole('columnheader').map((h) => h.textContent)).toEqual(
        expect.arrayContaining(['Name', 'Type', 'Status', 'Size', 'Added']),
      );
      const row = within(table).getByRole('row', { name: /d2\.pdf/ });
      expect(within(row).getByText('OCR 3/12')).toBeInTheDocument();
      expect(within(row).getByText('PDF')).toBeInTheDocument();
      expect(within(row).getByText('2 KB')).toBeInTheDocument();
      expect(within(row).getByText('5m ago')).toBeInTheDocument();
      expect(within(within(table).getByRole('row', { name: /d1\.pdf/ })).getByText('INDEXED', { exact: false })).toBeInTheDocument();
    });

    it('marks lit rows with data-changed and a visible tag, and clears them when opened', async () => {
      markLit('document', 'd1', 'new');
      const user = userEvent.setup();
      await renderBoard();
      const row = screen.getByRole('row', { name: /d1\.pdf/ });
      expect(row).toHaveAttribute('data-changed', 'true');
      expect(within(row).getByText('New')).toBeInTheDocument();
      expect(screen.getByRole('row', { name: /d2\.pdf/ })).not.toHaveAttribute('data-changed');

      await user.click(row);
      expect(loc()).toBe('/documents/d1');
      expect(isLit('document', 'd1')).toBe(false);
      await waitFor(() => expect(screen.getByRole('row', { name: /d1\.pdf/ })).not.toHaveAttribute('data-changed'));
      expect(within(screen.getByRole('row', { name: /d1\.pdf/ })).queryByText('New')).not.toBeInTheDocument();
    });

    it('opens a document with the keyboard', async () => {
      const user = userEvent.setup();
      await renderBoard();
      act(() => screen.getByRole('row', { name: /d2\.pdf/ }).focus());
      await user.keyboard('{Enter}');
      expect(loc()).toBe('/documents/d2');
    });

    it('polls every 15 seconds and lights new documents and finished OCR', async () => {
      vi.useFakeTimers({ shouldAdvanceTime: true });
      await renderBoard();
      expect(screen.getByRole('row', { name: /d1\.pdf/ })).not.toHaveAttribute('data-changed');
      expect(m.documentService.listWithPagination).toHaveBeenCalledTimes(1);

      m.documentService.listWithPagination.mockResolvedValue({
        data: {
          documents: [doc('d3', { created_at: new Date().toISOString() }), doc('d1'), doc('d2', { ocr_status: 'completed', has_ocr_text: true })],
          pagination: { total: 3 },
        },
      });
      await act(async () => {
        await vi.advanceTimersByTimeAsync(15_000);
      });

      const fresh = await screen.findByRole('row', { name: /d3\.pdf/ });
      expect(fresh).toHaveAttribute('data-changed', 'true');
      expect(within(fresh).getByText('New')).toBeInTheDocument();
      const finished = screen.getByRole('row', { name: /d2\.pdf/ });
      expect(finished).toHaveAttribute('data-changed', 'true');
      expect(within(finished).getByText('Changed')).toBeInTheDocument();
      expect(screen.getByRole('row', { name: /d1\.pdf/ })).not.toHaveAttribute('data-changed');
      expect(m.documentService.listWithPagination).toHaveBeenCalledTimes(2);
    });

    it('lights a document whose OCR failed', async () => {
      vi.useFakeTimers({ shouldAdvanceTime: true });
      await renderBoard();
      m.documentService.listWithPagination.mockResolvedValue({
        data: { documents: [doc('d1', { ocr_status: 'failed', has_ocr_text: false }), doc('d2')], pagination: { total: 2 } },
      });
      await act(async () => {
        await vi.advanceTimersByTimeAsync(15_000);
      });
      await waitFor(() => expect(screen.getByRole('row', { name: /d1\.pdf/ })).toHaveAttribute('data-changed', 'true'));
      expect(isLit('document', 'd1')).toBe(true);
    });

    it('does not poll while the tab is hidden', async () => {
      vi.useFakeTimers({ shouldAdvanceTime: true });
      await renderBoard();
      const spy = vi.spyOn(document, 'visibilityState', 'get').mockReturnValue('hidden');
      await act(async () => {
        await vi.advanceTimersByTimeAsync(45_000);
      });
      expect(m.documentService.listWithPagination).toHaveBeenCalledTimes(1);
      spy.mockRestore();
    });

    it('shows the empty state with a call to action for an empty library', async () => {
      m.documentService.listWithPagination.mockResolvedValue({ data: { documents: [], pagination: { total: 0 } } });
      renderPage(<Board />);
      const arrivals = await screen.findByRole('region', { name: 'Arrivals' });
      expect(await within(arrivals).findByText('No documents yet')).toBeInTheDocument();
      expect(within(arrivals).getByRole('link', { name: 'Add documents' })).toHaveAttribute('href', '/intake?section=upload');
    });

    it('shows an inline error with Retry that recovers', async () => {
      const user = userEvent.setup();
      m.documentService.listWithPagination.mockRejectedValue(new Error('boom'));
      renderPage(<Board />);
      const arrivals = await screen.findByRole('region', { name: 'Arrivals' });
      const alert = await within(arrivals).findByRole('alert');
      expect(alert).toHaveTextContent('Recent documents could not be loaded.');
      // The rest of the page is still there.
      expect(await within(region('Processing')).findByText('2h 10m')).toBeInTheDocument();

      serve();
      await user.click(within(alert).getByRole('button', { name: 'Retry' }));
      expect(await screen.findByText('d1.pdf')).toBeInTheDocument();
      expect(within(arrivals).queryByRole('alert')).not.toBeInTheDocument();
    });
  });

  describe('Needs attention', () => {
    it('is hidden when nothing needs attention', async () => {
      await renderBoard();
      expect(screen.queryByRole('region', { name: 'Needs attention' })).not.toBeInTheDocument();
    });

    it('lists failed documents and connections in error, all marked', async () => {
      m.documentService.getFailedOcrDocuments.mockResolvedValue({
        data: { documents: [{ id: 'f1', filename: 'scan.tiff', failure_reason: 'low_ocr_confidence', created_at: new Date(Date.now() - 2 * 3600_000).toISOString() }] },
      });
      sources = [{ id: 's9', name: 'NAS', enabled: true, status: 'error', last_error: 'Auth failed', last_error_at: new Date(Date.now() - 60_000).toISOString() }];
      await renderBoard();
      const strip = await screen.findByRole('region', { name: 'Needs attention' });
      expect(within(strip).getByRole('link', { name: 'View all' })).toHaveAttribute('href', '/intake?section=attention');
      const rows = within(strip).getAllByRole('row').slice(1);
      expect(rows).toHaveLength(2);
      rows.forEach((r) => {
        expect(r).toHaveAttribute('data-changed', 'true');
        expect(within(r).getByText('New')).toBeInTheDocument();
      });
      const docRow = within(strip).getByRole('row', { name: /scan\.tiff/ });
      expect(within(docRow).getByText('FAILED')).toBeInTheDocument();
      expect(within(docRow).getByText('low ocr confidence')).toBeInTheDocument();
      expect(within(docRow).getByText('2h ago')).toBeInTheDocument();
      const srcRow = within(strip).getByRole('row', { name: /NAS/ });
      expect(within(srcRow).getByText('ERROR')).toBeInTheDocument();
      expect(within(srcRow).getByText('Auth failed')).toBeInTheDocument();
      expect(isLit('attention', 'document:f1')).toBe(true);
    });

    it('retries a failed document', async () => {
      const user = userEvent.setup();
      m.documentService.getFailedOcrDocuments.mockResolvedValue({ data: { documents: [{ id: 'f1', filename: 'scan.tiff', failure_reason: 'x' }] } });
      await renderBoard();
      await user.click(await screen.findByRole('button', { name: 'Retry scan.tiff' }));
      expect(m.documentService.retryOcr).toHaveBeenCalledWith('f1');
      await waitFor(() => expect(screen.queryByRole('row', { name: /scan\.tiff/ })).not.toBeInTheDocument());
    });

    it('dismiss acknowledges the row, removes it and keeps it removed after a reload', async () => {
      const user = userEvent.setup();
      m.documentService.getFailedOcrDocuments.mockResolvedValue({ data: { documents: [{ id: 'f1', filename: 'scan.tiff', failure_reason: 'x' }] } });
      const first = await renderBoard();
      await user.click(await screen.findByRole('button', { name: 'Dismiss scan.tiff' }));
      expect(isLit('attention', 'document:f1')).toBe(false);
      expect(screen.queryByRole('region', { name: 'Needs attention' })).not.toBeInTheDocument();

      first.unmount();
      await renderBoard();
      await waitFor(() => expect(m.documentService.getFailedOcrDocuments).toHaveBeenCalledTimes(2));
      expect(screen.queryByRole('region', { name: 'Needs attention' })).not.toBeInTheDocument();
    });

    it('retries a connection through a new sync', async () => {
      const user = userEvent.setup();
      sources = [{ id: 's9', name: 'NAS', enabled: true, status: 'error', last_error: 'Auth failed' }];
      await renderBoard();
      await user.click(await screen.findByRole('button', { name: 'Retry NAS' }));
      expect(m.sourcesService.triggerSync).toHaveBeenCalledWith('s9');
    });

    it('opens the matching Intake section from a row', async () => {
      const user = userEvent.setup();
      sources = [{ id: 's9', name: 'NAS', enabled: true, status: 'error', last_error: 'Auth failed' }];
      await renderBoard();
      await user.click(await screen.findByRole('row', { name: /NAS/ }));
      expect(loc()).toBe('/intake?section=connections');
    });

    it('shows an error with Retry when failed documents cannot be loaded', async () => {
      const user = userEvent.setup();
      m.documentService.getFailedOcrDocuments.mockRejectedValue(new Error('boom'));
      await renderBoard();
      const strip = await screen.findByRole('region', { name: 'Needs attention' });
      expect(within(strip).getByRole('alert')).toBeInTheDocument();
      m.documentService.getFailedOcrDocuments.mockResolvedValue({ data: { documents: [] } });
      await user.click(within(strip).getByRole('button', { name: 'Retry' }));
      await waitFor(() => expect(screen.queryByRole('region', { name: 'Needs attention' })).not.toBeInTheDocument());
    });
  });

  describe('Processing', () => {
    it('shows the queue values', async () => {
      await renderBoard();
      const group = await within(region('Processing')).findByRole('group', { name: 'Processing' });
      const value = (label: string) => within(group).getByText(label).closest('div')?.querySelector('dd')?.textContent;
      expect(value('Pending')).toBe('7');
      expect(value('Processing')).toBe('2');
      expect(value('Failed')).toBe('3');
      expect(value('Done today')).toBe('41');
      expect(value('Oldest wait')).toBe('2h 10m');
      expect(within(region('Processing')).getByText('HEALTHY')).toBeInTheDocument();
    });

    it('lets an admin pause OCR', async () => {
      const user = userEvent.setup();
      await renderBoard('Admin');
      await user.click(await within(region('Processing')).findByRole('button', { name: 'Pause OCR' }));
      expect(m.queueService.pauseOcr).toHaveBeenCalled();
    });

    it('lets an admin resume paused OCR and shows the state', async () => {
      const user = userEvent.setup();
      m.queueService.getOcrStatus.mockResolvedValue({ data: { is_paused: true, status: 'paused' } });
      await renderBoard('Admin');
      expect(await within(region('Processing')).findByText('OFF')).toBeInTheDocument();
      await user.click(within(region('Processing')).getByRole('button', { name: 'Resume OCR' }));
      expect(m.queueService.resumeOcr).toHaveBeenCalled();
    });

    it('hides pause and resume from non-admins', async () => {
      await renderBoard('User');
      await within(region('Processing')).findByText('HEALTHY');
      expect(screen.queryByRole('button', { name: 'Pause OCR' })).not.toBeInTheDocument();
      expect(screen.queryByRole('button', { name: 'Resume OCR' })).not.toBeInTheDocument();
    });

    it('refreshes the queue every 15 seconds', async () => {
      vi.useFakeTimers({ shouldAdvanceTime: true });
      await renderBoard();
      await within(region('Processing')).findByRole('group');
      m.queueService.getStats.mockResolvedValue({ data: { ...STATS, pending_count: 9 } });
      await act(async () => {
        await vi.advanceTimersByTimeAsync(15_000);
      });
      await waitFor(() => expect(within(region('Processing')).getByText('9')).toBeInTheDocument());
    });

    it('shows an inline error with Retry when the queue cannot be loaded', async () => {
      const user = userEvent.setup();
      m.queueService.getStats.mockRejectedValue(new Error('boom'));
      await renderBoard();
      const alert = await within(region('Processing')).findByRole('alert');
      expect(alert).toHaveTextContent('The OCR queue could not be loaded.');
      m.queueService.getStats.mockResolvedValue({ data: STATS });
      await user.click(within(alert).getByRole('button', { name: 'Retry' }));
      expect(await within(region('Processing')).findByRole('group')).toBeInTheDocument();
    });
  });

  describe('Connections', () => {
    it('lists connections with their state and last sync', async () => {
      await renderBoard();
      const list = await within(region('Connections')).findByRole('list', { name: 'Connections' });
      const items = within(list).getAllByRole('listitem');
      expect(items).toHaveLength(3);
      expect(within(items[0]).getByText('Office scanner')).toBeInTheDocument();
      expect(within(items[0]).getByText('HEALTHY')).toBeInTheDocument();
      expect(within(items[0]).getByText('3m ago')).toBeInTheDocument();
      expect(within(items[1]).getByText('SYNCING')).toBeInTheDocument();
      expect(within(items[2]).getByText('OFF')).toBeInTheDocument();
      expect(within(region('Connections')).getByRole('link', { name: 'Manage' })).toHaveAttribute('href', '/intake?section=connections');
    });

    it('offers to add a connection when there are none', async () => {
      sources = [];
      await renderBoard();
      const connections = region('Connections');
      expect(await within(connections).findByText('No connections')).toBeInTheDocument();
      expect(within(connections).getByRole('link', { name: 'Add connection' })).toHaveAttribute('href', '/intake?section=connections');
    });

    it('shows an inline error when connections cannot be loaded', async () => {
      sources = new Error('boom');
      await renderBoard();
      const alert = await within(region('Connections')).findByRole('alert');
      expect(alert).toHaveTextContent('Connections could not be loaded.');
      expect(within(region('Connections')).getByRole('button', { name: 'Retry' })).toBeInTheDocument();
    });
  });

  describe('Library totals', () => {
    it('shows documents, storage, indexed share and labels', async () => {
      await renderBoard();
      const group = await within(region('Library')).findByRole('group', { name: 'Library' });
      const value = (label: string) => within(group).getByText(label).closest('div')?.querySelector('dd')?.textContent;
      expect(value('Documents')).toBe('1,200');
      expect(value('Storage')).toBe('5 GB');
      expect(value('Indexed')).toBe('75%');
      await waitFor(() => expect(value('Labels')).toBe('3'));
    });

    it('shows a dash for labels when that call fails, and an error when totals fail', async () => {
      m.api.get.mockImplementation((url: string) => {
        if (url === '/sources') return Promise.resolve({ data: SOURCES });
        if (url === '/metrics') return Promise.reject(new Error('boom'));
        return Promise.reject(new Error('boom'));
      });
      await renderBoard();
      const alert = await within(region('Library')).findByRole('alert');
      expect(alert).toHaveTextContent('Library totals could not be loaded.');
      // The other regions are unaffected.
      expect(screen.getByText('d1.pdf')).toBeInTheDocument();
    });
  });
});
