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

/**
 * The flat squares the server returns instead of a preview (PDF red, TXT green, DOC blue, other
 * grey). Real previews are never one of these solid colours.
 */
const PLACEHOLDER_COLOURS: readonly [number, number, number][] = [
  [220, 38, 27],
  [34, 139, 34],
  [41, 128, 185],
  [108, 117, 125],
];
/** JPEG noise allowance per channel. */
const TOLERANCE = 16;
const SAMPLE = 12;

/** True when every sampled RGBA pixel is the same known placeholder colour. */
export function isPlaceholderPixels(data: ArrayLike<number>): boolean {
  if (data.length < 4) return false;
  return PLACEHOLDER_COLOURS.some((c) => {
    for (let i = 0; i < data.length; i += 4) {
      if (
        Math.abs(data[i] - c[0]) > TOLERANCE ||
        Math.abs(data[i + 1] - c[1]) > TOLERANCE ||
        Math.abs(data[i + 2] - c[2]) > TOLERANCE
      ) {
        return false;
      }
    }
    return true;
  });
}

/**
 * Whether an image blob is one of the server's flat placeholder squares. Where images cannot be
 * decoded (tests, very old browsers) it is taken as a real thumbnail.
 */
export async function isPlaceholderImage(blob: Blob): Promise<boolean> {
  if (typeof createImageBitmap !== 'function' || typeof document === 'undefined') return false;
  try {
    const bitmap = await createImageBitmap(blob);
    const canvas = document.createElement('canvas');
    canvas.width = SAMPLE;
    canvas.height = SAMPLE;
    const ctx = canvas.getContext('2d');
    if (!ctx) return false;
    ctx.drawImage(bitmap, 0, 0, SAMPLE, SAMPLE);
    bitmap.close?.();
    return isPlaceholderPixels(ctx.getImageData(0, 0, SAMPLE, SAMPLE).data);
  } catch {
    return false;
  }
}

/**
 * Fetch one thumbnail and turn it into an object URL (owned by the caller), or null when the
 * server only has a placeholder square. Rejects when the request fails.
 */
export async function loadThumbnail(id: string, fetcher: Fetcher): Promise<string | null> {
  const res = await fetcher(id);
  const blob = new Blob([res.data]);
  if (await isPlaceholderImage(blob)) return null;
  return URL.createObjectURL(blob);
}

const cache = new Map<string, string>();
/** Documents whose thumbnail turned out to be a placeholder, so they are not fetched again. */
const none = new Set<string>();
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
    loadThumbnail(id, fetcher)
      .then((url) => {
        if (gen !== generation) {
          if (url) URL.revokeObjectURL(url);
          return null;
        }
        if (url) remember(id, url);
        else if (none.size < CACHE_SIZE) none.add(id);
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

/** True when the document is known to have only a placeholder, not a real thumbnail. */
export function hasNoThumbnail(id: string): boolean {
  return none.has(id);
}

/**
 * Ask for a document's thumbnail. `done` gets the object URL, or null when there is none.
 * Returns a cancel function; cancelling before the request starts removes it from the queue.
 */
export function requestThumbnail(id: string, fetcher: Fetcher, done: Waiter): () => void {
  const hit = cachedThumbnail(id);
  if (hit || none.has(id)) {
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
  none.clear();
  waiters.clear();
  queue.length = 0;
  active = 0;
  generation += 1;
}
