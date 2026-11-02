import { useEffect, useState } from 'react';
import { labelService, type LabelResponse } from '../../services/api';

/** Fired (on window) by any screen that creates, edits, deletes or assigns labels. */
export const LABELS_CHANGED_EVENT = 'readur:labels-changed';

/** How many collections the sidebar lists before "All collections". */
export const SIDEBAR_COLLECTION_LIMIT = 8;

const isLabel = (x: unknown): x is LabelResponse =>
  typeof x === 'object' && x !== null && typeof (x as LabelResponse).id === 'string' && typeof (x as LabelResponse).name === 'string';

/** Most-used first, then by name; capped at `limit`. */
export function topCollections(labels: readonly LabelResponse[], limit = SIDEBAR_COLLECTION_LIMIT): LabelResponse[] {
  return [...labels]
    .sort((a, b) => (b.document_count ?? 0) - (a.document_count ?? 0) || a.name.localeCompare(b.name))
    .slice(0, limit);
}

/**
 * The user's labels with document counts. Loads on mount and again whenever
 * `readur:labels-changed` fires, so a collection saved from Search shows up at once. Null until
 * the first load succeeds; a failed reload keeps the previous list.
 */
export function useCollections(): LabelResponse[] | null {
  const [labels, setLabels] = useState<LabelResponse[] | null>(null);

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      try {
        const res = await labelService.list(true);
        if (cancelled || !Array.isArray(res?.data)) return;
        setLabels(res.data.filter(isLabel));
      } catch {
        /* keep what we had */
      }
    };
    void load();
    const onChanged = () => void load();
    window.addEventListener(LABELS_CHANGED_EVENT, onChanged);
    return () => {
      cancelled = true;
      window.removeEventListener(LABELS_CHANGED_EVENT, onChanged);
    };
  }, []);

  return labels;
}
