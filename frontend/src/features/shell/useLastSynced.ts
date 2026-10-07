import { useEffect, useMemo, useRef, useState } from 'react';
import api from '../../services/api';
import type { SourceResponse } from '../../services/api';

export const SYNC_POLL_MS = 60_000;
/** While a source is syncing the list is checked this often, so the sync toast starts and ends promptly. */
export const ACTIVE_SYNC_POLL_MS = 10_000;

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
  const anySyncing = (sources ?? []).some((s) => s?.status === 'syncing');

  const loadRef = useRef<() => Promise<void>>(async () => undefined);

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
    loadRef.current = load;
    void load();
    const onVisibility = () => {
      if (!hidden()) void load();
    };
    const onChanged = () => void load();
    document.addEventListener('visibilitychange', onVisibility);
    window.addEventListener(SOURCES_CHANGED_EVENT, onChanged);
    return () => {
      cancelled = true;
      document.removeEventListener('visibilitychange', onVisibility);
      window.removeEventListener(SOURCES_CHANGED_EVENT, onChanged);
    };
  }, []);

  // The poll interval follows whether anything is syncing; changing it does not load again.
  useEffect(() => {
    const timer = window.setInterval(() => {
      if (typeof document === 'undefined' || !document.hidden) void loadRef.current();
    }, anySyncing ? ACTIVE_SYNC_POLL_MS : SYNC_POLL_MS);
    return () => window.clearInterval(timer);
  }, [anySyncing]);

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
