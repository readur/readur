/**
 * Shared, throttled thumbnail loading for long lists. At most `MAX_IN_FLIGHT` requests run at
 * once; finished thumbnails are kept as object URLs in a bounded LRU cache keyed by document id,
 * so a row that scrolls back into view (or remounts) does not refetch. A caller that goes away
 * before its request starts is dropped from the queue.
 */
export const MAX_IN_FLIGHT = 4;
export const CACHE_SIZE = 200;

type Fetcher = (documentId: string) => Promise<{ data: BlobPart }>;
type Waiter = (url: string | null) => void;

const cache = new Map<string, string>();
const waiters = new Map<string, Set<Waiter>>();
const queue: { id: string; fetcher: Fetcher }[] = [];
let active = 0;
// Bumped by reset so requests from before a reset cannot touch the new state.
let generation = 0;

function remember(id: string, url: string) {
  cache.delete(id);
  cache.set(id, url);
  while (cache.size > CACHE_SIZE) {
    const [oldest, oldUrl] = cache.entries().next().value as [string, string];
    cache.delete(oldest);
    URL.revokeObjectURL(oldUrl);
  }
}

function pump() {
  while (active < MAX_IN_FLIGHT && queue.length > 0) {
    const { id, fetcher } = queue.shift()!;
    const gen = generation;
    active += 1;
    fetcher(id)
      .then((res) => {
        if (gen !== generation) return null;
        const url = URL.createObjectURL(new Blob([res.data]));
        remember(id, url);
        return url;
      })
      .catch(() => null)
      .then((url) => {
        if (gen !== generation) return;
        active -= 1;
        const list = waiters.get(id);
        waiters.delete(id);
        list?.forEach((w) => w(url));
        pump();
      });
  }
}

/** The cached thumbnail URL for a document, if any (marks it recently used). */
export function cachedThumbnail(id: string): string | null {
  const url = cache.get(id);
  if (url === undefined) return null;
  remember(id, url);
  return url;
}

/**
 * Ask for a document's thumbnail. `done` gets the object URL, or null when there is none.
 * Returns a cancel function; cancelling before the request starts removes it from the queue.
 */
export function requestThumbnail(id: string, fetcher: Fetcher, done: Waiter): () => void {
  const hit = cachedThumbnail(id);
  if (hit) {
    done(hit);
    return () => undefined;
  }
  let list = waiters.get(id);
  if (!list) {
    list = new Set();
    waiters.set(id, list);
    queue.push({ id, fetcher });
  }
  list.add(done);
  pump();
  return () => {
    const current = waiters.get(id);
    if (!current) return;
    current.delete(done);
    if (current.size === 0) {
      const queued = queue.findIndex((q) => q.id === id);
      if (queued !== -1) {
        queue.splice(queued, 1);
        waiters.delete(id);
      }
    }
  };
}

/** Tests only: forget every cached thumbnail and queued request. */
export function resetThumbnailLoader(): void {
  cache.forEach((url) => URL.revokeObjectURL(url));
  cache.clear();
  waiters.clear();
  queue.length = 0;
  active = 0;
  generation += 1;
}
