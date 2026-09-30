import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

type LitModule = typeof import('../litStore');

class MemoryStorage implements Storage {
  private data = new Map<string, string>();
  get length() {
    return this.data.size;
  }
  clear() {
    this.data.clear();
  }
  getItem(key: string) {
    return this.data.has(key) ? (this.data.get(key) as string) : null;
  }
  key(i: number) {
    return Array.from(this.data.keys())[i] ?? null;
  }
  removeItem(key: string) {
    this.data.delete(key);
  }
  setItem(key: string, value: string) {
    this.data.set(key, String(value));
  }
}

let storage: MemoryStorage;
const originalStorage = window.localStorage;

const installStorage = (s: Storage) =>
  Object.defineProperty(window, 'localStorage', { value: s, configurable: true, writable: true });

/** Fresh module instance, as after a page reload. */
const loadModule = async (): Promise<LitModule> => {
  vi.resetModules();
  return import('../litStore');
};

beforeEach(() => {
  storage = new MemoryStorage();
  installStorage(storage);
});

afterEach(() => {
  installStorage(originalStorage);
});

describe('litStore', () => {
  it('is safe to read before any entry exists', async () => {
    const lit = await loadModule();
    expect(lit.isLit('document', 'x')).toBe(false);
    const { result } = renderHook(() => lit.useLitCount());
    expect(result.current).toBe(0);
    expect(() => lit.acknowledge('document', 'x')).not.toThrow();
    expect(() => lit.acknowledgeAll()).not.toThrow();
  });

  it('marks and acknowledges one entry', async () => {
    const lit = await loadModule();
    lit.markLit('document', 'a', 'new');
    expect(lit.isLit('document', 'a')).toBe(true);
    expect(lit.isLit('source', 'a')).toBe(false);
    lit.acknowledge('document', 'a');
    expect(lit.isLit('document', 'a')).toBe(false);
  });

  it('acknowledgeAll clears one kind or everything', async () => {
    const lit = await loadModule();
    lit.markLit('document', 'a', 'new');
    lit.markLit('document', 'b', 'changed');
    lit.markLit('source', 's1', 'failed');
    lit.markLit('attention', 'x', 'failed');

    lit.acknowledgeAll('document');
    expect(lit.isLit('document', 'a')).toBe(false);
    expect(lit.isLit('document', 'b')).toBe(false);
    expect(lit.isLit('source', 's1')).toBe(true);

    lit.acknowledgeAll();
    expect(lit.isLit('source', 's1')).toBe(false);
    expect(lit.isLit('attention', 'x')).toBe(false);
  });

  it('useLit reports the reason and re-renders on change', async () => {
    const lit = await loadModule();
    const { result } = renderHook(() => lit.useLit('document', 'a'));
    expect(result.current).toEqual({ lit: false });

    act(() => lit.markLit('document', 'a', 'failed'));
    expect(result.current).toEqual({ lit: true, reason: 'failed' });

    act(() => lit.markLit('document', 'a', 'changed'));
    expect(result.current).toEqual({ lit: true, reason: 'changed' });

    act(() => lit.acknowledge('document', 'a'));
    expect(result.current).toEqual({ lit: false });
  });

  it('useLitCount counts per kind and in total', async () => {
    const lit = await loadModule();
    const total = renderHook(() => lit.useLitCount());
    const docs = renderHook(() => lit.useLitCount('document'));

    act(() => {
      lit.markLit('document', 'a', 'new');
      lit.markLit('document', 'b', 'new');
      lit.markLit('source', 's', 'failed');
    });
    expect(total.result.current).toBe(3);
    expect(docs.result.current).toBe(2);

    act(() => lit.acknowledge('document', 'a'));
    expect(total.result.current).toBe(2);
    expect(docs.result.current).toBe(1);
  });

  it('persists entries across store re-creation and drops acknowledged ones', async () => {
    let lit = await loadModule();
    lit.markLit('document', 'a', 'new');
    lit.markLit('source', 's', 'failed');
    lit.acknowledge('document', 'a');
    expect(storage.getItem(lit.LIT_STORAGE_KEY)).not.toBeNull();

    lit = await loadModule();
    expect(lit.isLit('source', 's')).toBe(true);
    expect(lit.isLit('document', 'a')).toBe(false);
    const { result } = renderHook(() => lit.useLit('source', 's'));
    expect(result.current).toEqual({ lit: true, reason: 'failed' });
  });

  it('uses the readur.lit.v1 storage key', async () => {
    const lit = await loadModule();
    expect(lit.LIT_STORAGE_KEY).toBe('readur.lit.v1');
    lit.markLit('attention', 'q', 'failed');
    expect(storage.getItem('readur.lit.v1')).toContain('"q"');
  });

  it('caps at 2000 entries by dropping the oldest', async () => {
    let lit = await loadModule();
    for (let i = 0; i < 2005; i += 1) lit.markLit('document', `d${i}`, 'new');
    const { result } = renderHook(() => lit.useLitCount());
    expect(result.current).toBe(2000);
    expect(lit.isLit('document', 'd0')).toBe(false);
    expect(lit.isLit('document', 'd4')).toBe(false);
    expect(lit.isLit('document', 'd5')).toBe(true);
    expect(lit.isLit('document', 'd2004')).toBe(true);

    lit = await loadModule();
    expect(lit.isLit('document', 'd4')).toBe(false);
    expect(lit.isLit('document', 'd2004')).toBe(true);
  });

  it('re-marking an entry makes it the newest, so it survives the cap', async () => {
    const lit = await loadModule();
    for (let i = 0; i < 2000; i += 1) lit.markLit('document', `d${i}`, 'new');
    lit.markLit('document', 'd0', 'changed');
    lit.markLit('document', 'extra', 'new');
    expect(lit.isLit('document', 'd0')).toBe(true);
    expect(lit.isLit('document', 'd1')).toBe(false);
  });

  it('ignores corrupted storage without throwing', async () => {
    storage.setItem('readur.lit.v1', '{not json');
    let lit = await loadModule();
    expect(() => lit.isLit('document', 'a')).not.toThrow();
    expect(lit.isLit('document', 'a')).toBe(false);
    lit.markLit('document', 'a', 'new');
    expect(lit.isLit('document', 'a')).toBe(true);

    storage.setItem('readur.lit.v1', JSON.stringify([['bogus', 1], ['document', 'ok', 'new', 1]]));
    lit = await loadModule();
    expect(lit.isLit('document', 'ok')).toBe(true);
  });

  it('keeps working when storage throws', async () => {
    const broken = new MemoryStorage();
    broken.getItem = () => {
      throw new Error('denied');
    };
    broken.setItem = () => {
      throw new Error('quota');
    };
    installStorage(broken);
    const lit = await loadModule();
    expect(() => lit.markLit('document', 'a', 'new')).not.toThrow();
    expect(lit.isLit('document', 'a')).toBe(true);
  });

  it('createLitStore gives isolated instances that share storage', async () => {
    const { createLitStore } = await loadModule();
    const one = createLitStore();
    one.markLit('source', 's', 'changed');
    const two = createLitStore();
    expect(two.isLit('source', 's')).toBe(true);
    expect(two.reasonOf('source', 's')).toBe('changed');
    expect(two.count('source')).toBe(1);
  });
});
