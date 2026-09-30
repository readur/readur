/**
 * Shared change-tracking store: remembers which documents, sources and attention items changed
 * since the user last looked at them. Entries survive reloads via localStorage and are removed
 * once acknowledged. Components read it through `useLit` / `useLitCount`.
 */
import { useMemo, useSyncExternalStore } from 'react';

export type LitKind = 'document' | 'source' | 'attention';
export type LitReason = 'new' | 'changed' | 'failed';

export interface LitState {
  lit: boolean;
  reason?: string;
}

export const LIT_STORAGE_KEY = 'readur.lit.v1';
export const LIT_CAP = 2000;

const KINDS: readonly string[] = ['document', 'source', 'attention'];
const REASONS: readonly string[] = ['new', 'changed', 'failed'];

/** One persisted entry: [kind, id, reason, markedAt]. */
type StoredEntry = [LitKind, string, LitReason, number];

interface Entry {
  kind: LitKind;
  id: string;
  reason: LitReason;
  at: number;
}

const keyOf = (kind: LitKind, id: string) => `${kind}\u0000${id}`;

function getStorage(): Storage | null {
  try {
    return typeof window !== 'undefined' ? window.localStorage : null;
  } catch {
    return null;
  }
}

function isStoredEntry(v: unknown): v is StoredEntry {
  return (
    Array.isArray(v) &&
    v.length === 4 &&
    KINDS.includes(v[0]) &&
    typeof v[1] === 'string' &&
    REASONS.includes(v[2]) &&
    typeof v[3] === 'number'
  );
}

export interface LitStore {
  markLit(kind: LitKind, id: string, reason: LitReason): void;
  acknowledge(kind: LitKind, id: string): void;
  acknowledgeAll(kind?: LitKind): void;
  isLit(kind: LitKind, id: string): boolean;
  reasonOf(kind: LitKind, id: string): LitReason | undefined;
  count(kind?: LitKind): number;
  subscribe(listener: () => void): () => void;
}

/** Creates an isolated store. The app uses the module-level singleton below. */
export function createLitStore(): LitStore {
  // Insertion order doubles as age order: the first key is the oldest entry.
  let entries: Map<string, Entry> | null = null;
  const listeners = new Set<() => void>();

  const trim = (map: Map<string, Entry>) => {
    while (map.size > LIT_CAP) {
      const oldest = map.keys().next().value as string;
      map.delete(oldest);
    }
  };

  // Storage is read lazily, so the store is safe to use before anything was ever saved.
  const load = (): Map<string, Entry> => {
    if (entries) return entries;
    const map = new Map<string, Entry>();
    try {
      const raw = getStorage()?.getItem(LIT_STORAGE_KEY);
      const parsed: unknown = raw ? JSON.parse(raw) : [];
      if (Array.isArray(parsed)) {
        for (const item of parsed) {
          if (!isStoredEntry(item)) continue;
          const [kind, id, reason, at] = item;
          map.set(keyOf(kind, id), { kind, id, reason, at });
        }
      }
    } catch {
      /* unreadable or corrupted storage: start empty */
    }
    trim(map);
    entries = map;
    return map;
  };

  const persist = (map: Map<string, Entry>) => {
    try {
      const out: StoredEntry[] = [];
      map.forEach((e) => out.push([e.kind, e.id, e.reason, e.at]));
      getStorage()?.setItem(LIT_STORAGE_KEY, JSON.stringify(out));
    } catch {
      /* storage full or unavailable: keep the in-memory state */
    }
  };

  const commit = (map: Map<string, Entry>) => {
    persist(map);
    listeners.forEach((l) => l());
  };

  return {
    markLit(kind, id, reason) {
      const map = load();
      const key = keyOf(kind, id);
      if (map.get(key)?.reason === reason) return;
      map.delete(key);
      map.set(key, { kind, id, reason, at: Date.now() });
      trim(map);
      commit(map);
    },
    acknowledge(kind, id) {
      const map = load();
      if (map.delete(keyOf(kind, id))) commit(map);
    },
    acknowledgeAll(kind) {
      const map = load();
      let changed = false;
      Array.from(map.entries()).forEach(([key, e]) => {
        if (!kind || e.kind === kind) {
          map.delete(key);
          changed = true;
        }
      });
      if (changed) commit(map);
    },
    isLit(kind, id) {
      return load().has(keyOf(kind, id));
    },
    reasonOf(kind, id) {
      return load().get(keyOf(kind, id))?.reason;
    },
    count(kind) {
      const map = load();
      if (!kind) return map.size;
      let n = 0;
      map.forEach((e) => {
        if (e.kind === kind) n += 1;
      });
      return n;
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}

const store = createLitStore();

export const markLit = (kind: LitKind, id: string, reason: LitReason): void =>
  store.markLit(kind, id, reason);
export const acknowledge = (kind: LitKind, id: string): void => store.acknowledge(kind, id);
export const acknowledgeAll = (kind?: LitKind): void => store.acknowledgeAll(kind);
export const isLit = (kind: LitKind, id: string): boolean => store.isLit(kind, id);

/** Whether one item is currently marked as changed, and why. */
export function useLit(kind: LitKind, id: string): LitState {
  const reason = useSyncExternalStore(
    store.subscribe,
    () => store.reasonOf(kind, id),
    () => undefined,
  );
  return useMemo(() => (reason ? { lit: true, reason } : { lit: false }), [reason]);
}

/** Number of marked items, optionally of one kind. */
export function useLitCount(kind?: LitKind): number {
  return useSyncExternalStore(
    store.subscribe,
    () => store.count(kind),
    () => 0,
  );
}
