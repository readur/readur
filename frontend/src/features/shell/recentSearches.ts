/** Recent search queries, shared with the old global search bar (same storage key and limit). */
export const RECENT_SEARCHES_KEY = 'recentSearches';
export const RECENT_SEARCHES_LIMIT = 5;

export function readRecentSearches(): string[] {
  try {
    const raw = window.localStorage.getItem(RECENT_SEARCHES_KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed.filter((s): s is string => typeof s === 'string') : [];
  } catch {
    return [];
  }
}

/** Moves `query` to the front, dropping duplicates and anything past the limit. */
export function saveRecentSearch(query: string): void {
  const q = query.trim();
  if (!q) return;
  const next = [q, ...readRecentSearches().filter((s) => s !== q)].slice(0, RECENT_SEARCHES_LIMIT);
  try {
    window.localStorage.setItem(RECENT_SEARCHES_KEY, JSON.stringify(next));
  } catch {
    /* storage unavailable: nothing to remember */
  }
}

/** Forgets every recent search (sign-out). */
export function clearRecentSearches(): void {
  try {
    window.localStorage.removeItem(RECENT_SEARCHES_KEY);
  } catch {
    /* storage unavailable: nothing stored */
  }
}
