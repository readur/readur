import { useEffect } from 'react';
import { sourcesService, type SourceResponse } from '../../../services/api';
import { flagNewFailures, SOURCE_EVENTS_KEY } from '../shared/seenEvents';
import { useLoader } from '../shared/useLoader';
import { isFailing } from './sourceModel';

export const SYNC_POLL_MS = 5000;

/**
 * All connections. While any connection is syncing the list is refreshed every 5 seconds.
 * A connection whose sync fails is marked once per failure (keyed by its error time).
 */
export function useSources() {
  const loader = useLoader(async () => {
    const res = await sourcesService.list();
    return Array.isArray(res.data) ? res.data : [];
  }, []);
  const sources = loader.data;
  const anySyncing = (sources ?? []).some((s) => s.status === 'syncing');
  const { reload } = loader;

  useEffect(() => {
    if (!anySyncing) return undefined;
    const timer = window.setInterval(() => void reload(), SYNC_POLL_MS);
    return () => window.clearInterval(timer);
  }, [anySyncing, reload]);

  useEffect(() => {
    if (!sources) return;
    flagNewFailures(
      SOURCE_EVENTS_KEY,
      'source',
      sources.filter(isFailing).map((s: SourceResponse) => ({
        id: s.id,
        eventKey: `${s.id}@${s.last_error_at ?? s.updated_at}`,
      })),
    );
  }, [sources]);

  return { ...loader, isAutoRefreshing: anySyncing };
}
