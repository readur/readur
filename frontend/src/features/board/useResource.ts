import { useCallback, useEffect, useRef, useState } from 'react';

export interface Resource<T> {
  data: T | undefined;
  /** True until the first response (success or failure) arrives. */
  loading: boolean;
  error: unknown;
  reload: () => void;
}

/**
 * Loads `fetcher` on mount and, with `intervalMs`, again on a timer while the tab is visible
 * (and once when it becomes visible again). A failed refresh keeps the last data but reports the
 * error. `fetcher` is read through a ref, so an inline function is fine.
 */
export function useResource<T>(fetcher: () => Promise<T>, intervalMs?: number): Resource<T> {
  const [state, setState] = useState<{ data?: T; loading: boolean; error: unknown }>({
    loading: true,
    error: undefined,
  });
  const fetchRef = useRef(fetcher);
  fetchRef.current = fetcher;
  const alive = useRef(true);

  const run = useCallback(() => {
    fetchRef
      .current()
      .then((data) => alive.current && setState({ data, loading: false, error: undefined }))
      .catch((error) => alive.current && setState((s) => ({ data: s.data, loading: false, error })));
  }, []);

  useEffect(() => {
    alive.current = true;
    run();
    let timer: number | undefined;
    const visible = () => typeof document === 'undefined' || document.visibilityState !== 'hidden';
    const onVisibility = () => visible() && run();
    if (intervalMs) {
      timer = window.setInterval(() => visible() && run(), intervalMs);
      document.addEventListener('visibilitychange', onVisibility);
    }
    return () => {
      alive.current = false;
      if (timer !== undefined) window.clearInterval(timer);
      document.removeEventListener('visibilitychange', onVisibility);
    };
  }, [run, intervalMs]);

  const reload = useCallback(() => {
    setState((s) => ({ ...s, loading: s.data === undefined, error: undefined }));
    run();
  }, [run]);

  return { data: state.data, loading: state.loading, error: state.error, reload };
}
