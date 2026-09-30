/** Keys of "Needs attention" rows the user dismissed. Kept small: only the newest entries survive. */
const KEY = 'readur.board.dismissed.v1';
const CAP = 500;

function storage(): Storage | null {
  try {
    return typeof window !== 'undefined' ? window.localStorage : null;
  } catch {
    return null;
  }
}

export function loadDismissed(): string[] {
  try {
    const parsed: unknown = JSON.parse(storage()?.getItem(KEY) ?? '[]');
    return Array.isArray(parsed) ? parsed.filter((k): k is string => typeof k === 'string') : [];
  } catch {
    return [];
  }
}

export function saveDismissed(keys: string[]): string[] {
  const kept = keys.slice(-CAP);
  try {
    storage()?.setItem(KEY, JSON.stringify(kept));
  } catch {
    /* storage unavailable: the dismissal lasts for this session only */
  }
  return kept;
}
