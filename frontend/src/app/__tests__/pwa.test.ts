import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const root = resolve(__dirname, '../../..');
const html = readFileSync(resolve(root, 'index.html'), 'utf8');
const publicFile = (path: string) => resolve(root, 'public', path.replace(/^\//, ''));

describe('installable app (home screen, no browser bars)', () => {
  it('links a manifest that opens standalone at Home, with icons that exist', () => {
    const link = html.match(/<link rel="manifest" href="([^"]+)"/);
    expect(link).not.toBeNull();
    const manifest = JSON.parse(readFileSync(publicFile(link![1]), 'utf8'));
    expect(manifest).toMatchObject({ short_name: 'Readur', display: 'standalone', start_url: '/home', scope: '/' });
    const sizes = manifest.icons.map((i: { sizes: string }) => i.sizes);
    expect(sizes).toEqual(expect.arrayContaining(['192x192', '512x512']));
    expect(manifest.icons.some((i: { purpose?: string }) => i.purpose === 'maskable')).toBe(true);
    for (const icon of manifest.icons) expect(existsSync(publicFile(icon.src))).toBe(true);
  });

  it('tells iOS to run it full screen with its own icon and title', () => {
    expect(html).toContain('<meta name="apple-mobile-web-app-capable" content="yes" />');
    expect(html).toContain('<meta name="mobile-web-app-capable" content="yes" />');
    expect(html).toContain('<meta name="apple-mobile-web-app-title" content="Readur" />');
    expect(html).toMatch(/<meta name="apple-mobile-web-app-status-bar-style" content="[a-z-]+" \/>/);
    const touch = html.match(/<link rel="apple-touch-icon" href="([^"]+)"/);
    expect(touch).not.toBeNull();
    expect(existsSync(publicFile(touch![1]))).toBe(true);
  });

  it('lets the page reach the screen edges so the tab bar can pad for the home indicator', () => {
    expect(html).toMatch(/<meta name="viewport" content="[^"]*viewport-fit=cover/);
  });
});
