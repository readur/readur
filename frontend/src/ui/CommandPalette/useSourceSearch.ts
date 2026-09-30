import { useEffect, useRef, useState } from 'react';
import type { CommandSource, CommandItem } from './types';

export const SEARCH_DEBOUNCE_MS = 150;

export interface SourceResults {
  results: Array<{ source: CommandSource; items: CommandItem[] }>;
  isLoading: boolean;
}

/**
 * Queries every source with a debounce and keeps only the latest answer. A source that rejects
 * contributes no items instead of failing the whole search.
 */
export function useSourceSearch(sources: CommandSource[], query: string, enabled: boolean): SourceResults {
  const [results, setResults] = useState<SourceResults['results']>([]);
  const [isLoading, setLoading] = useState(false);
  const sourcesRef = useRef(sources);
  sourcesRef.current = sources;
  const requestId = useRef(0);

  useEffect(() => {
    const id = ++requestId.current;
    if (!enabled) {
      setResults([]);
      setLoading(false);
      return undefined;
    }
    setLoading(true);
    const timer = setTimeout(() => {
      const current = sourcesRef.current;
      void Promise.all(
        current.map((source) =>
          Promise.resolve()
            .then(() => source.search(query))
            .then(
              (items) => ({ source, items: Array.isArray(items) ? items : [] }),
              () => ({ source, items: [] as CommandItem[] }),
            ),
        ),
      ).then((next) => {
        if (id !== requestId.current) return;
        setResults(next.filter((group) => group.items.length > 0));
        setLoading(false);
      });
    }, SEARCH_DEBOUNCE_MS);
    return () => clearTimeout(timer);
  }, [query, enabled]);

  return { results, isLoading };
}
