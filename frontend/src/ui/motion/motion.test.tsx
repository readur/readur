import { useRef } from 'react';
import { render, renderHook, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { ThemeModeContext, type ThemeModeValue } from '../../theme/useThemeMode';
import { useFlip } from './useFlip';
import { usePrefersReducedMotion } from './usePrefersReducedMotion';

const ROW = 40;
const originalMatchMedia = window.matchMedia;
const originalRect = HTMLElement.prototype.getBoundingClientRect;

function mockReducedMotion(reduce: boolean) {
  Object.defineProperty(window, 'matchMedia', {
    configurable: true,
    writable: true,
    value: (query: string) => ({
      matches: query.includes('prefers-reduced-motion') ? reduce : false,
      media: query,
      addEventListener: () => {},
      removeEventListener: () => {},
      addListener: () => {},
      removeListener: () => {},
      dispatchEvent: () => false,
    }),
  });
}

/** Lays rows out top-to-bottom by DOM order so positions change when the order does. */
function mockLayout() {
  HTMLElement.prototype.getBoundingClientRect = function (this: HTMLElement) {
    const key = this.getAttribute('data-flip-key');
    let top = 0;
    if (key !== null && this.parentElement) {
      top = Array.from(this.parentElement.children).indexOf(this) * ROW;
    }
    return { top, left: 0, right: 100, bottom: top + ROW, width: 100, height: ROW, x: 0, y: top, toJSON: () => ({}) } as DOMRect;
  };
}

function List({ keys }: { keys: string[] }) {
  const ref = useRef<HTMLUListElement>(null);
  useFlip(keys, ref);
  return (
    <ul ref={ref} aria-label="items">
      {keys.map((k) => (
        <li key={k} data-flip-key={k}>
          {k}
        </li>
      ))}
    </ul>
  );
}

describe('useFlip', () => {
  beforeEach(() => mockLayout());
  afterEach(() => {
    HTMLElement.prototype.getBoundingClientRect = originalRect;
    Object.defineProperty(window, 'matchMedia', { configurable: true, writable: true, value: originalMatchMedia });
  });

  it('animates moved rows from their previous position', () => {
    mockReducedMotion(false);
    const { rerender } = render(<List keys={['a', 'b', 'c']} />);
    rerender(<List keys={['c', 'a', 'b']} />);
    // 'c' moved from index 2 to 0; it plays from +80px back to 0 with a transition.
    const c = screen.getByText('c');
    expect(c.style.transition).toContain('transform');
    expect(c.style.transform).toBe('');
  });

  it('does not animate keys that are new', () => {
    mockReducedMotion(false);
    const { rerender } = render(<List keys={['a']} />);
    rerender(<List keys={['z', 'a']} />);
    expect(screen.getByText('z').style.transition).toBe('');
    expect(screen.getByText('a').style.transition).toContain('transform');
  });

  it('is a no-op under reduced motion', () => {
    mockReducedMotion(true);
    const { rerender } = render(<List keys={['a', 'b', 'c']} />);
    rerender(<List keys={['c', 'b', 'a']} />);
    for (const k of ['a', 'b', 'c']) {
      expect(screen.getByText(k).style.transition).toBe('');
      expect(screen.getByText(k).style.transform).toBe('');
    }
  });

  it('honours the theme provider value over the media query', () => {
    mockReducedMotion(false);
    const theme: ThemeModeValue = { mode: 'light', setMode: () => {}, toggle: () => {}, prefersReducedMotion: true };
    const { rerender } = render(
      <ThemeModeContext.Provider value={theme}>
        <List keys={['a', 'b']} />
      </ThemeModeContext.Provider>,
    );
    rerender(
      <ThemeModeContext.Provider value={theme}>
        <List keys={['b', 'a']} />
      </ThemeModeContext.Provider>,
    );
    expect(screen.getByText('a').style.transition).toBe('');
  });
});

describe('usePrefersReducedMotion', () => {
  afterEach(() => {
    Object.defineProperty(window, 'matchMedia', { configurable: true, writable: true, value: originalMatchMedia });
  });

  it('falls back to the media query without a theme provider', () => {
    mockReducedMotion(true);
    expect(renderHook(() => usePrefersReducedMotion()).result.current).toBe(true);
    mockReducedMotion(false);
    expect(renderHook(() => usePrefersReducedMotion()).result.current).toBe(false);
  });
});
