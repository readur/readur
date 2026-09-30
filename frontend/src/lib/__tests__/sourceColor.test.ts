import { describe, expect, it } from 'vitest';
import { UPLOAD_SLOT, WATCH_SLOT, sourceColorVar, sourceHue } from '../sourceColor';

describe('sourceHue', () => {
  it('gives uploads slot 1 and the watch folder slot 2', () => {
    expect(sourceHue(null, 'upload')).toEqual({ index: UPLOAD_SLOT, varName: '--src-1', softVarName: '--src-1-soft' });
    expect(sourceHue('anything', 'watch').index).toBe(WATCH_SLOT);
    expect(sourceHue('upload').index).toBe(1);
    expect(sourceHue('watch').index).toBe(2);
  });

  it('treats a missing id with no kind as an upload', () => {
    expect(sourceHue(null).index).toBe(1);
    expect(sourceHue(undefined).index).toBe(1);
    expect(sourceHue('').index).toBe(1);
  });

  it('is deterministic for the same id, whatever the kind', () => {
    const id = '5b0c2e6a-8f1e-4a55-9d36-2c7e7c0f4c11';
    expect(sourceHue(id, 'webdav')).toEqual(sourceHue(id, 'webdav'));
    expect(sourceHue(id, 's3').index).toBe(sourceHue(id, 'local_folder').index);
  });

  it('puts real sources in slots 3 to 8 and uses all of them', () => {
    const seen = new Set<number>();
    for (let i = 0; i < 200; i += 1) {
      const { index } = sourceHue(`source-${i}`, 'webdav');
      expect(index).toBeGreaterThanOrEqual(3);
      expect(index).toBeLessThanOrEqual(8);
      seen.add(index);
    }
    expect([...seen].sort()).toEqual([3, 4, 5, 6, 7, 8]);
  });

  it('builds a var() reference', () => {
    expect(sourceColorVar(null, 'watch')).toBe('var(--src-2)');
  });
});
