/**
 * Shared change-tracking store: remembers which documents, sources and attention items changed
 * since the user last looked at them. Entries survive reloads via localStorage and are removed
 * once acknowledged. Components read it through `useLit` / `useLitCount`.
 */
import { useEffect, useMemo, useRef, useSyncExternalStore } from 'react';

export type LitKind = 'document' | 'source' | 'attention';
export type LitReason = 'new' | 'changed' | 'failed';

export interface LitState {
  lit: boolean;
  reason?: string;
}

export const LIT_STORAGE_KEY = 'readur.lit.v1';
export const LIT_CAP = 2000;
/**
 * How many entries of one kind are flagged at once: the ones whose items are newest. Older
 * entries stay stored (and come back once newer ones are seen), so a bulk import can never flag
 * hundreds of rows.
 */
export const LIT_SHOWN_CAP = 25;

const KINDS: readonly string[] = ['document', 'source', 'attention'];
const REASONS: readonly string[] = ['new', 'changed', 'failed'];

/** One persisted entry: [kind, id, reason, at] (the item's own time, or when it was marked). */
type StoredEntry = [LitKind, string, LitReason, number];

interface Entry {
  kind: LitKind;
  id: string;
  reason: LitReason;
  /** The item's own time (ms) when the caller gave one, otherwise when it was marked. */
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
  /**
   * Marks an item as changed. `at` is the item's own time in ms (when it failed, arrived…); it
   * ranks the entry for the shown cap. Without it, the time of marking is used.
   */
  markLit(kind: LitKind, id: string, reason: LitReason, at?: number): void;
  acknowledge(kind: LitKind, id: string): void;
  acknowledgeAll(kind?: LitKind): void;
  isLit(kind: LitKind, id: string): boolean;
  reasonOf(kind: LitKind, id: string): LitReason | undefined;
  count(kind?: LitKind): number;
  /** Marked, and among the {@link LIT_SHOWN_CAP} newest entries of its kind (by `at`). */
  isShown(kind: LitKind, id: string): boolean;
  /** The reason of a shown entry; undefined when not marked or beyond the shown cap. */
  shownReason(kind: LitKind, id: string): LitReason | undefined;
  /** Number of shown entries of one kind (at most {@link LIT_SHOWN_CAP}). */
  shownCount(kind: LitKind): number;
  subscribe(listener: () => void): () => void;
  /** Writes any pending change to storage now (writes are otherwise batched per tick). */
  flush(): void;
}

/** Creates an isolated store. The app uses the module-level singleton below. */
export function createLitStore(): LitStore {
  // Insertion order doubles as age order: the first key is the oldest entry.
  let entries: Map<string, Entry> | null = null;
  /** Keys of the newest entries per kind; rebuilt lazily after any change. */
  let shown: Map<LitKind, Set<string>> | null = null;
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

  // Writes are coalesced: any number of changes in one tick cost a single storage write.
  let dirty = false;
  const flush = () => {
    if (!dirty) return;
    dirty = false;
    if (entries) persist(entries);
  };

  const shownOf = (kind: LitKind): Set<string> => {
    if (!shown) {
      const next = new Map<LitKind, Set<string>>();
      // Rank by each item's own time, newest first, so marking an older page of a list after a
      // newer one never pushes the newer items out; ties go to the entry marked last.
      const all = Array.from(load().entries()).map(([key, e], order) => ({ key, e, order }));
      all.sort((a, b) => b.e.at - a.e.at || b.order - a.order);
      for (const { key, e } of all) {
        let set = next.get(e.kind);
        if (!set) {
          set = new Set();
          next.set(e.kind, set);
        }
        if (set.size < LIT_SHOWN_CAP) set.add(key);
      }
      shown = next;
    }
    return shown.get(kind) ?? new Set();
  };

  const commit = (_map: Map<string, Entry>) => {
    shown = null;
    if (!dirty) {
      dirty = true;
      queueMicrotask(flush);
    }
    listeners.forEach((l) => l());
  };

  // Another tab changed the stored entries: drop the in-memory copy and re-read on next use.
  const onStorage = (e: StorageEvent) => {
    if (e.key !== null && e.key !== LIT_STORAGE_KEY) return;
    if (e.storageArea && e.storageArea !== getStorage()) return;
    dirty = false;
    entries = null;
    shown = null;
    listeners.forEach((l) => l());
  };

  return {
    markLit(kind, id, reason, at) {
      const map = load();
      const key = keyOf(kind, id);
      const time = at !== undefined && Number.isFinite(at) ? at : undefined;
      const existing = map.get(key);
      if (existing?.reason === reason) {
        if (time === undefined || existing.at === time) return;
        existing.at = time;
        commit(map);
        return;
      }
      map.delete(key);
      map.set(key, { kind, id, reason, at: time ?? Date.now() });
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
    isShown(kind, id) {
      return shownOf(kind).has(keyOf(kind, id));
    },
    shownReason(kind, id) {
      const key = keyOf(kind, id);
      return shownOf(kind).has(key) ? load().get(key)?.reason : undefined;
    },
    shownCount(kind) {
      return shownOf(kind).size;
    },
    subscribe(listener) {
      if (listeners.size === 0 && typeof window !== 'undefined') window.addEventListener('storage', onStorage);
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
        if (listeners.size === 0 && typeof window !== 'undefined') window.removeEventListener('storage', onStorage);
      };
    },
    flush,
  };
}

