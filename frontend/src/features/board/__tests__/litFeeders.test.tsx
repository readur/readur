import { act, render, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const m = vi.hoisted(() => ({
  documentService: { listWithPagination: vi.fn() },
}));

vi.mock('../../../services/api', () => ({
  default: {},
  documentService: m.documentService,
}));

import { NotificationProvider, useNotifications } from '../../../contexts/NotificationContext';
import { resetDocumentBaseline, syncDocuments, useLitFeeders } from '../litFeeders';
import { BULK_ARRIVALS_KEY, BULK_THRESHOLD } from '../litFeeders';
import { isLit, litReason, useLit } from '../litStore';
import { doc, resetBoardState } from './boardTestUtils';

type AddFn = ReturnType<typeof useNotifications>['addNotification'];
let add: AddFn;

function Harness() {
  const { addNotification } = useNotifications();
  add = addNotification;
  useLitFeeders();
  const state = useLit('document', '42');
  return <output aria-label="lit">{state.lit ? state.reason : 'off'}</output>;
}

const mount = () =>
  render(
    <NotificationProvider>
      <Harness />
    </NotificationProvider>,
  );

beforeEach(() => {
  resetBoardState();
  vi.clearAllMocks();
  m.documentService.listWithPagination.mockResolvedValue({ data: { documents: [], pagination: { total: 0 } } });
});

describe('useLitFeeders', () => {
  it('marks the document named by a success notification as changed', async () => {
    const view = mount();
    act(() => add({ type: 'success', title: 'OCR Complete', message: 'done', metadata: { documentId: 42 } }));
    await waitFor(() => expect(view.getByLabelText('lit')).toHaveTextContent('changed'));
    expect(isLit('document', '42')).toBe(true);
    expect(m.documentService.listWithPagination).not.toHaveBeenCalled();
  });

  it('marks the document named by an error notification as failed', async () => {
    const view = mount();
    act(() => add({ type: 'error', title: 'OCR Failed', message: 'nope', metadata: { documentId: 42 } }));
    await waitFor(() => expect(view.getByLabelText('lit')).toHaveTextContent('failed'));
  });

  it('diffs the newest documents when a notification carries no id', async () => {
    m.documentService.listWithPagination.mockResolvedValue({
      data: { documents: [doc('42', { created_at: new Date().toISOString() }), doc('old', { created_at: '2020-01-01T00:00:00Z' })], pagination: { total: 2 } },
    });
    const view = mount();
    act(() => add({ type: 'success', title: 'File Uploaded', message: 'a.pdf uploaded successfully' }));
    await waitFor(() => expect(view.getByLabelText('lit')).toHaveTextContent('new'));
    expect(m.documentService.listWithPagination).toHaveBeenCalledWith(10, 0);
    expect(isLit('document', 'old')).toBe(false);
  });

  it('ignores a failing refresh', async () => {
    m.documentService.listWithPagination.mockRejectedValue(new Error('down'));
    const view = mount();
    act(() => add({ type: 'success', title: 'File Uploaded', message: 'x' }));
    await waitFor(() => expect(m.documentService.listWithPagination).toHaveBeenCalled());
    expect(view.getByLabelText('lit')).toHaveTextContent('off');
  });

  it('does nothing until a notification arrives', () => {
    mount();
    expect(m.documentService.listWithPagination).not.toHaveBeenCalled();
  });
});

describe('syncDocuments', () => {
  beforeEach(() => resetDocumentBaseline());

  it('only records a baseline the first time', () => {
    syncDocuments([doc('a'), doc('b')]);
    expect(isLit('document', 'a')).toBe(false);
    expect(isLit('document', 'b')).toBe(false);
  });

  it('marks unseen documents as new after the baseline', () => {
    syncDocuments([doc('a')]);
    syncDocuments([doc('b', { created_at: new Date().toISOString() }), doc('a')]);
    expect(isLit('document', 'b')).toBe(true);
    expect(isLit('document', 'a')).toBe(false);
  });

  it('does not mark an older document that only entered the list after another was deleted', () => {
    syncDocuments([doc('a'), doc('b', { created_at: '2020-01-01T00:00:00Z' })].slice(0, 1));
    syncDocuments([doc('a'), doc('older', { created_at: '2021-01-01T00:00:00Z' })]);
    expect(isLit('document', 'older')).toBe(false);
  });

  it('with a stored last-seen time, the first sync marks only newer documents and stores the new time', () => {
    const t0 = Date.parse('2026-01-01T10:00:00Z');
    window.localStorage.setItem('readur.board.lastSeen.v1', String(t0));
    syncDocuments([
      doc('newer', { created_at: '2026-01-01T11:00:00Z' }),
      doc('same', { created_at: '2026-01-01T10:00:00Z' }),
      doc('older', { created_at: '2025-12-31T10:00:00Z' }),
    ]);
    expect(isLit('document', 'newer')).toBe(true);
    expect(isLit('document', 'same')).toBe(false);
    expect(isLit('document', 'older')).toBe(false);
    expect(window.localStorage.getItem('readur.board.lastSeen.v1')).toBe(String(Date.parse('2026-01-01T11:00:00Z')));
  });

  it('without a stored time, the first sync marks nothing but stores a baseline time', () => {
    syncDocuments([doc('a', { created_at: '2026-01-01T11:00:00Z' })]);
    expect(isLit('document', 'a')).toBe(false);
    expect(window.localStorage.getItem('readur.board.lastSeen.v1')).toBe(String(Date.parse('2026-01-01T11:00:00Z')));
  });

  it('survives unavailable storage', () => {
    const broken = { getItem: () => { throw new Error('x'); }, setItem: () => { throw new Error('x'); } };
    Object.defineProperty(window, 'localStorage', { value: broken, configurable: true, writable: true });
    expect(() => syncDocuments([doc('a')])).not.toThrow();
  });

  it('marks a finished OCR as changed and a failed one as failed, once', () => {
    syncDocuments([doc('a', { ocr_status: 'processing' }), doc('b', { ocr_status: 'processing' }), doc('c', { ocr_status: 'pending' })]);
    syncDocuments([doc('a', { ocr_status: 'completed' }), doc('b', { ocr_status: 'failed' }), doc('c', { ocr_status: 'processing' })]);
    expect(isLit('document', 'a')).toBe(true);
    expect(isLit('document', 'b')).toBe(true);
    expect(isLit('document', 'c')).toBe(false);
  });

  it('treats unseen documents created after `newerThan` as new even on the first call', () => {
    const now = Date.now();
    syncDocuments([doc('fresh', { created_at: new Date(now).toISOString() }), doc('stale', { created_at: new Date(now - 3600_000).toISOString() })], now - 1000);
    expect(isLit('document', 'fresh')).toBe(true);
    expect(isLit('document', 'stale')).toBe(false);
  });
  it('keeps an unseen arrival NEW when its OCR then finishes, but a failure turns it CHANGED', () => {
    const now = Date.now();
    syncDocuments([doc('old', { created_at: new Date(now - 3600_000).toISOString() })]);
    syncDocuments([
      doc('a', { created_at: new Date(now).toISOString(), ocr_status: 'processing' }),
      doc('b', { created_at: new Date(now).toISOString(), ocr_status: 'processing' }),
    ]);
    syncDocuments([
      doc('a', { created_at: new Date(now).toISOString(), ocr_status: 'completed' }),
      doc('b', { created_at: new Date(now).toISOString(), ocr_status: 'failed' }),
    ]);
    expect(litReason('document', 'a')).toBe('new');
    expect(litReason('document', 'b')).toBe('failed');
  });

  it('marks an arrival whose OCR already failed as failed, so it reads CHANGED like Needs attention', () => {
    const now = Date.now();
    syncDocuments([doc('old', { created_at: new Date(now - 3600_000).toISOString() })]);
    syncDocuments([doc('bad', { created_at: new Date(now).toISOString(), ocr_status: 'failed' })]);
    expect(litReason('document', 'bad')).toBe('failed');
  });

  it(`raises one summary instead of lighting rows when more than ${BULK_THRESHOLD} documents arrive at once`, () => {
    const now = Date.now();
    syncDocuments([doc('old', { created_at: new Date(now - 3600_000).toISOString() })], undefined, 1);
    const batch = Array.from({ length: 10 }, (_, i) =>
      doc(`n${i}`, { created_at: new Date(now - i * 1000).toISOString(), ocr_status: 'processing' }),
    );
    syncDocuments(batch, undefined, 1 + 40);
    batch.forEach((d) => expect(isLit('document', d.id)).toBe(false));
    expect(window.localStorage.getItem(BULK_ARRIVALS_KEY)).toBe('40');
    // Their OCR finishing later does not light them one by one either.
    syncDocuments(batch.map((d) => ({ ...d, ocr_status: 'completed' })), undefined, 41);
    batch.forEach((d) => expect(isLit('document', d.id)).toBe(false));
  });

  it(`lights a batch of ${BULK_THRESHOLD} or fewer as individual NEW rows`, () => {
    const now = Date.now();
    syncDocuments([doc('old', { created_at: new Date(now - 3600_000).toISOString() })], undefined, 1);
    syncDocuments([doc('n1', { created_at: new Date(now).toISOString() })], undefined, 1 + BULK_THRESHOLD);
    expect(litReason('document', 'n1')).toBe('new');
    expect(window.localStorage.getItem(BULK_ARRIVALS_KEY)).toBeNull();
  });
});
