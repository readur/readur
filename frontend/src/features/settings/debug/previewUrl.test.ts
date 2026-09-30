import { afterEach, describe, expect, it, vi } from 'vitest';
import { previewObjectUrl } from './previewUrl';

describe('previewObjectUrl', () => {
  afterEach(() => vi.restoreAllMocks());

  it('returns a blob URL for a raster image', () => {
    URL.createObjectURL = vi.fn(() => 'blob:http://localhost/abc');
    expect(previewObjectUrl(new File(['x'], 'a.png', { type: 'image/png' }))).toBe('blob:http://localhost/abc');
  });

  it('never previews hostile names or types, and never builds a URL from them', () => {
    const create = vi.fn(() => 'blob:x');
    URL.createObjectURL = create;
    const hostile = '"><img src=x onerror=alert(1)>.png';
    expect(previewObjectUrl(new File(['x'], hostile, { type: 'image/svg+xml' }))).toBeNull();
    expect(previewObjectUrl(new File(['x'], hostile, { type: 'image/png"><script>' }))).toBeNull();
    expect(create).not.toHaveBeenCalled();
  });

  it('rejects a non-blob URL', () => {
    URL.createObjectURL = vi.fn(() => 'javascript:alert(1)');
    expect(previewObjectUrl(new File(['x'], 'a.png', { type: 'image/png' }))).toBeNull();
  });
});
