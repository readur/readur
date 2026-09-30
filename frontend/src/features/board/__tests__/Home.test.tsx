import { act, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const m = vi.hoisted(() => ({
  api: { get: vi.fn() },
  documentService: { listWithPagination: vi.fn(), getFailedOcrDocuments: vi.fn(), getThumbnail: vi.fn() },
  queueService: { getStats: vi.fn(), getOcrStatus: vi.fn(), pauseOcr: vi.fn(), resumeOcr: vi.fn() },
  sourceService: { getArrivals: vi.fn(), list: vi.fn() },
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
import { doc, lane, renderPage, resetBoardState, seenUpToNow } from './homeTestUtils';
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
  m.sourceService.list.mockResolvedValue({ data: [] });
  m.sourceService.getArrivals.mockImplementation(() =>
    lanes instanceof Error ? Promise.reject(lanes) : Promise.resolve({ data: lanes }),
  );
}

const region = (name: string) => screen.getByRole('region', { name });
const summary = () => screen.getByRole('group', { name: 'Summary' });
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
  await within(await screen.findByRole('region', { name: 'Coming in' })).findAllByText(/today|Nothing arrived|No sources/);
  return view;
}

beforeEach(() => {
  resetBoardState();
  seenUpToNow();
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
    expect(await within(summary()).findByText('30 arrived this week')).toBeInTheDocument();
    expect(screen.getAllByRole('link', { name: 'Add documents' })[0]).toHaveAttribute('href', '/intake?section=upload');
    expect(region('Coming in')).toBeInTheDocument();
    expect(region('Processing')).toBeInTheDocument();
    expect(region('Just arrived')).toBeInTheDocument();
  });

  it('orders the page: summary, Just arrived, Coming in, then the processing detail', async () => {
    await renderHome();
    const order = [summary(), region('Just arrived'), region('Coming in'), region('Processing')];
    for (let i = 1; i < order.length; i += 1) {
      expect(order[i - 1].compareDocumentPosition(order[i]) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    }
  });

  it('sums up processing and failures in the line under the greeting', async () => {
    m.documentService.getFailedOcrDocuments.mockResolvedValue({ data: { documents: [], pagination: { total: 3 } } });
    await renderHome();
    expect(await within(summary()).findByText('4 processing · 12 pending')).toBeInTheDocument();
    expect(await within(summary()).findByText('3 failed')).toBeInTheDocument();
    expect(within(summary()).getByRole('link', { name: 'Review' })).toHaveAttribute('href', '/intake?section=attention');
  });

  it('says "nothing waiting" when the queue is empty, and only pending when nothing runs', async () => {
    m.queueService.getStats.mockResolvedValue({ data: { pending_count: 0, processing_count: 0 } });
    const view = await renderHome();
    expect(await within(summary()).findByText('nothing waiting')).toBeInTheDocument();
    expect(within(summary()).queryByText(/failed/)).not.toBeInTheDocument();
    view.unmount();
    m.queueService.getStats.mockResolvedValue({ data: { pending: 5, processing: 0 } });
    await renderHome();
    expect(await within(summary()).findByText('5 pending')).toBeInTheDocument();
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
      expect(within(uploads).getByText('Healthy')).toBeInTheDocument();

      const watch = laneItem('Watch folder');
      expect(within(watch).getByRole('link', { name: 'Watch folder' })).toHaveAttribute('href', '/intake?section=watch');
      expect(within(watch).getByText('Nothing yet')).toBeInTheDocument();
      expect(within(watch).getByRole('img', { name: 'Nothing in the last 14 days' })).toBeInTheDocument();
      // A lane that never received anything is idle, not healthy; its kind is not repeated under its name.
      expect(within(watch).getByText('Idle')).toBeInTheDocument();
      expect(within(watch).getAllByText('Watch folder')).toHaveLength(1);
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
        lane('w1', [1], { name: 'Checked share' }),
      ];
      m.sourceService.list.mockResolvedValue({ data: [{ id: 'w1', validation_status: 'warning' }] });
      await renderHome();
      expect(within(laneItem('Broken share')).getByText('Error')).toBeInTheDocument();
      expect(within(laneItem('Busy share')).getByText('Syncing')).toBeInTheDocument();
      expect(within(laneItem('Busy share')).getByText('S3')).toBeInTheDocument();
      expect(within(laneItem('Old share')).getByText('Off')).toBeInTheDocument();
      expect(within(laneItem('Broken share')).getByText('WebDAV')).toBeInTheDocument();
      expect(within(laneItem('Checked share')).getByText(/^check$/i)).toBeInTheDocument();
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

    it('shows the five most active lanes, problems first, and links to the rest in Intake', async () => {
      serve();
      lanes = [
        ...Array.from({ length: 190 }, (_, i) => lane(`idle${i}`, [0, 0], { name: `Idle ${i}`, last_arrival_at: null })),
        ...Array.from({ length: 6 }, (_, i) => lane(`a${i}`, [0, i + 1], { name: `Active ${i}` })),
        lane('bad', [0, 0], { name: 'Broken share', status: 'error', last_arrival_at: null }),
      ];
      await renderHome();
      const shown = within(screen.getByRole('list', { name: 'Coming in' })).getAllByRole('link');
      // The five most active, in activity order; the broken source takes the least active slot.
      expect(shown.map((a) => a.textContent)).toEqual(['Active 5', 'Active 4', 'Active 3', 'Active 2', 'Broken share']);
      expect(within(region('Coming in')).getByRole('link', { name: /^192 more sources/ })).toHaveAttribute(
        'href',
        '/intake?section=connections',
      );
      expect(within(region('Coming in')).getByRole('link', { name: 'Manage sources' })).toBeInTheDocument();
      expect(within(region('Coming in')).queryByText('Idle 0')).not.toBeInTheDocument();
    });

    it('says so when none of many sources received anything', async () => {
      serve();
      lanes = Array.from({ length: 7 }, (_, i) => lane(`idle${i}`, [0], { name: `Idle ${i}`, last_arrival_at: null }));
      await renderHome();
      expect(within(region('Coming in')).getByText('Nothing arrived from any source in the last 14 days.')).toBeInTheDocument();
      expect(within(region('Coming in')).getByRole('link', { name: /^7 more sources/ })).toBeInTheDocument();
    });

    it('asks the server for 14 days', async () => {
      await renderHome();
      expect(m.sourceService.getArrivals).toHaveBeenCalledWith(14);
    });
  });

  describe('Processing detail', () => {
    it('adds the day\'s throughput and the oldest wait', async () => {
      await renderHome();
      expect(await within(region('Processing')).findByText('41 done today')).toBeInTheDocument();
      expect(within(region('Processing')).getByText('oldest waiting 2h 10m')).toBeInTheDocument();
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
      await within(region('Processing')).findByText('41 done today');
      expect(screen.queryByRole('button', { name: 'Pause OCR' })).not.toBeInTheDocument();
    });

    it('leaves the queue out, without an error, when it is admin-only; with no failures there is no detail', async () => {
      m.queueService.getStats.mockRejectedValue({ response: { status: 403 } });
      await renderHome('user');
      await waitFor(() => expect(screen.queryByRole('region', { name: 'Processing' })).not.toBeInTheDocument());
      expect(within(summary()).queryByText(/pending|waiting/)).not.toBeInTheDocument();
      expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    });

    it('shows an inline error with Retry when the queue cannot be loaded', async () => {
      const user = userEvent.setup();
      m.queueService.getStats.mockRejectedValue(new Error('boom'));
      await renderHome();
      const alert = await within(region('Processing')).findByRole('alert');
      expect(alert).toHaveTextContent('The OCR queue could not be loaded.');
      m.queueService.getStats.mockResolvedValue({ data: STATS });
      await user.click(within(alert).getByRole('button', { name: 'Retry' }));
      expect(await within(region('Processing')).findByText('41 done today')).toBeInTheDocument();
    });

    it('shows an error when the OCR state cannot be loaded', async () => {
      m.queueService.getOcrStatus.mockRejectedValue(new Error('boom'));
      await renderHome();
      expect(await within(region('Processing')).findByText('The OCR state could not be loaded.')).toBeInTheDocument();
    });

    it('refreshes the queue every 15 seconds', async () => {
      vi.useFakeTimers({ shouldAdvanceTime: true });
      await renderHome();
      await within(summary()).findByText('4 processing · 12 pending');
      m.queueService.getStats.mockResolvedValue({ data: { ...STATS, pending: 9 } });
      await act(async () => {
        await vi.advanceTimersByTimeAsync(15_000);
      });
      await waitFor(() => expect(within(summary()).getByText('4 processing · 9 pending')).toBeInTheDocument());
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

    it('names the main causes in plain words, with one Review link in the summary', async () => {
      m.documentService.getFailedOcrDocuments.mockResolvedValue(failures());
      await renderHome();
      const panel = region('Processing');
      expect(await within(panel).findByText('190 failed')).toBeInTheDocument();
      expect(within(summary()).getByText('190 failed')).toBeInTheDocument();
      expect(within(panel).getByText("OCR failed · Can't read .doc files: install antiword or catdoc")).toBeInTheDocument();
      expect(screen.getAllByRole('link', { name: 'Review' })).toHaveLength(1);
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

    it('shows no failure line when nothing failed', async () => {
      await renderHome();
      await within(region('Processing')).findByText('41 done today');
      expect(screen.queryByText(/failed$/)).not.toBeInTheDocument();
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

    it('on a first visit, marks the last day\'s arrivals as new', async () => {
      resetBoardState();
      m.documentService.listWithPagination.mockResolvedValue({
        data: { documents: [doc('d1'), doc('old', { created_at: '2026-01-01T00:00:00Z' })], pagination: { total: 2 } },
      });
      renderPage(<Home />);
      await screen.findByText('old.pdf');
      await waitFor(() => expect(within(card('d1.pdf')).getByText('New')).toBeInTheDocument());
      expect(within(card('old.pdf')).queryByText('New')).not.toBeInTheDocument();
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
      expect(await within(summary()).findByText('4 processing · 12 pending')).toBeInTheDocument();
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
