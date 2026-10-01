import { useLayoutEffect, useState, type RefObject } from 'react';

/** Never shrink the reading area below this, however short the window. */
export const MIN_READING_HEIGHT = 360;
/** Space left under the reading area on desktop. */
export const BOTTOM_GAP = 16;
/** Below this width the shell has a bottom tab bar, and its padding must be kept clear. */
const BOTTOM_BAR_BELOW = 900;

/** Padding the app shell keeps under its content, summed over every ancestor. */
function bottomReserve(el: HTMLElement): number {
  let total = 0;
  for (let node = el.parentElement; node && node !== document.documentElement; node = node.parentElement) {
    const pad = parseFloat(window.getComputedStyle(node).paddingBottom);
    if (Number.isFinite(pad)) total += pad;
  }
  return total;
}

export interface FillHeight {
  /** Height that reaches the bottom of the window. */
  height?: number;
  /**
   * Negative bottom margin that lets the area use the shell's bottom padding on desktop, so it
   * ends a small gap above the window edge instead of a large one.
   */
  pullUp?: number;
}

/**
 * Sizes `ref` to run from where it starts on the page to the bottom of the window, so the
 * reading panes fill the screen and scroll on their own instead of the page scrolling around
 * them. Re-measured when the window resizes or anything above it (`watch`) changes size.
 * `key` re-attaches the measurement when the element first appears (e.g. after loading).
 */
export function useFillHeight(
  ref: RefObject<HTMLElement | null>,
  watch: RefObject<HTMLElement | null>,
  key: unknown,
): FillHeight {
  const [fill, setFill] = useState<FillHeight>({});

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return undefined;
    const measure = () => {
      const top = Math.ceil(el.getBoundingClientRect().top + window.scrollY);
      const reserve = bottomReserve(el);
      const pullUp = window.innerWidth >= BOTTOM_BAR_BELOW ? Math.max(0, reserve - BOTTOM_GAP) : 0;
      const available = Math.floor(window.innerHeight - top - reserve + pullUp);
      setFill((prev) => {
        const height = Math.max(MIN_READING_HEIGHT, available);
        return prev.height === height && prev.pullUp === pullUp ? prev : { height, pullUp };
      });
    };
    measure();
    window.addEventListener('resize', measure);
    let observer: ResizeObserver | undefined;
    if (typeof ResizeObserver !== 'undefined' && watch.current) {
      observer = new ResizeObserver(measure);
      observer.observe(watch.current);
    }
    return () => {
      window.removeEventListener('resize', measure);
      observer?.disconnect();
    };
  }, [ref, watch, key]);

  return fill;
}
