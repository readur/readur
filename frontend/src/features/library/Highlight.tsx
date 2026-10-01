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

const encoder = new TextEncoder();

/**
 * The backend reports highlight ranges as UTF-8 byte offsets into the snippet, while JavaScript
 * strings index UTF-16 code units, so the two differ as soon as the text has a non-ASCII letter.
 * Maps each byte range onto the snippet text; an offset inside a character snaps outwards to
 * that character's edge.
 */
export function byteRangesToUtf16(text: string, ranges: readonly Range[]): Range[] {
  // byteStart[i] / utf16Start[i] are where the i-th code point begins; a final entry marks the end.
  const byteStart: number[] = [];
  const utf16Start: number[] = [];
  let bytes = 0;
  let units = 0;
  for (const ch of text) {
    byteStart.push(bytes);
    utf16Start.push(units);
    bytes += encoder.encode(ch).length;
    units += ch.length;
  }
  byteStart.push(bytes);
  utf16Start.push(units);

  const toUnits = (offset: number, edge: 'start' | 'end') => {
    if (offset <= 0) return 0;
    if (offset >= bytes) return units;
    // Last code point that begins at or before the offset.
    let lo = 0;
    let hi = byteStart.length - 1;
    while (lo < hi) {
      const mid = (lo + hi + 1) >> 1;
      if (byteStart[mid] <= offset) lo = mid;
      else hi = mid - 1;
    }
    const inside = byteStart[lo] !== offset;
    return edge === 'end' && inside ? utf16Start[lo + 1] : utf16Start[lo];
  };

  return ranges.map((r) => ({ start: toUnits(r.start, 'start'), end: toUnits(r.end, 'end') }));
}

export function SnippetLine({ snippet }: { snippet: SearchSnippet }) {
  return (
    <span className={styles.snippet}>
      <HighlightedText text={snippet.text} ranges={byteRangesToUtf16(snippet.text, snippet.highlight_ranges)} />
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
