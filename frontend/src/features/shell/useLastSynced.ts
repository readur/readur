import { useEffect, useMemo, useState } from 'react';
import api from '../../services/api';
import type { SourceResponse } from '../../services/api';

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

/** Fired (on window) by any screen that adds, edits or removes a source. */
export const SOURCES_CHANGED_EVENT = 'readur:sources-changed';

/**
 * Fetches the sources list on mount, once a minute while the tab is visible, once when it becomes
 * visible again, and whenever `readur:sources-changed` fires. Null until the first load succeeds;
 * a failed poll keeps the previous list.
 */
export function useSourcesList(): SourceResponse[] | null {
  const [sources, setSources] = useState<SourceResponse[] | null>(null);

  useEffect(() => {
    let cancelled = false;
    const hidden = () => typeof document !== 'undefined' && document.hidden;
    const load = async () => {
      try {
        const res = await api.get('/sources');
        if (cancelled) return;
        if (Array.isArray(res?.data)) setSources(res.data as SourceResponse[]);
      } catch {
        /* keep the previous list: a failed poll is not a reason to hide the sources */
      }
    };
    void load();
    const timer = window.setInterval(() => {
      if (!hidden()) void load();
    }, SYNC_POLL_MS);
    const onVisibility = () => {
      if (!hidden()) void load();
    };
    const onChanged = () => void load();
    document.addEventListener('visibilitychange', onVisibility);
    window.addEventListener(SOURCES_CHANGED_EVENT, onChanged);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
      document.removeEventListener('visibilitychange', onVisibility);
      window.removeEventListener(SOURCES_CHANGED_EVENT, onChanged);
    };
  }, []);

  return sources;
}

/** Most recent sync across `sources`; the same Date instance while the time is unchanged. */
export function useLatestSync(sources: SourceResponse[] | null): Date | null {
  const time = latestSync(sources)?.getTime() ?? null;
  return useMemo(() => (time === null ? null : new Date(time)), [time]);
}

/** Polls the sources list and returns the most recent sync time, or null. */
export function useLastSynced(): Date | null {
  return useLatestSync(useSourcesList());
}
