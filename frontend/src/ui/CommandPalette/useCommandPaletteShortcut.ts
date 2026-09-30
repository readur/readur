import { useEffect, useRef } from 'react';

const TYPING = 'input, textarea, select, [contenteditable=""], [contenteditable="true"]';

function isTyping(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false;
  return Boolean(target.closest(TYPING));
}

/**
 * Binds ⌘K / Ctrl+K anywhere, and `/` when focus is not in a text field, to `open`.
 */
export function useCommandPaletteShortcut(open: () => void): void {
  const openRef = useRef(open);
  openRef.current = open;

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.defaultPrevented || e.isComposing) return;
      const key = e.key.toLowerCase();
      if (key === 'k' && (e.metaKey || e.ctrlKey) && !e.altKey && !e.shiftKey) {
        e.preventDefault();
        openRef.current();
        return;
      }
      if (e.key === '/' && !e.metaKey && !e.ctrlKey && !e.altKey && !isTyping(e.target)) {
        e.preventDefault();
        openRef.current();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, []);
}
