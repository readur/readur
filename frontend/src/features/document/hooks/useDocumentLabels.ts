import { useCallback, useEffect, useState } from 'react';
import api from '../../../services/api';
import type { LabelData } from '../../labels';

export type NewLabel = Omit<
  LabelData,
  'id' | 'is_system' | 'created_at' | 'updated_at' | 'document_count' | 'source_count'
>;

export interface UseDocumentLabelsResult {
  labels: LabelData[];
  available: LabelData[];
  isLoading: boolean;
  save: (next: LabelData[]) => Promise<void>;
  create: (label: NewLabel) => Promise<LabelData>;
}

/**
 * Labels on one document plus the user's label catalogue. The services barrel has no labels
 * module yet, so this goes through the shared `api` client exactly as the old page did.
 */
export function useDocumentLabels(documentId: string | undefined): UseDocumentLabelsResult {
  const [labels, setLabels] = useState<LabelData[]>([]);
  const [available, setAvailable] = useState<LabelData[]>([]);
  const [isLoading, setLoading] = useState(false);

  useEffect(() => {
    if (!documentId) return undefined;
    let cancelled = false;
    setLoading(true);
    Promise.allSettled([
      api.get(`/labels/documents/${documentId}`),
      api.get('/labels?include_counts=false'),
    ]).then(([mine, all]) => {
      if (cancelled) return;
      if (mine.status === 'fulfilled' && Array.isArray(mine.value?.data)) setLabels(mine.value.data);
      if (all.status === 'fulfilled' && Array.isArray(all.value?.data)) setAvailable(all.value.data);
      setLoading(false);
    });
    return () => {
      cancelled = true;
    };
  }, [documentId]);

  const save = useCallback(
    async (next: LabelData[]) => {
      if (!documentId) return;
      await api.put(`/labels/documents/${documentId}`, { label_ids: next.map((l) => l.id) });
      setLabels(next);
    },
    [documentId],
  );

  const create = useCallback(async (label: NewLabel) => {
    const res = await api.post('/labels', label);
    const created = res.data as LabelData;
    setAvailable((prev) => [...prev, created]);
    return created;
  }, []);

  return { labels, available, isLoading, save, create };
}
