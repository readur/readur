import { act, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { StrictMode } from 'react';
import { I18nextProvider } from 'react-i18next';
import i18n from 'i18next';
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
import { ChangedTag as BoardTag } from '../Region';
import { ChangeTag as LibraryTag } from '../../library/cells';
import { LibraryTable } from '../../library/LibraryTable';
import { ChangedTag as IntakeTag } from '../../intake/shared/parts';
import { BULK_THRESHOLD, syncDocuments } from '../litFeeders';
import { LIT_SHOWN_CAP, isLit, isShownLit, markLit, useAcknowledgeOnLeave } from '../litStore';
import { doc, renderPage, resetBoardState } from './boardTestUtils';

function serve(documents = [doc('d1'), doc('d2')], total = documents.length) {
  m.documentService.listWithPagination.mockResolvedValue({ data: { documents, pagination: { total } } });
  m.documentService.getFailedOcrDocuments.mockResolvedValue({ data: { documents: [] } });
  m.queueService.getStats.mockResolvedValue({ data: { pending: 4, processing: 1, failed: 99, completed_today: 8, oldest_pending_minutes: 3 } });
  m.queueService.getOcrStatus.mockResolvedValue({ data: { is_paused: false, status: 'running' } });
  m.api.get.mockImplementation((url: string) => {
    if (url === '/sources') return Promise.resolve({ data: [] });
    if (url === '/metrics') return Promise.resolve({ data: { documents: { total_documents: 2, total_storage_bytes: 4096, documents_with_ocr: 2 } } });
    if (url.startsWith('/labels')) return Promise.resolve({ data: [] });
    return Promise.reject(new Error(`unexpected ${url}`));
  });
}

async function renderBoard() {
  const view = renderPage(<Board />);
  await screen.findByText('d1.pdf');
  return view;
}

/** Lets the deferred acknowledge-on-leave (a microtask after unmount) run. */
const settle = () => act(async () => {});

beforeEach(() => {
  resetBoardState();
  vi.clearAllMocks();
  serve();
});

afterEach(() => {
  vi.useRealTimers();
});

describe('changed state: acknowledge on leave', () => {
  it('keeps a lit row lit for the whole visit and acknowledges it when the user leaves the Board', async () => {
    markLit('document', 'd1', 'new');
    const view = await renderBoard();
    expect(screen.getByRole('row', { name: /d1\.pdf/ })).toHaveAttribute('data-changed', 'true');
    // Still flagged for the rest of the visit.
    await settle();
    expect(isLit('document', 'd1')).toBe(true);
    expect(screen.getByRole('row', { name: /d1\.pdf/ })).toHaveAttribute('data-changed', 'true');
    view.unmount();
    await settle();
    expect(isLit('document', 'd1')).toBe(false);
  });

  it('only acknowledges rows that were on screen and lit, not other lit documents', async () => {
    markLit('document', 'd1', 'changed');
    markLit('document', 'elsewhere', 'new');
    const view = await renderBoard();
    view.unmount();
    await settle();
    expect(isLit('document', 'd1')).toBe(false);
    expect(isLit('document', 'elsewhere')).toBe(true);
  });

  it('acknowledges seen Needs attention rows on leave, and a seen failure is not lit again next visit', async () => {
    const failedAt = '2026-01-01T10:00:00Z';
    m.documentService.getFailedOcrDocuments.mockResolvedValue({
      data: { documents: [{ id: 'f1', filename: 'scan.tiff', failure_reason: 'x', updated_at: failedAt }], pagination: { total: 1 } },
    });
    const first = await renderBoard();
    const row = await screen.findByRole('row', { name: /scan\.tiff/ });
    expect(row).toHaveAttribute('data-changed', 'true');
    first.unmount();
    await settle();
    expect(isLit('attention', `document:f1@${failedAt}`)).toBe(false);

    await renderBoard();
    const again = await screen.findByRole('row', { name: /scan\.tiff/ });
    expect(again).not.toHaveAttribute('data-changed');
  });

  it('acknowledges the Library rows seen lit when the user leaves the Library', async () => {
    markLit('document', 'L1', 'changed');
    markLit('document', 'other', 'new');
    const row = (id: string) =>
      ({ id, filename: `${id}.pdf`, original_filename: `${id}.pdf`, mime_type: 'application/pdf', file_size: 1, created_at: '2026-01-01T00:00:00Z', updated_at: '2026-01-01T00:00:00Z', labels: [], ocr_status: 'completed' }) as never;
    const view = render(
      <I18nextProvider i18n={i18n}>
        <LibraryTable
          rows={[row('L1'), row('L2')]}
          sort={undefined}
          onSortChange={() => {}}
          selectedKeys={new Set()}
          onSelectionChange={() => {}}
          onOpen={() => {}}
          sourceName={() => 'Upload'}
          showSnippets={false}
          compact={false}
          isLoading={false}
          emptyState={null}
          litVersion={0}
        />
      </I18nextProvider>,
    );
    const lit = screen.getByRole('row', { name: /L1\.pdf/ });
    expect(lit).toHaveAttribute('data-changed', 'true');
    expect(within(lit).getByText('CHANGED')).toBeInTheDocument();
    view.unmount();
    await settle();
    expect(isLit('document', 'L1')).toBe(false);
    expect(isLit('document', 'other')).toBe(true);
  });

  it('survives the development double mount (StrictMode) without clearing rows early', async () => {
    markLit('document', 'x', 'new');
    function Probe() {
      useAcknowledgeOnLeave('document', ['x']);
      return null;
    }
    const view = render(
      <StrictMode>
        <Probe />
      </StrictMode>,
    );
    await settle();
    expect(isLit('document', 'x')).toBe(true);
    view.unmount();
    await settle();
    expect(isLit('document', 'x')).toBe(false);
  });

  it('offers "Mark all seen" while lit rows show, and it clears them', async () => {
    const user = userEvent.setup();
    markLit('document', 'd1', 'new');
    await renderBoard();
    await user.click(screen.getByRole('button', { name: 'Mark all seen' }));
    expect(isLit('document', 'd1')).toBe(false);
    await waitFor(() => expect(screen.getByRole('row', { name: /d1\.pdf/ })).not.toHaveAttribute('data-changed'));
    expect(screen.queryByRole('button', { name: 'Mark all seen' })).not.toBeInTheDocument();
  });

  it('has no "Mark all seen" when nothing is lit', async () => {
    await renderBoard();
    expect(screen.queryByRole('button', { name: 'Mark all seen' })).not.toBeInTheDocument();
  });
});

describe('changed state: one vocabulary', () => {
  const word = (el: HTMLElement) => el.textContent?.trim().toUpperCase();

  function renderTags(reason: 'new' | 'changed' | 'failed') {
    markLit('document', 'x', reason);
    // Each surface's tag rendered on its own, read as the text it shows.
    return [<BoardTag key="board" reason={reason} />, <LibraryTag key="library" id="x" />, <IntakeTag key="intake" reason={reason} />].map(
      (tag) => word(render(<I18nextProvider i18n={i18n}>{tag}</I18nextProvider>).container),
    );
  }

  it.each([
    ['new', 'NEW'],
    ['changed', 'CHANGED'],
    ['failed', 'CHANGED'],
  ] as const)('the same %s event reads %s on the Board, the Library and Intake', (reason, expected) => {
    expect(renderTags(reason)).toEqual([expected, expected, expected]);
  });

  it('a failed document reads CHANGED in Arrivals and in Needs attention alike', async () => {
    const failedAt = '2026-01-01T10:00:00Z';
    markLit('document', 'd1', 'failed');
    serve([doc('d1', { ocr_status: 'failed', has_ocr_text: false })]);
    m.documentService.getFailedOcrDocuments.mockResolvedValue({
      data: { documents: [{ id: 'd1', filename: 'd1.pdf', failure_reason: 'x', updated_at: failedAt }], pagination: { total: 1 } },
    });
    renderPage(<Board />);
    const attention = await screen.findByRole('region', { name: 'Needs attention' });
    const arrivals = screen.getByRole('region', { name: 'Arrivals' });
    await within(arrivals).findByText('d1.pdf');
    expect(within(within(attention).getByRole('row', { name: /d1\.pdf/ })).getByText('Changed')).toBeInTheDocument();
    expect(within(within(arrivals).getByRole('row', { name: /d1\.pdf/ })).getByText('Changed')).toBeInTheDocument();
  });
});

describe('changed state: first run and bulk imports', () => {
  it(`never shows more than ${LIT_SHOWN_CAP} lit rows of a kind: the newest`, () => {
    for (let i = 0; i < 40; i += 1) markLit('document', `d${i}`, 'new');
    const shown = Array.from({ length: 40 }, (_, i) => isShownLit('document', `d${i}`));
    expect(shown.filter(Boolean)).toHaveLength(LIT_SHOWN_CAP);
    expect(isShownLit('document', 'd39')).toBe(true);
    expect(isShownLit('document', 'd14')).toBe(false);
    expect(isShownLit('document', 'd15')).toBe(true);
  });

  it(`a bulk import of more than ${BULK_THRESHOLD} lights no rows, only an "N new documents" summary`, async () => {
    const now = Date.now();
    const old = new Date(now - 3600_000).toISOString();
    syncDocuments([doc('a', { created_at: old })], undefined, 1);
    const fresh = Array.from({ length: 10 }, (_, i) => doc(`n${i}`, { created_at: new Date(now - i * 1000).toISOString() }));
    serve(fresh, 121);
    renderPage(<Board />);
    await screen.findByText('n0.pdf');
    const arrivals = screen.getByRole('region', { name: 'Arrivals' });
    const summary = await within(arrivals).findByRole('status');
    expect(summary).toHaveTextContent('120 new documents');
    expect(within(summary).getByRole('link', { name: 'Open in Library, newest first' })).toHaveAttribute(
      'href',
      '/documents?sort=created_at&order=desc',
    );
    const lit = within(arrivals)
      .getAllByRole('row')
      .filter((r) => r.getAttribute('data-changed') === 'true');
    expect(lit).toHaveLength(0);
    fresh.forEach((d) => expect(isLit('document', d.id)).toBe(false));
  });

  it(`a small batch (${BULK_THRESHOLD} or fewer) lights its rows as NEW`, async () => {
    const now = Date.now();
    syncDocuments([doc('a', { created_at: new Date(now - 3600_000).toISOString() })], undefined, 1);
    serve([doc('d1', { created_at: new Date(now).toISOString() }), doc('a')], 2);
    await renderBoard();
    const row = await screen.findByRole('row', { name: /d1\.pdf/ });
    expect(row).toHaveAttribute('data-changed', 'true');
    expect(within(row).getByText('New')).toBeInTheDocument();
    expect(screen.queryByText(/new documents/)).not.toBeInTheDocument();
  });
});

describe('Board truth: queue figures', () => {
  it('reads the queue stats the server actually sends (pending / processing)', async () => {
    await renderBoard();
    const group = await within(screen.getByRole('region', { name: 'Processing' })).findByRole('group', { name: 'Processing' });
    const value = (label: string) => within(group).getByText(label).closest('div')?.querySelector('dd')?.textContent;
    expect(value('Pending')).toBe('4');
    expect(value('Processing')).toBe('1');
    // The queue's 99 failed jobs are not shown: FAILED is the failed-documents count.
    expect(value('Failed')).toBe('0');
  });

  it('shows dashes, not an error, when the queue figures are admin-only', async () => {
    m.queueService.getStats.mockRejectedValue({ response: { status: 403 } });
    await renderBoard();
    const processing = screen.getByRole('region', { name: 'Processing' });
    const group = await within(processing).findByRole('group', { name: 'Processing' });
    expect(within(processing).queryByRole('alert')).not.toBeInTheDocument();
    expect(within(group).getByText('Pending').closest('div')?.querySelector('dd')?.textContent).toBe('—');
  });
});
