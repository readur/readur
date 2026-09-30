import { useEffect, useState } from 'react';
import { documentService, type DocumentRetryHistoryItem } from '../../../services/api';

export interface RetryInfo {
  total: number;
  /** Most recent recorded failure (from the latest retry entry), if the server has one. */
  lastFailure: string | null;
}

/**
 * Retry count and the most recent failure text for one document. The document endpoint does not
 * carry these, so they come from the per-document retry-history endpoint. `version` re-reads it
 * (e.g. after a retry was queued or the OCR status changed).
 */
export function useRetryHistory(documentId: string | undefined, version: unknown): RetryInfo {
  const [info, setInfo] = useState<RetryInfo>({ total: 0, lastFailure: null });

  useEffect(() => {
    if (!documentId) return undefined;
    let cancelled = false;
    documentService
      .getDocumentRetryHistory(documentId)
      .then((res) => {
        if (cancelled || !res?.data) return;
        const items: DocumentRetryHistoryItem[] = Array.isArray(res.data.retry_history) ? res.data.retry_history : [];
        const latest = [...items].sort((a, b) => b.created_at.localeCompare(a.created_at))[0];
        setInfo({
          total: Number(res.data.total_retries) || items.length,
          lastFailure: latest?.previous_error || latest?.previous_failure_reason || null,
        });
      })
      .catch(() => {
        // No history available: show nothing rather than a guess.
      });
    return () => {
      cancelled = true;
    };
  }, [documentId, version]);

  return info;
}
