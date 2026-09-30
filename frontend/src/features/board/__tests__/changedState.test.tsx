import { act, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { StrictMode } from 'react';
import { I18nextProvider } from 'react-i18next';
import i18n from 'i18next';
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

import Home from '../Home';
import { ChangeTag } from '../../../ui';
import { ChangedTag as HomeTag } from '../Region';
import { ChangeTag as LibraryTag } from '../../library/cells';
import { LibraryTable } from '../../library/LibraryTable';
import { ChangedTag as IntakeTag } from '../../intake/shared/parts';
import { BULK_THRESHOLD, syncDocuments } from '../litFeeders';
import { LIT_SHOWN_CAP, isLit, isShownLit, markLit, useAcknowledgeOnLeave } from '../litStore';
import { doc, renderPage, resetBoardState } from './homeTestUtils';

function serve(documents = [doc('d1'), doc('d2')], total = documents.length) {
  m.documentService.listWithPagination.mockResolvedValue({ data: { documents, pagination: { total } } });
  m.documentService.getFailedOcrDocuments.mockResolvedValue({ data: { documents: [] } });
  m.documentService.getThumbnail.mockRejectedValue(new Error('none'));
  m.queueService.getStats.mockResolvedValue({ data: { pending: 4, processing: 1, failed: 99, completed_today: 8, oldest_pending_minutes: 3 } });
  m.queueService.getOcrStatus.mockResolvedValue({ data: { is_paused: false, status: 'running' } });
  m.sourceService.getArrivals.mockResolvedValue({ data: [] });
}

async function renderHome() {
  const view = renderPage(<Home />);
  await screen.findByText('d1.pdf');
  return view;
}

/** The Just arrived card of a document. */
const card = (name: string) => screen.getByText(name).closest('li') as HTMLElement;

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
  it('keeps a new item marked for the whole visit and acknowledges it when the user leaves Home', async () => {
    markLit('document', 'd1', 'new');
    const view = await renderHome();
    expect(card('d1.pdf')).toHaveAttribute('data-changed', 'true');
    // Still flagged for the rest of the visit.
    await settle();
    expect(isLit('document', 'd1')).toBe(true);
    expect(card('d1.pdf')).toHaveAttribute('data-changed', 'true');
    view.unmount();
    await settle();
    expect(isLit('document', 'd1')).toBe(false);
  });

  it('only acknowledges items that were on screen and marked, not other marked documents', async () => {
    markLit('document', 'd1', 'changed');
    markLit('document', 'elsewhere', 'new');
    const view = await renderHome();
    view.unmount();
    await settle();
    expect(isLit('document', 'd1')).toBe(false);
    expect(isLit('document', 'elsewhere')).toBe(true);
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

  it('offers "Mark all seen" while marked items show, and it clears them', async () => {
    const user = userEvent.setup();
    markLit('document', 'd1', 'new');
    await renderHome();
    await user.click(screen.getByRole('button', { name: 'Mark all seen' }));
    expect(isLit('document', 'd1')).toBe(false);
    await waitFor(() => expect(card('d1.pdf')).not.toHaveAttribute('data-changed'));
    expect(screen.queryByRole('button', { name: 'Mark all seen' })).not.toBeInTheDocument();
  });

  it('moves focus to the page heading when "Mark all seen" removes itself', async () => {
    const user = userEvent.setup();
    markLit('document', 'd1', 'new');
    await renderHome();
    screen.getByRole('button', { name: 'Mark all seen' }).focus();
    await user.keyboard('{Enter}');
    await waitFor(() => expect(screen.queryByRole('button', { name: 'Mark all seen' })).not.toBeInTheDocument());
    expect(screen.getByRole('heading', { level: 1 })).toHaveFocus();
  });

  it('has no "Mark all seen" when nothing is lit', async () => {
    await renderHome();
    expect(screen.queryByRole('button', { name: 'Mark all seen' })).not.toBeInTheDocument();
  });
});

describe('changed state: one vocabulary', () => {
  const word = (el: HTMLElement) => el.textContent?.trim().toUpperCase();

  function renderTags(reason: 'new' | 'changed' | 'failed') {
    markLit('document', 'x', reason);
    // Each surface's tag rendered on its own, read as the text it shows.
    return [<HomeTag key="board" reason={reason} />, <LibraryTag key="library" id="x" />, <IntakeTag key="intake" reason={reason} />].map(
      (tag) => word(render(<I18nextProvider i18n={i18n}>{tag}</I18nextProvider>).container),
    );
  }

  it.each([
    ['new', 'NEW'],
    ['changed', 'CHANGED'],
    ['failed', 'CHANGED'],
  ] as const)('the same %s event reads %s on Home, the Library and Intake', (reason, expected) => {
    expect(renderTags(reason)).toEqual([expected, expected, expected]);
  });

  it('draws the tag with one shared style on Home, the Library and Intake', () => {
    markLit('document', 'x', 'new');
    const shared = render(<ChangeTag>New</ChangeTag>).container.firstElementChild!.className;
    const classes = [<HomeTag key="board" reason="new" />, <LibraryTag key="library" id="x" />, <IntakeTag key="intake" reason="new" />].map(
      (tag) => render(<I18nextProvider i18n={i18n}>{tag}</I18nextProvider>).container.firstElementChild!.className.split(' '),
    );
    for (const list of classes) expect(list).toContain(shared);
  });

  it('a failed document reads Changed in Just arrived', async () => {
    markLit('document', 'd1', 'failed');
    serve([doc('d1', { ocr_status: 'failed', has_ocr_text: false })]);
    await renderHome();
    expect(within(card('d1.pdf')).getByText('Changed')).toBeInTheDocument();
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
    renderPage(<Home />);
    await screen.findByText('n0.pdf');
    const arrivals = screen.getByRole('region', { name: 'Just arrived' });
    const summary = await within(arrivals).findByRole('status');
    expect(summary).toHaveTextContent('120 new documents');
    expect(within(summary).getByRole('link', { name: 'Open in Library, newest first' })).toHaveAttribute(
      'href',
      '/documents?sort=created_at&order=desc',
    );
    const lit = within(within(arrivals).getByRole('list'))
      .getAllByRole('listitem')
      .filter((r) => r.getAttribute('data-changed') === 'true');
    expect(lit).toHaveLength(0);
    fresh.forEach((d) => expect(isLit('document', d.id)).toBe(false));
  });

  it(`a small batch (${BULK_THRESHOLD} or fewer) marks its items as New`, async () => {
    const now = Date.now();
    syncDocuments([doc('a', { created_at: new Date(now - 3600_000).toISOString() })], undefined, 1);
    serve([doc('d1', { created_at: new Date(now).toISOString() }), doc('a')], 2);
    await renderHome();
    await waitFor(() => expect(card('d1.pdf')).toHaveAttribute('data-changed', 'true'));
    expect(within(card('d1.pdf')).getByText('New')).toBeInTheDocument();
    expect(screen.queryByText(/new documents/)).not.toBeInTheDocument();
  });
});

describe('Home truth: queue figures', () => {
  it('reads the queue stats the server actually sends (pending / processing), not its failed jobs', async () => {
    await renderHome();
    const processing = screen.getByRole('region', { name: 'Processing' });
    expect(await within(processing).findByText('Processing 1 · pending 4')).toBeInTheDocument();
    // The queue's 99 failed jobs are not shown: the failures line counts failed documents.
    expect(within(processing).queryByText(/99/)).not.toBeInTheDocument();
    expect(within(processing).getByText('No failed documents')).toBeInTheDocument();
  });
});
