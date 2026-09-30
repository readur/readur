import type { ReactNode } from 'react';
import type { SearchSnippet } from '../../types/generated';
import styles from './Library.module.css';

interface Range {
  start: number;
  end: number;
}

/** Text with the given character ranges wrapped in <mark>. Overlapping or bad ranges are tidied. */
export function HighlightedText({ text, ranges }: { text: string; ranges: readonly Range[] }) {
  const clean = [...ranges]
    .map((r) => ({ start: Math.max(0, r.start), end: Math.min(text.length, r.end) }))
    .filter((r) => r.end > r.start)
    .sort((a, b) => a.start - b.start);
  const parts: ReactNode[] = [];
  let cursor = 0;
  clean.forEach((r, i) => {
    if (r.end <= cursor) return;
    const start = Math.max(r.start, cursor);
    if (start > cursor) parts.push(text.slice(cursor, start));
    parts.push(
      <mark key={i} className={styles.mark}>
        {text.slice(start, r.end)}
      </mark>,
    );
    cursor = r.end;
  });
  if (cursor < text.length) parts.push(text.slice(cursor));
  return <>{parts}</>;
}

/** The snippet with the most matches (the first on a tie). */
export function bestSnippet(snippets: readonly SearchSnippet[] | undefined): SearchSnippet | null {
  if (!snippets || snippets.length === 0) return null;
  return snippets.reduce((best, s) => (s.highlight_ranges.length > best.highlight_ranges.length ? s : best));
}

export function SnippetLine({ snippet }: { snippet: SearchSnippet }) {
  return (
    <span className={styles.snippet}>
      <HighlightedText text={snippet.text} ranges={snippet.highlight_ranges} />
    </span>
  );
}

/** Ranges where any word of `query` occurs in `text` (case-insensitive), for local highlighting. */
export function matchRanges(text: string, query: string): Range[] {
  const words = query
    .toLocaleLowerCase()
    .split(/[\s"'()*]+/)
    .filter((w) => w.length >= 2 && !['and', 'or', 'not'].includes(w));
  if (words.length === 0) return [];
  const haystack = text.toLocaleLowerCase();
  const ranges: Range[] = [];
  for (const word of words) {
    let from = 0;
    for (;;) {
      const at = haystack.indexOf(word, from);
      if (at === -1) break;
      ranges.push({ start: at, end: at + word.length });
      from = at + word.length;
    }
  }
  return ranges;
}
