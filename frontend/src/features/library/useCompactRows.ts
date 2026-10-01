import { useCallback, useState } from 'react';

export const DENSITY_STORAGE_KEY = 'readur.library.compact';

function read(): boolean {
  try {
    return window.localStorage.getItem(DENSITY_STORAGE_KEY) === 'true';
  } catch {
    return false;
  }
}

/** Compact rows on or off, remembered in this browser. */
export function useCompactRows(): [boolean, (compact: boolean) => void] {
  const [compact, setCompactState] = useState(read);
  const setCompact = useCallback((next: boolean) => {
    setCompactState(next);
    try {
      window.localStorage.setItem(DENSITY_STORAGE_KEY, String(next));
    } catch {
      // Storage can be unavailable (private mode); the setting then lasts for this visit.
    }
  }, []);
  return [compact, setCompact];
}
