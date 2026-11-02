import { useCallback, useState } from 'react';

const STORAGE_KEY = 'readur.document.previewHeight';
export const PREVIEW_MIN = 120;
/** Room kept for the tabs (tab row, find bar and a few lines of text) under the file. */
const TABS_ROOM = 240;

const windowHeight = () => (typeof window !== 'undefined' ? window.innerHeight : 800);

/** Before anyone resizes it: about a third of the window, within sensible bounds. */
const defaultHeight = () => Math.round(Math.min(520, Math.max(200, windowHeight() * 0.34)));

function readStored(): number | null {
  try {
    const raw = Number(window.localStorage.getItem(STORAGE_KEY));
    return Number.isFinite(raw) && raw > 0 ? raw : null;
  } catch {
    return null;
  }
}

/**
 * The file's height in the document drawer, set with the handle under it and remembered in this
 * browser. `max` leaves the tabs room in a body `bodyHeight` px tall.
 */
export function usePreviewHeight() {
  const [height, setHeight] = useState(() => readStored() ?? defaultHeight());
  const max = useCallback(
    (bodyHeight: number) => Math.max(PREVIEW_MIN, (bodyHeight > 0 ? bodyHeight : windowHeight() * 0.8) - TABS_ROOM),
    [],
  );
  const remember = useCallback((value: number) => {
    try {
      window.localStorage.setItem(STORAGE_KEY, String(value));
    } catch {
      /* private mode or blocked storage: the height just isn't remembered */
    }
  }, []);
  return { height, setHeight, max, remember };
}
