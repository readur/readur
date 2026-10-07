import { useRef } from 'react';

/**
 * The latest non-null value seen. A panel whose record is cleared as it closes keeps rendering
 * the last record, so its exit animation has something to animate.
 */
export function useLastDefined<T>(value: T | null | undefined): T | null {
  const last = useRef<T | null>(null);
  if (value !== null && value !== undefined) last.current = value;
  return last.current;
}
