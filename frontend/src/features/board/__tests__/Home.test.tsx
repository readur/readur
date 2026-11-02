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
const sourceRow = (name: string) =>
  within(screen.getByRole('list', { name: 'Sources' }))
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
  await within(await screen.findByRole('region', { name: 'Sources' })).findAllByText(/today|Nothing|No sources|Last arrival/);
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

  it('greets the user, counts this week, offers Connect source and Upload, and shows every region', async () => {
    await renderHome();
    expect(screen.getAllByRole('heading', { level: 1 })).toHaveLength(1);
    expect(screen.getByRole('heading', { level: 1 })).toHaveTextContent(/^Good (morning|afternoon|evening), ada$/);
    expect(await within(summary()).findByText('30 arrived this week')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: 'Connect source' })).toHaveAttribute('href', '/sources?section=connections&new=1');
    expect(screen.getAllByRole('link', { name: 'Upload' })[0]).toHaveAttribute('href', '/intake?section=upload');
    expect(region('Processing pipeline')).toBeInTheDocument();
    expect(region('Sources')).toBeInTheDocument();
    expect(region('Just arrived')).toBeInTheDocument();
  });

  it('reads pipeline, then needs attention and sources, then Just arrived (the side column drops inline when narrow)', async () => {
    m.documentService.getFailedOcrDocuments.mockResolvedValue({ data: { documents: [], pagination: { total: 3 } } });
    await renderHome();
    await screen.findByRole('region', { name: 'Needs attention' });
    const order = [summary(), region('Processing pipeline'), region('Needs attention'), region('Sources'), region('Just arrived')];
    for (let i = 1; i < order.length; i += 1) {
      expect(order[i - 1].compareDocumentPosition(order[i]) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    }
  });

  it('sums up processing and failures in the line under the greeting', async () => {
    m.documentService.getFailedOcrDocuments.mockResolvedValue({ data: { documents: [], pagination: { total: 3 } } });
    await renderHome();
    expect(await within(summary()).findByText('4 processing · 12 pending')).toBeInTheDocument();
    expect(await within(summary()).findByText('3 failed')).toBeInTheDocument();
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

  describe('Sources', () => {
    it('lists each way documents come in with its icon, kind, today or last arrival, and health', async () => {
      await renderHome();
      const uploads = sourceRow('Uploads');
      expect(within(uploads).getByRole('link', { name: 'Uploads' })).toHaveAttribute('href', '/intake?section=upload');
      expect(uploads.querySelector('[data-tile] svg')).not.toBeNull();
      expect(within(uploads).getByText('Web and API uploads · 6 today')).toBeInTheDocument();
      expect(within(uploads).getByText('Healthy')).toBeInTheDocument();

      const watch = sourceRow('Watch folder');
      expect(within(watch).getByRole('link', { name: 'Watch folder' })).toHaveAttribute('href', '/sources?section=watch');
      // A lane that never received anything is idle, not healthy; its kind is not repeated under its name.
      expect(within(watch).getByText('Nothing yet')).toBeInTheDocument();
      expect(within(watch).getByText('Idle')).toBeInTheDocument();
    });

    it('flags a lane that usually receives documents but has gone quiet', async () => {
      await renderHome();
      const scanner = sourceRow('Scanner inbox');
      expect(within(scanner).getByRole('link', { name: 'Scanner inbox' })).toHaveAttribute('href', '/sources?section=connections&source=s1');
      expect(within(scanner).getByText(/^Local folder · Last arrival/)).toBeInTheDocument();
      expect(within(scanner).getByText('Quiet')).toBeInTheDocument();
      expect(within(sourceRow('Watch folder')).queryByText('Quiet')).not.toBeInTheDocument();
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
      expect(within(sourceRow('Broken share')).getByText('Error')).toBeInTheDocument();
      expect(within(sourceRow('Busy share')).getByText('Syncing')).toBeInTheDocument();
      expect(within(sourceRow('Busy share')).getByText(/^S3 · /)).toBeInTheDocument();
      expect(within(sourceRow('Old share')).getByText('Off')).toBeInTheDocument();
      expect(within(sourceRow('Checked share')).getByText(/^check$/i)).toBeInTheDocument();
    });

    it('offers to connect a source when there are none', async () => {
      serve();
      lanes = [];
      renderPage(<Home />);
      const card = await screen.findByRole('region', { name: 'Sources' });
      expect(await within(card).findByText('No sources yet')).toBeInTheDocument();
    });

    it('shows an inline error with Retry that recovers', async () => {
      const user = userEvent.setup();
      lanes = new Error('boom');
      renderPage(<Home />);
      const card = await screen.findByRole('region', { name: 'Sources' });
      const alert = await within(card).findByRole('alert');
      expect(alert).toHaveTextContent('Arrivals could not be loaded.');
      lanes = LANES();
      await user.click(within(alert).getByRole('button', { name: 'Retry' }));
      expect(await within(card).findByRole('list', { name: 'Sources' })).toBeInTheDocument();
    });

    it('shows the five most active, problems first, and links to the rest', async () => {
      serve();
      lanes = [
        ...Array.from({ length: 190 }, (_, i) => lane(`idle${i}`, [0, 0], { name: `Idle ${i}`, last_arrival_at: null })),
        ...Array.from({ length: 6 }, (_, i) => lane(`a${i}`, [0, i + 1], { name: `Active ${i}` })),
        lane('bad', [0, 0], { name: 'Broken share', status: 'error', last_arrival_at: null }),
      ];
      await renderHome();
      const shown = within(screen.getByRole('list', { name: 'Sources' })).getAllByRole('link');
      expect(shown.map((a) => a.textContent)).toEqual(['Active 5', 'Active 4', 'Active 3', 'Active 2', 'Broken share']);
      expect(within(region('Sources')).getByRole('link', { name: /^192 more sources/ })).toHaveAttribute('href', '/sources');
      expect(within(region('Sources')).getByRole('link', { name: 'Manage' })).toHaveAttribute('href', '/sources');
    });

    it('says so when none of many sources received anything', async () => {
      serve();
      lanes = Array.from({ length: 7 }, (_, i) => lane(`idle${i}`, [0], { name: `Idle ${i}`, last_arrival_at: null }));
      await renderHome();
      expect(within(region('Sources')).getByText('Nothing arrived from any source in the last 14 days.')).toBeInTheDocument();
      expect(within(region('Sources')).getByRole('link', { name: /^7 more sources/ })).toBeInTheDocument();
    });

    it('asks the server for 14 days', async () => {
      await renderHome();
      expect(m.sourceService.getArrivals).toHaveBeenCalledWith(14);
    });
  });

  describe('Processing pipeline', () => {
    it('splits the day into done, processing, failed and queued, with done out of the total', async () => {
      m.documentService.getFailedOcrDocuments.mockResolvedValue({ data: { documents: [], pagination: { total: 3 } } });
      await renderHome();
      const card = region('Processing pipeline');
      expect(
        await within(card).findByRole('img', { name: 'Processing pipeline: Done today 41, Processing 4, Failed 3, Queued 12' }),
      ).toBeInTheDocument();
      expect(within(card).getByText('41 / 60')).toBeInTheDocument();
      expect(within(card).getByText('oldest waiting 2h 10m')).toBeInTheDocument();
    });

    it('lets an admin pause OCR', async () => {
      const user = userEvent.setup();
      await renderHome('admin');
      await user.click(await within(region('Processing pipeline')).findByRole('button', { name: 'Pause OCR' }));
      expect(m.queueService.pauseOcr).toHaveBeenCalled();
    });

    it('shows paused OCR and lets an admin resume it', async () => {
      const user = userEvent.setup();
      m.queueService.getOcrStatus.mockResolvedValue({ data: { is_paused: true } });
      await renderHome('admin');
      expect(await within(region('Processing pipeline')).findByText('OCR is paused')).toBeInTheDocument();
      await user.click(within(region('Processing pipeline')).getByRole('button', { name: 'Resume OCR' }));
      expect(m.queueService.resumeOcr).toHaveBeenCalled();
    });

    it('tells the admin when pausing fails', async () => {
      const user = userEvent.setup();
      m.queueService.pauseOcr.mockRejectedValue(new Error('no'));
      await renderHome('admin');
      await user.click(await within(region('Processing pipeline')).findByRole('button', { name: 'Pause OCR' }));
      expect(await screen.findByText('Could not pause OCR')).toBeInTheDocument();
    });

    it('hides pause and resume from non-admins', async () => {
      await renderHome('user');
      await within(region('Processing pipeline')).findByText('41 / 57');
      expect(screen.queryByRole('button', { name: 'Pause OCR' })).not.toBeInTheDocument();
    });

    it('leaves the card out when the queue is admin-only and nothing failed', async () => {
      m.queueService.getStats.mockRejectedValue({ response: { status: 403 } });
      await renderHome('user');
      await waitFor(() => expect(screen.queryByRole('region', { name: 'Processing pipeline' })).not.toBeInTheDocument());
      expect(within(summary()).queryByText(/pending|waiting/)).not.toBeInTheDocument();
      expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    });

    it('shows only the failures to a user who cannot see the queue', async () => {
      m.queueService.getStats.mockRejectedValue({ response: { status: 403 } });
      m.documentService.getFailedOcrDocuments.mockResolvedValue({ data: { documents: [], pagination: { total: 2 } } });
      await renderHome('user');
      expect(
        await within(region('Processing pipeline')).findByRole('img', { name: 'Processing pipeline: Failed 2' }),
      ).toBeInTheDocument();
    });

    it('shows an inline error with Retry when the queue cannot be loaded', async () => {
      const user = userEvent.setup();
      m.queueService.getStats.mockRejectedValue(new Error('boom'));
      await renderHome();
      const alert = await within(region('Processing pipeline')).findByRole('alert');
      expect(alert).toHaveTextContent('The OCR queue could not be loaded.');
      m.queueService.getStats.mockResolvedValue({ data: STATS });
      await user.click(within(alert).getByRole('button', { name: 'Retry' }));
      expect(await within(region('Processing pipeline')).findByText('41 / 57')).toBeInTheDocument();
    });

    it('shows an error when the OCR state cannot be loaded', async () => {
      m.queueService.getOcrStatus.mockRejectedValue(new Error('boom'));
      await renderHome();
      expect(await within(region('Processing pipeline')).findByText('The OCR state could not be loaded.')).toBeInTheDocument();
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

  describe('Needs attention', () => {
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

    it('names the failures and their main causes in plain words, with one Review action', async () => {
      m.documentService.getFailedOcrDocuments.mockResolvedValue(failures());
      await renderHome();
      const attention = await screen.findByRole('region', { name: /Needs attention/ });
      expect(within(attention).getByText('190 documents failed OCR')).toBeInTheDocument();
      expect(within(attention).getByText("OCR failed · Can't read .doc files: install antiword or catdoc")).toBeInTheDocument();
      expect(screen.getAllByRole('link', { name: 'Review' })).toHaveLength(1);
      expect(within(attention).getByRole('link', { name: 'Review' })).toHaveAttribute('href', '/intake?section=attention');
      expect(screen.queryByText(/\/app\/uploads/)).not.toBeInTheDocument();
    });

    it('is left out when nothing failed', async () => {
      await renderHome();
      expect(screen.queryByRole('region', { name: /Needs attention/ })).not.toBeInTheDocument();
    });

    it('shows an error with Retry when failed documents cannot be loaded', async () => {
      const user = userEvent.setup();
      m.documentService.getFailedOcrDocuments.mockRejectedValue(new Error('boom'));
      await renderHome();
      const attention = await screen.findByRole('region', { name: /Needs attention/ });
      const alert = await within(attention).findByRole('alert');
      expect(alert).toHaveTextContent('Failed documents could not be loaded.');
      m.documentService.getFailedOcrDocuments.mockResolvedValue(failures());
      await user.click(within(alert).getByRole('button', { name: 'Retry' }));
      expect(await screen.findByText('190 documents failed OCR')).toBeInTheDocument();
    });
  });

  describe('Just arrived', () => {
    const cards = () => within(screen.getByRole('list', { name: 'Just arrived' })).getAllByRole('listitem');
    const card = (name: string) => cards().find((c) => within(c).queryByRole('link', { name: new RegExp(name) }))!;

    it('shows the newest documents with type, size, age, OCR status and source', async () => {
      await renderHome();
      expect(within(card('d1.pdf')).getByText('Scanner inbox')).toBeInTheDocument();
      expect(within(card('d2.pdf')).getByText('Uploads')).toBeInTheDocument();
      expect(within(card('d1.pdf')).getByText(/^PDF · 2\.0 KB · .*(5 min|5m)/)).toBeInTheDocument();
      expect(within(card('d1.pdf')).getByText('Indexed')).toBeInTheDocument();
      expect(card('d1.pdf').querySelector('[data-tile] svg')).not.toBeNull();
      expect(within(card('d1.pdf')).getByRole('link')).toHaveAttribute('href', '/home?document=d1');
      expect(m.documentService.listWithPagination).toHaveBeenCalledWith(12, 0);
    });

    it('shows OCR progress with a spinner and the first labels', async () => {
      m.documentService.listWithPagination.mockResolvedValue({
        data: {
          documents: [
            doc('d1', {
              ocr_status: 'processing',
              ocr_progress_current: 3,
              ocr_progress_total: 14,
              labels: [
                { id: 'l1', name: 'Housing', color: '#4878B8' },
                { id: 'l2', name: 'Taxes', color: '#C0841A' },
                { id: 'l3', name: 'Misc', color: '#888888' },
              ],
            }),
          ],
          pagination: { total: 1 },
        },
      });
      await renderHome();
      expect(within(card('d1.pdf')).getByText('OCR 3/14')).toBeInTheDocument();
      expect(card('d1.pdf').querySelector('[data-lead] svg')).not.toBeNull();
      expect(within(card('d1.pdf')).getByText('Housing')).toBeInTheDocument();
      expect(within(card('d1.pdf')).getByLabelText('1 more')).toHaveTextContent('+1');
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
      expect(loc()).toBe('/home?document=d1');
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
      expect(within(recent).getByRole('link', { name: 'Upload' })).toHaveAttribute('href', '/intake?section=upload');
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
