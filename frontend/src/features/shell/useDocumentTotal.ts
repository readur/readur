import { useEffect, useState } from 'react';
import api from '../../services/api';
import { SYNC_POLL_MS } from './useLastSynced';

/** Fired (on window) by any screen that adds or removes documents. */
export const DOCUMENTS_CHANGED_EVENT = 'readur:documents-changed';

/**
 * The number of documents in the library, for the sidebar. Asks for a one-row page and reads
 * its total; polls once a minute while the tab is visible. Null until the first load succeeds.
 */
export function useDocumentTotal(): number | null {
  const [total, setTotal] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      try {
        const res = await api.get('/documents', { params: { limit: 1, offset: 0 } });
        const value = (res?.data as { pagination?: { total?: unknown } } | undefined)?.pagination?.total;
        if (!cancelled && typeof value === 'number') setTotal(value);
      } catch {
        /* keep the previous figure */
      }
    };
    void load();
    const timer = window.setInterval(() => {
      if (!document.hidden) void load();
    }, SYNC_POLL_MS);
    const onChanged = () => void load();
    window.addEventListener(DOCUMENTS_CHANGED_EVENT, onChanged);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
      window.removeEventListener(DOCUMENTS_CHANGED_EVENT, onChanged);
    };
  }, []);

  return total;
}
