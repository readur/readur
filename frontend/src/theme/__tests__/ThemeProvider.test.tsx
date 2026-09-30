import React from 'react';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, act } from '@testing-library/react';
import { ThemeModeProvider } from '../ThemeProvider';
import { useThemeMode } from '../useThemeMode';

type Listener = (e: { matches: boolean }) => void;

interface FakeMql {
  matches: boolean;
  listeners: Set<Listener>;
  addEventListener: (t: string, l: Listener) => void;
  removeEventListener: (t: string, l: Listener) => void;
}

let queries: Record<string, FakeMql>;

const installMatchMedia = (initial: Record<string, boolean>) => {
  queries = {};
  window.matchMedia = vi.fn((q: string) => {
    if (!queries[q]) {
      const listeners = new Set<Listener>();
      queries[q] = {
        matches: initial[q] ?? false,
        listeners,
        addEventListener: (_t, l) => listeners.add(l),
        removeEventListener: (_t, l) => listeners.delete(l),
      };
    }
    return queries[q];
  }) as unknown as typeof window.matchMedia;
};

const fire = (q: string, matches: boolean) => {
  queries[q].matches = matches;
  act(() => queries[q].listeners.forEach((l) => l({ matches })));
};

const DARK = '(prefers-color-scheme: dark)';
const MOTION = '(prefers-reduced-motion: reduce)';

const Probe = () => {
  const { mode, toggle, prefersReducedMotion } = useThemeMode();
  return (
    <div>
      <span data-testid="mode">{mode}</span>
      <span data-testid="motion">{String(prefersReducedMotion)}</span>
      <button onClick={toggle}>toggle</button>
    </div>
  );
};

const mount = () =>
  render(
    <ThemeModeProvider>
      <Probe />
    </ThemeModeProvider>,
  );

const originalMatchMedia = window.matchMedia;

describe('ThemeModeProvider', () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute('data-theme');
  });

  afterEach(() => {
    window.matchMedia = originalMatchMedia;
  });

  it('defaults to the system preference', () => {
    installMatchMedia({ [DARK]: true });
    mount();
    expect(screen.getByTestId('mode').textContent).toBe('dark');
    expect(document.documentElement.dataset.theme).toBe('dark');
  });

  it('defaults to light when the system is not dark', () => {
    installMatchMedia({ [DARK]: false });
    mount();
    expect(screen.getByTestId('mode').textContent).toBe('light');
  });

  it('prefers the saved themeMode over the system', () => {
    installMatchMedia({ [DARK]: true });
    localStorage.setItem('themeMode', 'light');
    mount();
    expect(screen.getByTestId('mode').textContent).toBe('light');
    expect(document.documentElement.dataset.theme).toBe('light');
  });

  it('toggle persists and updates data-theme', () => {
    installMatchMedia({ [DARK]: false });
    mount();
    act(() => screen.getByText('toggle').click());
    expect(screen.getByTestId('mode').textContent).toBe('dark');
    expect(localStorage.getItem('themeMode')).toBe('dark');
    expect(document.documentElement.dataset.theme).toBe('dark');
    act(() => screen.getByText('toggle').click());
    expect(document.documentElement.dataset.theme).toBe('light');
  });

  it('follows system changes only while nothing is saved', () => {
    installMatchMedia({ [DARK]: false });
    mount();
    fire(DARK, true);
    expect(screen.getByTestId('mode').textContent).toBe('dark');

    act(() => screen.getByText('toggle').click()); // saves 'light'
    fire(DARK, true);
    expect(screen.getByTestId('mode').textContent).toBe('light');
  });

  it('reflects prefers-reduced-motion live', () => {
    installMatchMedia({ [MOTION]: true });
    mount();
    expect(screen.getByTestId('motion').textContent).toBe('true');
    fire(MOTION, false);
    expect(screen.getByTestId('motion').textContent).toBe('false');
  });
});
