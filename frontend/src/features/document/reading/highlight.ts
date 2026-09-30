export interface TextSegment {
  text: string;
  /** Index of the match among all matches, or -1 for plain text. */
  match: number;
}

const escapeRegExp = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

/** Splits `text` into plain and matching segments for a case-insensitive `query`. */
export function splitMatches(text: string, query: string): TextSegment[] {
  const needle = query.trim();
  if (!needle || !text) return [{ text, match: -1 }];
  const re = new RegExp(escapeRegExp(needle), 'gi');
  const out: TextSegment[] = [];
  let last = 0;
  let n = 0;
  for (let m = re.exec(text); m; m = re.exec(text)) {
    if (m.index > last) out.push({ text: text.slice(last, m.index), match: -1 });
    out.push({ text: m[0], match: n });
    n += 1;
    last = m.index + m[0].length;
  }
  if (last < text.length) out.push({ text: text.slice(last), match: -1 });
  return out;
}

export function countMatches(segments: TextSegment[]): number {
  return segments.reduce((n, s) => (s.match >= 0 ? n + 1 : n), 0);
}
