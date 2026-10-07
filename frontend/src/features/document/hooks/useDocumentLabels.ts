import { useCallback, useEffect, useRef, useState } from 'react';
import { labelService } from '../../../services/api';
import { notifyLabelsChanged, type LabelData } from '../../labels';

export type NewLabel = Omit<
  LabelData,
  'id' | 'is_system' | 'created_at' | 'updated_at' | 'document_count' | 'source_count'
>;

export interface UseDocumentLabelsResult {
  labels: LabelData[];
  available: LabelData[];
  isLoading: boolean;
  /**
   * Shows `next` at once and saves it. Resolves false when the save failed; the labels then roll
   * back, unless a newer save has started since (an older failure must not undo a newer change).
   */
  save: (next: LabelData[]) => Promise<boolean>;
  create: (label: NewLabel) => Promise<LabelData>;
}

/** Labels on one document plus the user's label catalogue. */
export function useDocumentLabels(documentId: string | undefined): UseDocumentLabelsResult {
  const [labels, setLabels] = useState<LabelData[]>([]);
  const [available, setAvailable] = useState<LabelData[]>([]);
  const [isLoading, setLoading] = useState(false);
  // The newest save started, and the newest labels the server accepted.
  const latest = useRef(0);
  const saved = useRef<LabelData[]>([]);

  useEffect(() => {
    if (!documentId) return undefined;
    let cancelled = false;
    setLoading(true);
    Promise.allSettled([labelService.getDocumentLabels(documentId), labelService.list(false)]).then(([mine, all]) => {
      if (cancelled) return;
      if (mine.status === 'fulfilled' && Array.isArray(mine.value?.data)) {
        setLabels(mine.value.data as LabelData[]);
        saved.current = mine.value.data as LabelData[];
      }
      if (all.status === 'fulfilled' && Array.isArray(all.value?.data)) setAvailable(all.value.data as LabelData[]);
      setLoading(false);
    });
    return () => {
      cancelled = true;
    };
  }, [documentId]);

  const save = useCallback(
    async (next: LabelData[]) => {
      if (!documentId) return false;
      const seq = ++latest.current;
      setLabels(next);
      try {
        await labelService.setDocumentLabels(documentId, next.map((l) => l.id));
        saved.current = next;
        notifyLabelsChanged();
        return true;
      } catch {
        if (latest.current === seq) setLabels(saved.current);
        return false;
      }
    },
    [documentId],
  );

  const create = useCallback(async (label: NewLabel) => {
    const res = await labelService.create(label);
    const created = res.data as LabelData;
    setAvailable((prev) => [...prev, created]);
    return created;
  }, []);

  return { labels, available, isLoading, save, create };
}
