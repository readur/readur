import { act, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { LIBRARY_OCR_POLL_MS, useRows } from '../useLibraryData';
import { parseQuery } from '../urlState';
import { doc, documentService, label, listResponse, setupLibraryMocks } from './serviceMocks';

vi.mock('../../../services/api', async () => (await import('./serviceMocks')).apiModule());
vi.mock('../../../services/api/labels', async () => (await import('./serviceMocks')).labelsModule());

const query = parseQuery(new URLSearchParams());
const tax = label('l-tax', 'Tax');

/** A list response the test resolves when it chooses. */
function deferred() {
  let resolve: (value: unknown) => void = () => {};
  const promise = new Promise((r) => (resolve = r));
  return { promise, resolve };
}

async function startPolling() {
  documentService.listFiltered.mockResolvedValue(listResponse([doc('d1', 'scan.pdf', { ocr_status: 'processing', labels: [tax] })]));
  const hook = renderHook(() => useRows(query, [], true));
  await waitFor(() => expect(hook.result.current.status).toBe('ready'));
  return hook;
}

describe('background OCR polls', () => {
  beforeEach(() => {
    setupLibraryMocks();
    vi.useFakeTimers({ shouldAdvanceTime: true });
  });
  afterEach(() => vi.useRealTimers());

  test('a slow poll that lands after a newer one does not bring back older rows', async () => {
    const { result } = await startPolling();
    const slow = deferred();
    documentService.listFiltered
      .mockImplementationOnce(() => slow.promise)
      .mockResolvedValueOnce(listResponse([doc('d1', 'scan.pdf', { ocr_status: 'completed', labels: [tax] })]));
    await vi.advanceTimersByTimeAsync(LIBRARY_OCR_POLL_MS);
    await vi.advanceTimersByTimeAsync(LIBRARY_OCR_POLL_MS);
    await waitFor(() => expect(result.current.rows[0].ocr_status).toBe('completed'));
    await act(async () => slow.resolve(listResponse([doc('d1', 'scan.pdf', { ocr_status: 'processing', labels: [tax] })])));
    expect(result.current.rows[0].ocr_status).toBe('completed');
  });

  test('a poll that started before a change made in the drawer does not undo it', async () => {
    const { result } = await startPolling();
    const slow = deferred();
    documentService.listFiltered.mockImplementationOnce(() => slow.promise);
    await vi.advanceTimersByTimeAsync(LIBRARY_OCR_POLL_MS);
    act(() => result.current.patchRow('d1', { labels: [] }));
    await act(async () => slow.resolve(listResponse([doc('d1', 'scan.pdf', { ocr_status: 'processing', labels: [tax] })])));
    expect(result.current.rows[0].labels).toEqual([]);
  });
});
