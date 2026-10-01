import { useEffect, useRef, useState } from 'react';
import type { CommandSource, CommandItem } from './types';

export const SEARCH_DEBOUNCE_MS = 150;

export type ResultGroups = Array<{ source: CommandSource; items: CommandItem[] }>;

export interface SourceResults {
  /** Results for the current query only; empty while that query is still being searched. */
  results: ResultGroups;
  /** True until results for the current query have arrived. */
  isLoading: boolean;
}

interface Answer {
  query: string;
  groups: ResultGroups;
}

/**
 * Queries every source with a debounce and keeps only the latest answer. Each answer is tagged
 * with the query it belongs to and is exposed only while that query is still current, so results
 * from an earlier query can never be selected. An empty query only reaches sources that set
 * `searchesEmpty`. A source that rejects contributes no items instead of failing the search.
 */
export function useSourceSearch(sources: CommandSource[], query: string, enabled: boolean): SourceResults {
  const [answer, setAnswer] = useState<Answer | null>(null);
  const sourcesRef = useRef(sources);
  sourcesRef.current = sources;
  const requestId = useRef(0);

  useEffect(() => {
    const id = ++requestId.current;
    if (!enabled) {
      setAnswer(null);
      return undefined;
    }
    const isEmpty = query.trim() === '';
    const targets = isEmpty ? sourcesRef.current.filter((s) => s.searchesEmpty) : sourcesRef.current;
    if (targets.length === 0) {
      setAnswer({ query, groups: [] });
      return undefined;
    }
    const timer = setTimeout(() => {
      void Promise.all(
        targets.map((source) =>
          Promise.resolve()
            .then(() => source.search(query))
            .then(
              (items) => ({ source, items: Array.isArray(items) ? items : [] }),
              () => ({ source, items: [] as CommandItem[] }),
            ),
        ),
      ).then((groups) => {
        if (id !== requestId.current) return;
        setAnswer({ query, groups: groups.filter((group) => group.items.length > 0) });
      });
    }, SEARCH_DEBOUNCE_MS);
    return () => clearTimeout(timer);
  }, [query, enabled]);

  const current = enabled && answer !== null && answer.query === query;
  return { results: current ? answer.groups : [], isLoading: enabled && !current };
}
