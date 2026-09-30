import { useEffect, useMemo, useRef, useState } from 'react';
import { fetchTimeline, timelineParams } from '../data';
import { fillMonths, type MonthBar } from '../months';
import type { LibraryQuery } from '../urlState';

export interface TimelineState {
  status: 'idle' | 'loading' | 'ready' | 'error';
  bars: MonthBar[];
  /** Matches across every month. */
  total: number;
}

const IDLE: TimelineState = { status: 'idle', bars: [], total: 0 };

/**
 * Matches per month for the search, across every date (the date filter is left out so the chart
 * keeps its shape while a month is picked). Only the newest request may update the chart; the
 * previous bars stay up while the next ones load.
 */
export function useTimeline(query: LibraryQuery, mimeTypes: string[], enabled: boolean): TimelineState {
  const [state, setState] = useState<TimelineState>(IDLE);
  const latest = useRef(0);
  const key = useMemo(() => JSON.stringify(timelineParams(query, mimeTypes)), [query, mimeTypes]);

  useEffect(() => {
    if (!enabled) {
      latest.current += 1;
      setState(IDLE);
      return;
    }
    const id = ++latest.current;
    setState((prev) => ({ ...prev, status: 'loading' }));
    fetchTimeline(query, mimeTypes)
      .then((months) => {
        if (id !== latest.current) return;
        const bars = fillMonths(months);
        setState({ status: 'ready', bars, total: months.reduce((sum, m) => sum + m.count, 0) });
      })
      .catch(() => {
        if (id === latest.current) setState({ status: 'error', bars: [], total: 0 });
      });
    // `key` captures everything the request depends on.
  }, [key, enabled]);

  return state;
}
