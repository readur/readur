import { useState } from 'react';
import { useMediaQuery } from '../../../ui/shared/useMediaQuery';

export type ReadingView = 'document' | 'split' | 'text';

export const READING_VIEWS: ReadingView[] = ['document', 'split', 'text'];

/** Side by side is the default from here up. */
export const SPLIT_DEFAULT_QUERY = '(min-width: 1200px)';
/** Below this width there is no room for two panes, so Side by side is not offered. */
export const SPLIT_ALLOWED_QUERY = '(min-width: 960px)';

export const VIEW_STORAGE_KEY = 'readur.document.view';

function readStored(): ReadingView | null {
  try {
    const value = window.localStorage.getItem(VIEW_STORAGE_KEY);
    return READING_VIEWS.includes(value as ReadingView) ? (value as ReadingView) : null;
  } catch {
    return null;
  }
}

function writeStored(view: ReadingView) {
  try {
    window.localStorage.setItem(VIEW_STORAGE_KEY, view);
  } catch {
    // Private windows can refuse storage; the choice still holds for this page.
  }
}

export interface ReadingViewOptions {
  /** The page was opened from a search: the text has to be visible to show the matches. */
  wantsText: boolean;
  /** The file itself can't be shown in the browser, so the text is the useful default. */
  noPreview: boolean;
}

export interface UseReadingViewResult {
  view: ReadingView;
  canSplit: boolean;
  setView: (view: ReadingView) => void;
}

/**
 * Which of Document / Side by side / Text the reading area shows. The user's pick is kept in
 * localStorage. Without one, wide screens get Side by side and the rest get Document; arriving
 * from a search, or on a file with no preview, the text is brought into view instead.
 */
export function useReadingView({ wantsText, noPreview }: ReadingViewOptions): UseReadingViewResult {
  const splitByDefault = useMediaQuery(SPLIT_DEFAULT_QUERY);
  const canSplit = useMediaQuery(SPLIT_ALLOWED_QUERY);
  const [stored] = useState(readStored);
  const [picked, setPicked] = useState<ReadingView | null>(null);

  let view: ReadingView = picked ?? stored ?? (splitByDefault ? 'split' : 'document');
  if (!picked && view === 'document' && (wantsText || noPreview)) view = canSplit && !noPreview ? 'split' : 'text';
  if (view === 'split' && !canSplit) view = wantsText || noPreview ? 'text' : 'document';

  const setView = (next: ReadingView) => {
    setPicked(next);
    writeStored(next);
  };

  return { view, canSplit, setView };
}
