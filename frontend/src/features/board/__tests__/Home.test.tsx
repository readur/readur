import { act, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const m = vi.hoisted(() => ({
  api: { get: vi.fn() },
  documentService: { listWithPagination: vi.fn(), getFailedOcrDocuments: vi.fn(), getThumbnail: vi.fn() },
  queueService: { getStats: vi.fn(), getOcrStatus: vi.fn(), pauseOcr: vi.fn(), resumeOcr: vi.fn() },
  sourceService: { getArrivals: vi.fn() },
}));

vi.mock('../../../services/api', () => ({
  default: m.api,
  api: m.api,
  documentService: m.documentService,
  queueService: m.queueService,
  sourceService: m.sourceService,
}));

import Home, { HomePage } from '../index';
import { ToastProvider } from '../../../ui';
import { isLit, markLit } from '../litStore';
import { doc, lane, renderPage, resetBoardState } from './homeTestUtils';
import type { UserRole } from '../../../types/generated';

const STATS = { pending: 12, processing: 4, completed_today: 41, oldest_pending_minutes: 130 };

const DOC_TOOLS =
  'OCR extraction failed: None of the DOC extraction tools (antiword, catdoc, wvText) are available or working.\n\nTried tools: antiword';
const OCR_PATH = "OCR extraction failed: OCR failed for '/app/uploads/documents/abc.pdf' after trying multiple strategies.";

let lanes: unknown;

const LANES = () => [
  lane('upload', [3, 2, 0, 4, 1, 0, 0, 2, 5, 1, 0, 3, 2, 6], { name: 'Uploads' }),
  lane('watch', [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], { name: 'Watch folder' }),
  lane('s1', [2, 1, 3, 2, 1, 2, 3, 1, 2, 2, 1, 2, 3, 0], {
    name: 'Scanner inbox',
    kind: 'local_folder',
    last_arrival_at: new Date(Date.now() - 3 * 24 * 3600_000).toISOString(),
  }),
];

function serve() {
  lanes = LANES();
  m.documentService.listWithPagination.mockResolvedValue({
    data: {
      documents: [doc('d1', { source_id: 's1', source_type: 'local_folder' }), doc('d2', { source_type: 'web_upload' })],
      pagination: { total: 2 },
    },
  });
  m.documentService.getFailedOcrDocuments.mockResolvedValue({ data: { documents: [], pagination: { total: 0 } } });
  m.documentService.getThumbnail.mockRejectedValue(new Error('none'));
  m.queueService.getStats.mockResolvedValue({ data: STATS });
  m.queueService.getOcrStatus.mockResolvedValue({ data: { is_paused: false, status: 'running' } });
  m.queueService.pauseOcr.mockResolvedValue({ data: {} });
  m.queueService.resumeOcr.mockResolvedValue({ data: {} });
  m.sourceService.getArrivals.mockImplementation(() =>
    lanes instanceof Error ? Promise.reject(lanes) : Promise.resolve({ data: lanes }),
  );
}

const region = (name: string) => screen.getByRole('region', { name });
const loc = () => screen.getByRole('status', { name: 'location', hidden: true }).textContent;
const laneItem = (name: string) =>
  within(screen.getByRole('list', { name: 'Coming in' }))
    .getAllByRole('listitem')
    .find((li) => within(li).queryByRole('link', { name }))!;

async function renderHome(role: UserRole = 'admin') {
  const view = renderPage(
    <ToastProvider>
      <Home />
    </ToastProvider>,
    role,
  );
  await screen.findByText('d1.pdf');
  await screen.findByRole('list', { name: 'Coming in' });
  return view;
}

beforeEach(() => {
  resetBoardState();
  vi.clearAllMocks();
  serve();
});

afterEach(() => {
  vi.useRealTimers();
});

describe('Home', () => {
  it('is exported for the route both as default and as HomePage', () => {
    expect(HomePage).toBe(Home);
  });

  it('greets the user, counts this week and shows every region', async () => {
    await renderHome();
    expect(screen.getAllByRole('heading', { level: 1 })).toHaveLength(1);
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent(/^Good (morning|afternoon|evening), ada$/);
    // Last seven days: uploads 0+2+5+1+0+3+2+6 → 19 over the last 7 entries (2,5,1,0,3,2,6) = 19, scanner 1+2+2+1+2+3+0 = 11.
    expect(await screen.findByText('30 arrived this week')).toBeInTheDocument();
    expect(screen.getAllByRole('link', { name: 'Add documents' })[0]).toHaveAttribute('href', '/intake?section=upload');
    expect(region('Coming in')).toBeInTheDocument();
    expect(region('Processing')).toBeInTheDocument();
    expect(region('Just arrived')).toBeInTheDocument();
  });

  it('says when nothing arrived this week', async () => {
    serve();
    lanes = [lane('upload', [4, 0, 0, 0, 0, 0, 0, 0], { name: 'Uploads' })];
    await renderHome();
    expect(await screen.findByText('Nothing arrived this week')).toBeInTheDocument();
  });

  it('greets without a name when there is no user', async () => {
    const { render } = await import('@testing-library/react');
    const { MemoryRouter } = await import('react-router-dom');
    const { AuthContext } = await import('../../../contexts/AuthContext');
    render(
      <MemoryRouter>
        <AuthContext.Provider value={{ user: null, loading: false } as never}>
          <ToastProvider>
            <Home />
          </ToastProvider>
        </AuthContext.Provider>
      </MemoryRouter>,
    );
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent(/^Good (morning|afternoon|evening)$/);
    await screen.findByText('d1.pdf');
  });

  it('shows loading placeholders before anything has arrived', () => {
    const never = new Promise(() => {});
    m.documentService.listWithPagination.mockReturnValue(never);
    m.queueService.getStats.mockReturnValue(never);
    m.queueService.getOcrStatus.mockReturnValue(never);
    m.documentService.getFailedOcrDocuments.mockReturnValue(never);
    m.sourceService.getArrivals.mockReturnValue(never);
    renderPage(<Home />);
    expect(screen.getAllByRole('status', { name: 'Loading' }).length).toBeGreaterThanOrEqual(3);
  });

  describe('Coming in', () => {
    it('shows one lane per source with its strip, today, last arrival and health', async () => {
      await renderHome();
      const uploads = laneItem('Uploads');
      expect(within(uploads).getByRole('link', { name: 'Uploads' })).toHaveAttribute('href', '/intake?section=upload');
      expect(within(uploads).getByText('Web and API uploads')).toBeInTheDocument();
      expect(within(uploads).getByText('6')).toBeInTheDocument();
      expect(within(uploads).getByText(/Last arrival/)).toBeInTheDocument();
      expect(within(uploads).getByRole('img', { name: /^29 in the last 14 days, most on .+ \(6\)$/ })).toBeInTheDocument();
      expect(within(uploads).getByText('HEALTHY')).toBeInTheDocument();

      const watch = laneItem('Watch folder');
      expect(within(watch).getByRole('link', { name: 'Watch folder' })).toHaveAttribute('href', '/intake?section=watch');
      expect(within(watch).getByText('Nothing yet')).toBeInTheDocument();
      expect(within(watch).getByRole('img', { name: 'Nothing in the last 14 days' })).toBeInTheDocument();
    });

    it('flags a lane that usually receives documents but has gone quiet', async () => {
      await renderHome();
      const scanner = laneItem('Scanner inbox');
      expect(within(scanner).getByRole('link', { name: 'Scanner inbox' })).toHaveAttribute(
        'href',
        '/intake?section=connections&source=s1',
      );
      expect(within(scanner).getByText('Local folder')).toBeInTheDocument();
      expect(within(scanner).getByText('Quiet')).toBeInTheDocument();
      // The watch folder never receives anything, so it is not quiet.
      expect(within(laneItem('Watch folder')).queryByText('Quiet')).not.toBeInTheDocument();
    });

    it('shows error, syncing and off states', async () => {
      serve();
      lanes = [
        lane('e1', [1], { name: 'Broken share', status: 'error' }),
        lane('y1', [1], { name: 'Busy share', status: 'syncing', kind: 's3' }),
        lane('o1', [0], { name: 'Old share', enabled: false }),
      ];
      await renderHome();
      expect(within(laneItem('Broken share')).getByText('ERROR')).toBeInTheDocument();
      expect(within(laneItem('Busy share')).getByText('SYNCING')).toBeInTheDocument();
      expect(within(laneItem('Busy share')).getByText('S3')).toBeInTheDocument();
      expect(within(laneItem('Old share')).getByText('OFF')).toBeInTheDocument();
      expect(within(laneItem('Broken share')).getByText('WebDAV')).toBeInTheDocument();
    });

    it('says when there are no sources', async () => {
      serve();
      lanes = [];
      renderPage(<Home />);
      expect(await within(await screen.findByRole('region', { name: 'Coming in' })).findByText('No sources yet')).toBeInTheDocument();
    });

    it('shows an inline error with Retry that recovers', async () => {
      const user = userEvent.setup();
      lanes = new Error('boom');
      renderPage(<Home />);
      const coming = await screen.findByRole('region', { name: 'Coming in' });
      const alert = await within(coming).findByRole('alert');
      expect(alert).toHaveTextContent('Arrivals could not be loaded.');
      lanes = LANES();
      await user.click(within(alert).getByRole('button', { name: 'Retry' }));
      expect(await within(coming).findByRole('list', { name: 'Coming in' })).toBeInTheDocument();
    });

    it('asks the server for 14 days', async () => {
      await renderHome();
      expect(m.sourceService.getArrivals).toHaveBeenCalledWith(14);
    });
  });

  describe('Processing', () => {
    it('shows what is processing and pending', async () => {
      await renderHome();
      expect(await within(region('Processing')).findByText('Processing 4 · pending 12')).toBeInTheDocument();
    });

    it('says "Nothing waiting" when the queue is empty', async () => {
      m.queueService.getStats.mockResolvedValue({ data: { pending_count: 0, processing_count: 0 } });
      await renderHome();
      expect(await within(region('Processing')).findByText('Nothing waiting')).toBeInTheDocument();
    });

    it('lets an admin pause OCR', async () => {
      const user = userEvent.setup();
      await renderHome('admin');
      await user.click(await within(region('Processing')).findByRole('button', { name: 'Pause OCR' }));
      expect(m.queueService.pauseOcr).toHaveBeenCalled();
    });

    it('shows paused OCR and lets an admin resume it', async () => {
      const user = userEvent.setup();
      m.queueService.getOcrStatus.mockResolvedValue({ data: { is_paused: true } });
      await renderHome('admin');
      expect(await within(region('Processing')).findByText('OCR is paused')).toBeInTheDocument();
      await user.click(within(region('Processing')).getByRole('button', { name: 'Resume OCR' }));
      expect(m.queueService.resumeOcr).toHaveBeenCalled();
    });

    it('tells the admin when pausing fails', async () => {
      const user = userEvent.setup();
      m.queueService.pauseOcr.mockRejectedValue(new Error('no'));
      await renderHome('admin');
      await user.click(await within(region('Processing')).findByRole('button', { name: 'Pause OCR' }));
      expect(await screen.findByText('Could not pause OCR')).toBeInTheDocument();
    });

    it('hides pause and resume from non-admins', async () => {
      await renderHome('user');
      await within(region('Processing')).findByText('Processing 4 · pending 12');
      expect(screen.queryByRole('button', { name: 'Pause OCR' })).not.toBeInTheDocument();
    });

    it('leaves the line out, without an error, when the queue is admin-only', async () => {
      m.queueService.getStats.mockRejectedValue({ response: { status: 403 } });
      await renderHome('user');
      await within(region('Processing')).findByText('No failed documents');
      expect(within(region('Processing')).queryByText(/pending/)).not.toBeInTheDocument();
      expect(within(region('Processing')).queryByRole('alert')).not.toBeInTheDocument();
    });

    it('shows an inline error with Retry when the queue cannot be loaded', async () => {
      const user = userEvent.setup();
      m.queueService.getStats.mockRejectedValue(new Error('boom'));
      await renderHome();
      const alert = await within(region('Processing')).findByRole('alert');
      expect(alert).toHaveTextContent('The OCR queue could not be loaded.');
      m.queueService.getStats.mockResolvedValue({ data: STATS });
      await user.click(within(alert).getByRole('button', { name: 'Retry' }));
      expect(await within(region('Processing')).findByText('Processing 4 · pending 12')).toBeInTheDocument();
    });

    it('shows an error when the OCR state cannot be loaded', async () => {
      m.queueService.getOcrStatus.mockRejectedValue(new Error('boom'));
      await renderHome();
      expect(await within(region('Processing')).findByText('The OCR state could not be loaded.')).toBeInTheDocument();
    });

    it('refreshes the queue every 15 seconds', async () => {
      vi.useFakeTimers({ shouldAdvanceTime: true });
      await renderHome();
      await within(region('Processing')).findByText('Processing 4 · pending 12');
      m.queueService.getStats.mockResolvedValue({ data: { ...STATS, pending: 9 } });
      await act(async () => {
        await vi.advanceTimersByTimeAsync(15_000);
      });
      await waitFor(() => expect(within(region('Processing')).getByText('Processing 4 · pending 9')).toBeInTheDocument());
    });
  });

  describe('Failures', () => {
    const failures = () => ({
      data: {
        documents: [
          { id: 'f1', filename: 'letter.doc', ocr_error: DOC_TOOLS, ocr_failure_reason: 'other', updated_at: new Date().toISOString() },
          { id: 'f2', filename: 'scan.pdf', ocr_error: OCR_PATH, ocr_failure_reason: 'ocr_memory_limit' },
          { id: 'f3', filename: 'scan2.pdf', error_message: OCR_PATH, failure_reason: 'ocr_memory_limit' },
          { id: 'f4', original_filename: 'odd.png', failure_category: 'Quota reached' },
        ],
        pagination: { total: 190 },
      },
    });

    it('shows one line with the count, the main causes in plain words and a Review link', async () => {
      m.documentService.getFailedOcrDocuments.mockResolvedValue(failures());
      await renderHome();
      const panel = region('Processing');
      expect(await within(panel).findByText('190 failed')).toBeInTheDocument();
      expect(within(panel).getByText("OCR failed · Can't read .doc files: install antiword or catdoc")).toBeInTheDocument();
      expect(within(panel).getByRole('link', { name: 'Review' })).toHaveAttribute('href', '/intake?section=attention');
      // The raw text is not on the page until asked for.
      for (const raw of within(panel).getAllByText(/\/app\/uploads/)) expect(raw.closest('details:not([open])')).not.toBeNull();
    });

    it('lists the latest failures behind a disclosure, raw text behind another', async () => {
      const user = userEvent.setup();
      m.documentService.getFailedOcrDocuments.mockResolvedValue(failures());
      await renderHome();
      const panel = region('Processing');
      await user.click(await within(panel).findByText('Show the latest failures'));
      const item = within(panel).getByRole('link', { name: 'scan.pdf' }).closest('li') as HTMLElement;
      expect(within(item).getByText('OCR failed')).toBeInTheDocument();
      await user.click(within(item).getByText('Error text'));
      expect(within(item).getByText(/\/app\/uploads\/documents\/abc\.pdf/)).toBeVisible();
      expect(within(panel).getByRole('link', { name: 'odd.png' })).toHaveAttribute('href', '/documents/f4');
      expect(within(panel).getByText('Quota reached')).toBeInTheDocument();
    });

    it('says there are no failures', async () => {
      await renderHome();
      expect(await within(region('Processing')).findByText('No failed documents')).toBeInTheDocument();
    });

    it('shows an error with Retry when failed documents cannot be loaded', async () => {
      const user = userEvent.setup();
      m.documentService.getFailedOcrDocuments.mockRejectedValue(new Error('boom'));
      await renderHome();
      const alert = await within(region('Processing')).findByText('Failed documents could not be loaded.');
      m.documentService.getFailedOcrDocuments.mockResolvedValue(failures());
      await user.click(within(alert.closest('[role="alert"]') as HTMLElement).getByRole('button', { name: 'Retry' }));
      expect(await within(region('Processing')).findByText('190 failed')).toBeInTheDocument();
    });
  });

  describe('Just arrived', () => {
    const cards = () => within(screen.getByRole('list', { name: 'Just arrived' })).getAllByRole('listitem');
    const card = (name: string) => cards().find((c) => within(c).queryByRole('link', { name: new RegExp(name) }))!;

    it('shows the newest documents with their source and when they arrived', async () => {
      await renderHome();
      expect(within(card('d1.pdf')).getByText('Scanner inbox')).toBeInTheDocument();
      expect(within(card('d2.pdf')).getByText('Uploads')).toBeInTheDocument();
      expect(within(card('d1.pdf')).getByText(/5 min|5m/)).toBeInTheDocument();
      expect(within(card('d1.pdf')).getByRole('link')).toHaveAttribute('href', '/documents/d1');
      expect(m.documentService.listWithPagination).toHaveBeenCalledWith(12, 0);
    });

    it('names a source it cannot find, and dates older documents', async () => {
      m.documentService.listWithPagination.mockResolvedValue({
        data: {
          documents: [
            doc('x1', { source_id: 'gone', source_type: 'webdav', created_at: '2026-01-02T10:00:00Z' }),
            doc('x2', { source_type: 'watch_folder' }),
          ],
          pagination: { total: 2 },
        },
      });
      lanes = [];
      renderPage(<Home />);
      await screen.findByText('x1.pdf');
      expect(within(card('x1.pdf')).getByText('Source')).toBeInTheDocument();
      expect(within(card('x1.pdf')).getByText(/2026/)).toBeInTheDocument();
      expect(within(card('x2.pdf')).getByText('Watch folder')).toBeInTheDocument();
    });

    it('marks new items and clears the mark when one is opened', async () => {
      markLit('document', 'd1', 'new');
      const user = userEvent.setup();
      await renderHome();
      expect(within(card('d1.pdf')).getByText('New')).toBeInTheDocument();
      expect(within(card('d2.pdf')).queryByText('New')).not.toBeInTheDocument();
      await user.click(within(card('d1.pdf')).getByRole('link'));
      expect(loc()).toBe('/documents/d1');
      expect(isLit('document', 'd1')).toBe(false);
    });

    it('polls every 15 seconds and marks new documents and finished OCR', async () => {
      vi.useFakeTimers({ shouldAdvanceTime: true });
      await renderHome();
      expect(within(card('d1.pdf')).queryByText('New')).not.toBeInTheDocument();
      m.documentService.listWithPagination.mockResolvedValue({
        data: {
          documents: [doc('d3', { created_at: new Date().toISOString() }), doc('d1'), doc('d2', { ocr_status: 'failed' })],
          pagination: { total: 3 },
        },
      });
      await act(async () => {
        await vi.advanceTimersByTimeAsync(15_000);
      });
      await screen.findByText('d3.pdf');
      expect(within(card('d3.pdf')).getByText('New')).toBeInTheDocument();
      expect(within(card('d2.pdf')).getByText('Changed')).toBeInTheDocument();
    });

    it('does not poll while the tab is hidden', async () => {
      vi.useFakeTimers({ shouldAdvanceTime: true });
      await renderHome();
      const spy = vi.spyOn(document, 'visibilityState', 'get').mockReturnValue('hidden');
      await act(async () => {
        await vi.advanceTimersByTimeAsync(45_000);
      });
      expect(m.documentService.listWithPagination).toHaveBeenCalledTimes(1);
      spy.mockRestore();
    });

    it('shows the empty state with a call to action for an empty library', async () => {
      m.documentService.listWithPagination.mockResolvedValue({ data: { documents: [], pagination: { total: 0 } } });
      renderPage(<Home />);
      const recent = await screen.findByRole('region', { name: 'Just arrived' });
      expect(await within(recent).findByText('No documents yet')).toBeInTheDocument();
      expect(within(recent).getByRole('link', { name: 'Add documents' })).toHaveAttribute('href', '/intake?section=upload');
    });

    it('shows an inline error with Retry that recovers, and keeps the rest of the page', async () => {
      const user = userEvent.setup();
      m.documentService.listWithPagination.mockRejectedValue(new Error('boom'));
      renderPage(<Home />);
      const recent = await screen.findByRole('region', { name: 'Just arrived' });
      const alert = await within(recent).findByRole('alert');
      expect(alert).toHaveTextContent('Recent documents could not be loaded.');
      expect(await within(region('Processing')).findByText('Processing 4 · pending 12')).toBeInTheDocument();
      serve();
      await user.click(within(alert).getByRole('button', { name: 'Retry' }));
      expect(await screen.findByText('d1.pdf')).toBeInTheDocument();
    });

    it('keeps the items and shows the error when a refresh fails', async () => {
      vi.useFakeTimers({ shouldAdvanceTime: true });
      await renderHome();
      m.documentService.listWithPagination.mockRejectedValue(new Error('boom'));
      await act(async () => {
        await vi.advanceTimersByTimeAsync(15_000);
      });
      expect(await within(region('Just arrived')).findByRole('alert')).toBeInTheDocument();
      expect(screen.getByText('d1.pdf')).toBeInTheDocument();
    });
  });
});