const store = createLitStore();

export const markLit = (kind: LitKind, id: string, reason: LitReason, at?: number): void =>
  store.markLit(kind, id, reason, at);
export const acknowledge = (kind: LitKind, id: string): void => store.acknowledge(kind, id);
export const acknowledgeAll = (kind?: LitKind): void => store.acknowledgeAll(kind);
export const isLit = (kind: LitKind, id: string): boolean => store.isLit(kind, id);
/** Why an entry is marked (undefined when it is not). */
export const litReason = (kind: LitKind, id: string): LitReason | undefined => store.reasonOf(kind, id);
/** Whether a row should be flagged: marked, and among the newest entries of its kind. */
export const isShownLit = (kind: LitKind, id: string): boolean => store.isShown(kind, id);
/** Writes pending changes to localStorage immediately. */
export const flushLit = (): void => store.flush();

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

/** Like `useLit`, but only for entries within the shown cap: what a row should draw. */
export function useShownLit(kind: LitKind, id: string): LitState {
  const reason = useSyncExternalStore(
    store.subscribe,
    () => store.shownReason(kind, id),
    () => undefined,
  );
  return useMemo(() => (reason ? { lit: true, reason } : { lit: false }), [reason]);
}

/** Number of rows of one kind currently flagged (at most LIT_SHOWN_CAP). */
export function useShownLitCount(kind: LitKind): number {
  return useSyncExternalStore(
    store.subscribe,
    () => store.shownCount(kind),
    () => 0,
  );
}

/**
 * Acknowledge-on-leave for a page. Every row in `ids` that is flagged while the page is on
 * screen is remembered, and all of them are acknowledged when the page unmounts (the user
 * leaves it) or the tab is hidden. A flagged row therefore stays flagged for the whole visit in
 * which it is first seen, and is clear on the next one.
 */
export function useAcknowledgeOnLeave(kind: LitKind, ids: readonly string[], onLeave?: () => void): void {
  const seen = useRef(new Set<string>());
  const leave = useRef(onLeave);
  useEffect(() => {
    leave.current = onLeave;
  }, [onLeave]);
  const version = useLitCount(kind);
  const idsKey = ids.join('\u0000');

  useEffect(() => {
    for (const id of idsKey ? idsKey.split('\u0000') : []) {
      if (store.isShown(kind, id)) seen.current.add(id);
    }
  }, [kind, idsKey, version]);

  const generation = useRef(0);
  useEffect(() => {
    const current = ++generation.current;
    const ackSeen = () => {
      seen.current.forEach((id) => store.acknowledge(kind, id));
      seen.current.clear();
      leave.current?.();
      store.flush();
    };
    window.addEventListener('pagehide', ackSeen);
    return () => {
      window.removeEventListener('pagehide', ackSeen);
      // Deferred by a microtask: React's development double-mount runs this effect again at
      // once (a new generation), which cancels the acknowledgement; a real unmount lets it run.
      queueMicrotask(() => {
        if (generation.current === current) ackSeen();
      });
    };
  }, [kind]);
}
