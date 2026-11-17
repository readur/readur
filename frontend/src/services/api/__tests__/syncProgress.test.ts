import { describe, expect, it } from 'vitest';
import { buildWebSocketUrl } from '../syncProgress';

describe('buildWebSocketUrl', () => {
  it('uses ws: and the page host on an http page, port included (the Vite dev server proxies it)', () => {
    expect(buildWebSocketUrl('src-1', { protocol: 'http:', host: 'localhost:5173' })).toBe(
      'ws://localhost:5173/api/sources/src-1/sync/progress/ws',
    );
  });

  it('uses wss: on an https page', () => {
    expect(buildWebSocketUrl('src-1', { protocol: 'https:', host: 'readur.example.com' })).toBe(
      'wss://readur.example.com/api/sources/src-1/sync/progress/ws',
    );
  });

  it('never points at a hard-coded backend port', () => {
    expect(buildWebSocketUrl('src-1', { protocol: 'http:', host: '127.0.0.1:5174' })).not.toContain(
      '8000',
    );
  });

  it('defaults to window.location', () => {
    const { protocol, host } = window.location;
    const ws = protocol === 'https:' ? 'wss:' : 'ws:';
    expect(buildWebSocketUrl('abc')).toBe(`${ws}//${host}/api/sources/abc/sync/progress/ws`);
  });

  it('encodes the source id', () => {
    expect(buildWebSocketUrl('a/b', { protocol: 'http:', host: 'h' })).toBe(
      'ws://h/api/sources/a%2Fb/sync/progress/ws',
    );
  });
});
