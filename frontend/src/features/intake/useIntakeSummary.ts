import { useEffect, useState } from 'react';
import { ocrService, queueService, sourcesService } from '../../services/api';

export interface IntakeSummary {
  connections: number | null;
  attention: number | null;
  processing: number | null;
}

const settle = async <T,>(p: Promise<T>): Promise<T | null> => {
  try {
    return await p;
  } catch {
    return null;
  }
};

/** Counts for the page header; each one is independent, so a failing request just hides its part. */
export function useIntakeSummary(refreshKey: unknown): IntakeSummary {
  const [summary, setSummary] = useState<IntakeSummary>({ connections: null, attention: null, processing: null });

  useEffect(() => {
    let alive = true;
    void Promise.all([
      settle(sourcesService.list()),
      settle(ocrService.listFailedDocuments({ limit: 1, offset: 0 })),
      settle(queueService.getStats()),
    ]).then(([sources, failed, stats]) => {
      if (!alive) return;
      setSummary({
        connections: Array.isArray(sources?.data) ? sources.data.length : null,
        attention:
          failed?.data?.statistics?.total_failed ?? failed?.data?.pagination?.total ?? null,
        processing: stats?.data ? (stats.data.pending_count ?? 0) + (stats.data.processing_count ?? 0) : null,
      });
    });
    return () => {
      alive = false;
    };
  }, [refreshKey]);

  return summary;
}
