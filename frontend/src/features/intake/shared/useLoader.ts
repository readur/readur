import { useCallback, useEffect, useRef, useState } from 'react';

export interface Loaded<T> {
  data: T | undefined;
  error: unknown;
  isLoading: boolean;
  /** Fetch again, keeping the current data visible while loading. */
  reload: () => Promise<void>;
  setData: (update: T | ((prev: T | undefined) => T)) => void;
}

/**
 * Runs `load` on mount and whenever `deps` change. Only the latest request may update state,
 * so a slow earlier response can't overwrite a newer one.
 */
export function useLoader<T>(load: () => Promise<T>, deps: readonly unknown[], enabled = true): Loaded<T> {
  const [data, setDataState] = useState<T | undefined>(undefined);
  const [error, setError] = useState<unknown>(null);
  const [isLoading, setLoading] = useState(enabled);
  const seq = useRef(0);
  const loadRef = useRef(load);
  loadRef.current = load;

  const run = useCallback(async () => {
    const id = ++seq.current;
    setLoading(true);
    try {
      const result = await loadRef.current();
      if (id === seq.current) {
        setDataState(result);
        setError(null);
      }
    } catch (err) {
      if (id === seq.current) setError(err ?? new Error('Request failed'));
    } finally {
      if (id === seq.current) setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (!enabled) {
      setLoading(false);
      return;
    }
    void run();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [enabled, run, ...deps]);

  useEffect(
    () => () => {
      seq.current += 1;
    },
    [],
  );

  const setData = useCallback((update: T | ((prev: T | undefined) => T)) => {
    setDataState((prev) => (typeof update === 'function' ? (update as (p: T | undefined) => T)(prev) : update));
  }, []);

  return { data, error, isLoading, reload: run, setData };
}
