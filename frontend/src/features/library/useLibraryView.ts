import { useCallback, useState } from 'react';

export type LibraryView = 'grid' | 'table';

export const VIEW_STORAGE_KEY = 'readur.library.view';

function read(): LibraryView {
  try {
    return window.localStorage.getItem(VIEW_STORAGE_KEY) === 'table' ? 'table' : 'grid';
  } catch {
    return 'grid';
  }
}

/** Grid of thumbnails or the dense table, remembered in this browser. Grid by default. */
export function useLibraryView(): [LibraryView, (view: LibraryView) => void] {
  const [view, setViewState] = useState<LibraryView>(read);
  const setView = useCallback((next: LibraryView) => {
    setViewState(next);
    try {
      window.localStorage.setItem(VIEW_STORAGE_KEY, next);
    } catch {
      // Storage can be unavailable (private mode); the choice then lasts for this visit.
    }
  }, []);
  return [view, setView];
}
