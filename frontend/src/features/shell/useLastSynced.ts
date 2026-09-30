import { useEffect, useState } from 'react';
import api from '../../services/api';

export const SYNC_POLL_MS = 60_000;

interface SourceLike {
  last_sync_at?: string | null;
}

/** Latest `last_sync_at` across a sources list, or null when none has synced. */
export function latestSync(sources: unknown): Date | null {
  if (!Array.isArray(sources)) return null;
  let best: number | null = null;
  for (const s of sources as SourceLike[]) {
    const at = s?.last_sync_at ? Date.parse(s.last_sync_at) : NaN;
    if (!Number.isNaN(at) && (best === null || at > best)) best = at;
  }
  return best === null ? null : new Date(best);
}

/**
 * Fetches the sources list on mount and once a minute, and returns the most recent sync time.
 * Returns null while loading, on error, or when there are no synced sources.
 */
export function useLastSynced(): Date | null {
  const [last, setLast] = useState<Date | null>(null);

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      try {
        const res = await api.get('/sources');
        if (!cancelled) setLast(latestSync(res?.data));
      } catch {
        /* keep the previous value: a failed poll is not a reason to hide the readout */
      }
    };
    void load();
    const timer = window.setInterval(load, SYNC_POLL_MS);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, []);

  return last;
}
