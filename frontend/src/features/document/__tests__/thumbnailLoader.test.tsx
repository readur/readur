import { act, render, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiModule from '../../../services/api';
import { DocumentThumbnail } from '../DocumentThumbnail';
import { MAX_IN_FLIGHT, isPlaceholderPixels, resetThumbnailLoader } from '../thumbnailLoader';
import type { ApiMock } from './mockApi';

vi.mock('../../../services/api', async () => (await import('./mockApi')).createApiMock());

const m = apiModule as unknown as ApiMock;

/** A controllable IntersectionObserver: `show(el)` reports the element as visible. */
class FakeObserver {
  static all: FakeObserver[] = [];
  elements: Element[] = [];
  constructor(private callback: IntersectionObserverCallback) {
    FakeObserver.all.push(this);
  }
  observe(el: Element) {
    this.elements.push(el);
  }
  unobserve() {}
  disconnect() {
    this.elements = [];
  }
  takeRecords() {
    return [];
  }
  static show(el: Element) {
    for (const o of FakeObserver.all) {
      if (o.elements.includes(el)) {
        o.callback([{ target: el, isIntersecting: true } as IntersectionObserverEntry], o as unknown as IntersectionObserver);
      }
    }
  }
}

/** getThumbnail that stays pending until the test resolves it, counting concurrency. */
function controlledFetch() {
  const pending: { id: string; resolve: () => void }[] = [];
  let inFlight = 0;
  let max = 0;
  m.documentService.getThumbnail.mockImplementation(
    (id: string) =>
      new Promise((resolve) => {
        inFlight += 1;
        max = Math.max(max, inFlight);
        pending.push({
          id,
          resolve: () => {
            inFlight -= 1;
            resolve({ data: new Blob(['png']) });
          },
        });
      }),
  );
  return { pending, max: () => max };
}

let urlCount = 0;
beforeEach(() => {
  urlCount = 0;
  Object.defineProperty(URL, 'createObjectURL', { configurable: true, writable: true, value: vi.fn(() => `blob:t${++urlCount}`) });
  Object.defineProperty(URL, 'revokeObjectURL', { configurable: true, writable: true, value: vi.fn() });
  resetThumbnailLoader();
  FakeObserver.all = [];
});

afterEach(() => {
  vi.unstubAllGlobals();
});

const list = (ids: string[]) => (
  <div>
    {ids.map((id) => (
      <DocumentThumbnail key={id} documentId={id} mimeType="application/pdf" size="small" lazy />
    ))}
  </div>
);

describe('lazy DocumentThumbnail', () => {
  it('fetches only thumbnails that come into view', async () => {
    vi.stubGlobal('IntersectionObserver', FakeObserver);
    m.documentService.getThumbnail.mockResolvedValue({ data: new Blob(['png']) });
    const { container } = render(list(['a', 'b', 'c']));
    expect(m.documentService.getThumbnail).not.toHaveBeenCalled();
    const holders = container.querySelectorAll('div > span');
    act(() => FakeObserver.show(holders[1]));
    await waitFor(() => expect(container.querySelector('img')).toHaveAttribute('src', 'blob:t1'));
    expect(m.documentService.getThumbnail).toHaveBeenCalledTimes(1);
    expect(m.documentService.getThumbnail).toHaveBeenCalledWith('b');
  });

  it(`never runs more than ${MAX_IN_FLIGHT} requests at once`, async () => {
    const ctl = controlledFetch();
    const ids = Array.from({ length: 10 }, (_, i) => `d${i}`);
    const { container } = render(list(ids));
    await waitFor(() => expect(ctl.pending).toHaveLength(MAX_IN_FLIGHT));
    while (ctl.pending.length > 0) {
      const next = ctl.pending.shift()!;
      await act(async () => next.resolve());
    }
    await waitFor(() => expect(container.querySelectorAll('img')).toHaveLength(10));
    expect(ctl.max()).toBe(MAX_IN_FLIGHT);
    expect(m.documentService.getThumbnail).toHaveBeenCalledTimes(10);
  });

  it('does not refetch a cached thumbnail when remounted', async () => {
    m.documentService.getThumbnail.mockResolvedValue({ data: new Blob(['png']) });
    const first = render(list(['a']));
    await waitFor(() => expect(first.container.querySelector('img')).toHaveAttribute('src', 'blob:t1'));
    first.unmount();
    const second = render(list(['a']));
    expect(second.container.querySelector('img')).toHaveAttribute('src', 'blob:t1');
    expect(m.documentService.getThumbnail).toHaveBeenCalledTimes(1);
  });

  it('drops queued requests for rows that unmount before their turn', async () => {
    const ctl = controlledFetch();
    const ids = Array.from({ length: 8 }, (_, i) => `d${i}`);
    const view = render(list(ids));
    await waitFor(() => expect(ctl.pending).toHaveLength(MAX_IN_FLIGHT));
    view.unmount();
    for (const p of [...ctl.pending]) await act(async () => p.resolve());
    expect(m.documentService.getThumbnail).toHaveBeenCalledTimes(MAX_IN_FLIGHT);
  });

  it('shows the type-code stub when there is no thumbnail', async () => {
    m.documentService.getThumbnail.mockRejectedValue(new Error('none'));
    const { container, findByText } = render(list(['a']));
    act(() => FakeObserver.show(container.querySelector('span')!));
    expect(await findByText('PDF')).toBeInTheDocument();
    await waitFor(() => expect(container.querySelector('[data-state="none"]')).toBeInTheDocument());
  });

  it('treats the server\'s flat placeholder square as no thumbnail, and remembers it', async () => {
    m.documentService.getThumbnail.mockResolvedValue({ data: new Blob(['jpg']) });
    // A decodable image whose every pixel is the server's PDF red.
    const red = new Uint8ClampedArray(12 * 12 * 4).map((_, i) => [220, 38, 27, 255][i % 4]);
    vi.stubGlobal('createImageBitmap', vi.fn(async () => ({ close: vi.fn() })));
    const getContext = vi
      .spyOn(HTMLCanvasElement.prototype, 'getContext')
      .mockReturnValue({ drawImage: vi.fn(), getImageData: () => ({ data: red }) } as unknown as CanvasRenderingContext2D);
    const first = render(list(['a']));
    act(() => FakeObserver.show(first.container.querySelector('span')!));
    await waitFor(() => expect(first.container.querySelector('[data-state="none"]')).toBeInTheDocument());
    expect(first.container.querySelector('img')).toBeNull();
    expect(URL.createObjectURL).not.toHaveBeenCalled();
    first.unmount();
    // Remounting knows the answer without asking again.
    const second = render(list(['a']));
    expect(second.container.querySelector('[data-state="none"]')).toBeInTheDocument();
    expect(m.documentService.getThumbnail).toHaveBeenCalledTimes(1);
    getContext.mockRestore();
  });
});

describe('isPlaceholderPixels', () => {
  const fill = (rgb: number[], n = 16) => Array.from({ length: n * 4 }, (_, i) => (i % 4 === 3 ? 255 : rgb[i % 4]));

  it('recognises each flat placeholder colour, allowing for JPEG noise', () => {
    expect(isPlaceholderPixels(fill([220, 38, 27]))).toBe(true);
    expect(isPlaceholderPixels(fill([41, 128, 185]))).toBe(true);
    expect(isPlaceholderPixels(fill([34, 139, 34]))).toBe(true);
    expect(isPlaceholderPixels(fill([108, 117, 125]))).toBe(true);
    expect(isPlaceholderPixels(fill([228, 30, 35]))).toBe(true);
  });

  it('keeps real previews: a white page, or anything with detail', () => {
    expect(isPlaceholderPixels(fill([255, 255, 255]))).toBe(false);
    const page = fill([220, 38, 27]);
    page.splice(0, 4, 0, 0, 0, 255);
    expect(isPlaceholderPixels(page)).toBe(false);
    expect(isPlaceholderPixels([])).toBe(false);
  });
});
