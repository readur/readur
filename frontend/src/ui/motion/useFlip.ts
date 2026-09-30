import { useLayoutEffect, useRef, type RefObject } from 'react';
import { usePrefersReducedMotion } from './usePrefersReducedMotion';

export const FLIP_ATTR = 'data-flip-key';

function measure(container: HTMLElement, elements: HTMLElement[]): Map<string, number> {
  const origin = container.getBoundingClientRect().top;
  const tops = new Map<string, number>();
  elements.forEach((el) => {
    const key = el.getAttribute(FLIP_ATTR);
    if (key !== null) tops.set(key, el.getBoundingClientRect().top - origin);
  });
  return tops;
}

/**
 * FLIP reorder animation. After each change to `keys`, every element inside `containerRef`
 * that carries `data-flip-key` and was already present slides from its previous position to
 * its new one. Elements with a new key appear in place. Does nothing under reduced motion.
 */
export function useFlip(keys: readonly string[], containerRef: RefObject<HTMLElement | null>): void {
  const reduced = usePrefersReducedMotion();
  const previous = useRef<Map<string, number> | null>(null);
  const signature = keys.join('\u0000');

  useLayoutEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const elements = Array.from(container.querySelectorAll<HTMLElement>(`[${FLIP_ATTR}]`));
    // Drop in-flight transforms so we measure the settled layout.
    elements.forEach((el) => {
      el.style.transition = '';
      el.style.transform = '';
    });
    const before = previous.current;
    const after = measure(container, elements);
    previous.current = after;
    if (reduced || !before) return;

    elements.forEach((el) => {
      const key = el.getAttribute(FLIP_ATTR);
      if (key === null || !before.has(key)) return;
      const delta = (before.get(key) ?? 0) - (after.get(key) ?? 0);
      if (Math.abs(delta) < 0.5) return;
      el.style.transform = `translateY(${delta}px)`;
      // Commit the inverted position before playing towards the new one.
      void el.getBoundingClientRect();
      el.style.transition = 'transform var(--dur-2) var(--ease)';
      el.style.transform = '';
      const done = () => {
        el.style.transition = '';
        el.removeEventListener('transitionend', done);
      };
      el.addEventListener('transitionend', done);
    });
    // `signature` encodes the key order; `keys` is usually a fresh array each render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [signature, reduced, containerRef]);
}
