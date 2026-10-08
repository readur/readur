import { useCallback, useEffect, useRef, useState } from 'react';
import { labelService } from '../../../services/api';
import { notifyLabelsChanged, type LabelData } from '../../labels';

export type NewLabel = Omit<
  LabelData,
  'id' | 'is_system' | 'created_at' | 'updated_at' | 'document_count' | 'source_count'
>;

export interface LabelSaveResult {
  ok: boolean;
  /** After a failure: the labels rolled back to, or null when a newer save kept its own. */
  restored: LabelData[] | null;
}

export interface UseDocumentLabelsResult {
  labels: LabelData[];
  available: LabelData[];
  isLoading: boolean;
  /**
   * Shows `next` at once and saves it. A failed save rolls the labels back to the last ones the
   * server accepted and resolves with those, unless a newer save has started since (an older
   * failure must not undo a newer change); then nothing rolls back and `restored` is null.
   */
  save: (next: LabelData[]) => Promise<LabelSaveResult>;
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
    async (next: LabelData[]): Promise<LabelSaveResult> => {
      if (!documentId) return { ok: false, restored: null };
      const seq = ++latest.current;
      setLabels(next);
      try {
        await labelService.setDocumentLabels(documentId, next.map((l) => l.id));
        saved.current = next;
        notifyLabelsChanged();
        return { ok: true, restored: null };
      } catch {
        if (latest.current !== seq) return { ok: false, restored: null };
        setLabels(saved.current);
        return { ok: false, restored: saved.current };
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
