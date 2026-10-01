import { describe, expect, it } from 'vitest';
import { act, renderHook, waitFor } from '@testing-library/react';
import { useResource } from '../useResource';

function deferred<T>() {
  let resolve!: (v: T) => void;
  let reject!: (e: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

describe('useResource', () => {
  it('never lets an older response overwrite newer data', async () => {
    const first = deferred<string>();
    const second = deferred<string>();
    const queue = [first, second];
    const { result } = renderHook(() => useResource(() => queue.shift()!.promise));

    act(() => result.current.reload());
    await act(async () => second.resolve('new'));
    expect(result.current.data).toBe('new');

    await act(async () => first.resolve('old'));
    expect(result.current.data).toBe('new');
    expect(result.current.loading).toBe(false);
  });

  it('ignores a late failure from an older request', async () => {
    const first = deferred<string>();
    const second = deferred<string>();
    const queue = [first, second];
    const { result } = renderHook(() => useResource(() => queue.shift()!.promise));

    act(() => result.current.reload());
    await act(async () => second.resolve('fresh'));
    await act(async () => first.reject(new Error('stale')));
    expect(result.current.data).toBe('fresh');
    expect(result.current.error).toBeUndefined();
  });

  it('applies the latest response and keeps data when a refresh fails', async () => {
    let n = 0;
    const { result } = renderHook(() =>
      useResource(() => (n++ === 0 ? Promise.resolve('one') : Promise.reject(new Error('down')))),
    );
    await waitFor(() => expect(result.current.data).toBe('one'));
    act(() => result.current.reload());
    await waitFor(() => expect(result.current.error).toBeInstanceOf(Error));
    expect(result.current.data).toBe('one');
  });
});
