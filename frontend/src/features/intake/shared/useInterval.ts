import { useEffect, useRef } from 'react';

/** Calls the latest `fn` every `ms` while mounted. */
export function useInterval(fn: () => void, ms: number): void {
  const latest = useRef(fn);
  latest.current = fn;
  useEffect(() => {
    const id = window.setInterval(() => latest.current(), ms);
    return () => window.clearInterval(id);
  }, [ms]);
}
