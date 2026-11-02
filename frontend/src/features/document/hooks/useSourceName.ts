import { useEffect, useState } from 'react';
import { sourcesService } from '../../../services/api';

/**
 * The name of the connection a document came in through ("Scanner inbox"), or null while it
 * loads, when the document has no source, or when the source can't be read.
 */
export function useSourceName(sourceId: string | null | undefined): string | null {
  const [name, setName] = useState<string | null>(null);

  useEffect(() => {
    setName(null);
    if (!sourceId) return undefined;
    let cancelled = false;
    Promise.resolve()
      .then(() => sourcesService.list())
      .then((res) => {
        if (cancelled) return;
        const match = (res?.data ?? []).find((s) => s.id === sourceId);
        setName(match?.name ?? null);
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [sourceId]);

  return name;
}
