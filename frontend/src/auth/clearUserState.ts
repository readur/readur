import { LIT_STORAGE_KEY, resetLit } from '../features/board/litStore';
import { resetDocumentBaseline } from '../features/board/litFeeders';
import { clearRecentSearches } from '../features/shell/recentSearches';

/** Storage key prefixes holding what one user has seen or dismissed on the Board and in Intake. */
const USER_KEY_PREFIXES = ['readur.board.', 'readur.intake.'];

/**
 * Forgets everything this browser remembers about the signed-in user's activity, so the next
 * person to sign in does not inherit their recent searches or changed-item markers.
 */
export function clearUserState(): void {
  try {
    const storage = window.localStorage;
    const doomed: string[] = [LIT_STORAGE_KEY];
    for (let i = 0; i < storage.length; i += 1) {
      const key = storage.key(i);
      if (key && USER_KEY_PREFIXES.some((p) => key.startsWith(p))) doomed.push(key);
    }
    doomed.forEach((key) => storage.removeItem(key));
  } catch {
    /* storage unavailable: nothing stored */
  }
  clearRecentSearches();
  resetLit();
  resetDocumentBaseline();
}
