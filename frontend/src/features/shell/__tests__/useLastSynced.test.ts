import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { act, renderHook } from '@testing-library/react';

const get = vi.fn();
vi.mock('../../../services/api', () => ({ default: { get: (...a: unknown[]) => get(...a) } }));

import { SYNC_POLL_MS, useLastSynced } from '../useLastSynced';

const setHidden = (value: boolean) => {
  Object.defineProperty(document, 'hidden', { configurable: true, get: () => value });
  document.dispatchEvent(new Event('visibilitychange'));
};

const flush = () => act(async () => {
  await Promise.resolve();
  await Promise.resolve();
});

describe('useLastSynced', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    get.mockReset();
    get.mockResolvedValue({ data: [{ last_sync_at: '2026-01-02T03:04:05Z' }] });
  });

  afterEach(() => {
    vi.useRealTimers();
    delete (document as { hidden?: boolean }).hidden;
  });

  it('pauses polling while the tab is hidden and loads again when it is shown', async () => {
    const { result } = renderHook(() => useLastSynced());
    await flush();
    expect(get).toHaveBeenCalledTimes(1);
    expect(result.current?.toISOString()).toBe('2026-01-02T03:04:05.000Z');

    await act(async () => setHidden(true));
    await act(async () => vi.advanceTimersByTime(SYNC_POLL_MS * 3));
    expect(get).toHaveBeenCalledTimes(1);

    await act(async () => setHidden(false));
    await flush();
    expect(get).toHaveBeenCalledTimes(2);
    await act(async () => vi.advanceTimersByTime(SYNC_POLL_MS));
    expect(get).toHaveBeenCalledTimes(3);
  });

  it('keeps the same value when the sync time has not changed', async () => {
    const { result } = renderHook(() => useLastSynced());
    await flush();
    const first = result.current;
    await act(async () => vi.advanceTimersByTime(SYNC_POLL_MS));
    await flush();
    expect(get).toHaveBeenCalledTimes(2);
    expect(result.current).toBe(first);

    get.mockResolvedValue({ data: [{ last_sync_at: '2026-01-03T00:00:00Z' }] });
    await act(async () => vi.advanceTimersByTime(SYNC_POLL_MS));
    await flush();
    expect(result.current).not.toBe(first);
    expect(result.current?.toISOString()).toBe('2026-01-03T00:00:00.000Z');
  });
});
