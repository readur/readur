import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { SearchField } from '../../ui';
import { MIN_QUERY } from './urlState';
import styles from './Library.module.css';

export const SEARCH_DEBOUNCE_MS = 250;

const TYPING = 'input, textarea, select, [contenteditable=""], [contenteditable="true"]';

interface SearchBoxProps {
  value: string;
  onChange: (q: string) => void;
  /** Focus the field on mount with the caret after the text (typing carried over from another page). */
  autoFocus?: boolean;
  /** The Search page's large field. */
  size?: 'md' | 'lg';
  placeholder?: string;
  /** `typing` searches after a pause; `submit` only on Enter (a field that leads to another page). */
  commitOn?: 'typing' | 'submit';
}

/**
 * The Library's search field. Typing is debounced; Enter and clearing apply at once; `/`
 * anywhere outside a text field focuses it (ahead of the command palette's `/`).
 */
export function SearchBox({ value, onChange, autoFocus = false, size = 'md', placeholder, commitOn = 'typing' }: SearchBoxProps) {
  const { t } = useTranslation();
  const [text, setText] = useState(value);
  const wrapRef = useRef<HTMLDivElement>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  // Follow the URL when it changes from elsewhere (Back, an example query, Clear all).
  useEffect(() => {
    setText((prev) => (prev.trim() === value ? prev : value));
  }, [value]);

  useEffect(() => () => {
    if (timer.current) clearTimeout(timer.current);
  }, []);

  useEffect(() => {
    if (!autoFocus) return;
    const input = wrapRef.current?.querySelector('input');
    if (!input) return;
    input.focus();
    const end = input.value.length;
    input.setSelectionRange(end, end);
    // Only on mount: later renders must not steal focus back.
  }, []);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key !== '/' || e.metaKey || e.ctrlKey || e.altKey || e.defaultPrevented) return;
      const target = e.target as Element | null;
      if (target?.closest?.(TYPING)) return;
      const input = wrapRef.current?.querySelector('input');
      if (!input) return;
      e.preventDefault();
      input.focus();
    };
    // Capture phase, so this runs before the palette's window listener and claims the key.
    window.addEventListener('keydown', onKeyDown, true);
    return () => window.removeEventListener('keydown', onKeyDown, true);
  }, []);

  const commit = (next: string) => {
    if (timer.current) clearTimeout(timer.current);
    timer.current = null;
    if (next.trim() !== value.trim()) onChangeRef.current(next.trim());
  };

  const trimmed = text.trim();
  const showHint = commitOn === 'typing' && trimmed.length > 0 && trimmed.length < MIN_QUERY;

  return (
    <div ref={wrapRef} className={size === 'lg' ? `${styles.search} ${styles.searchLarge}` : styles.search}>
      <SearchField
        aria-label={t('library.search.label', 'Search documents')}
        placeholder={placeholder ?? t('library.search.placeholder', 'Search names and text…')}
        value={text}
        onChange={(next) => {
          setText(next);
          if (timer.current) clearTimeout(timer.current);
          if (commitOn === 'typing') timer.current = setTimeout(() => commit(next), SEARCH_DEBOUNCE_MS);
        }}
        onSubmit={(next) => commit(next)}
        onClear={() => commit('')}
        description={showHint ? t('library.search.keepTyping', 'Keep typing…') : undefined}
      />
    </div>
  );
}
