export interface TextSegment {
  text: string;
  /** Index of the match among all matches, or -1 for plain text. */
  match: number;
}

const escapeRegExp = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

/**
 * The words of a query: quoted phrases stay whole, everything else splits on whitespace.
 * Longest first, so "injury" wins over "in" where both would match at the same place.
 */
export function queryTerms(query: string): string[] {
  const seen = new Set<string>();
  const terms: string[] = [];
  for (const m of query.matchAll(/"([^"]+)"|(\S+)/g)) {
    const term = (m[1] ?? m[2] ?? '').trim();
    const key = term.toLowerCase();
    if (term && !seen.has(key)) {
      seen.add(key);
      terms.push(term);
    }
  }
  return terms.sort((a, b) => b.length - a.length);
}

function split(text: string, re: RegExp): TextSegment[] {
  const out: TextSegment[] = [];
  let last = 0;
  let n = 0;
  for (let m = re.exec(text); m; m = re.exec(text)) {
    if (m[0] === '') {
      re.lastIndex += 1;
      continue;
    }
    if (m.index > last) out.push({ text: text.slice(last, m.index), match: -1 });
    out.push({ text: m[0], match: n });
    n += 1;
    last = m.index + m[0].length;
  }
  if (last < text.length) out.push({ text: text.slice(last), match: -1 });
  return out;
}

/**
 * Splits `text` into plain and matching segments for a case-insensitive `query`. The whole query
 * is tried first as a literal phrase; when the phrase does not occur, each of its words is
 * matched on its own, so a search for "shoulder injury" still lights up a page that only says
 * "injury to the left shoulder".
 */
export function splitMatches(text: string, query: string): TextSegment[] {
  const needle = query.trim();
  if (!needle || !text) return [{ text, match: -1 }];
  const phrase = split(text, new RegExp(escapeRegExp(needle.replace(/^"|"$/g, '')), 'gi'));
  if (countMatches(phrase) > 0) return phrase;
  const terms = queryTerms(needle);
  if (terms.length < 2) return phrase;
  return split(text, new RegExp(terms.map(escapeRegExp).join('|'), 'gi'));
}

export function countMatches(segments: TextSegment[]): number {
  return segments.reduce((n, s) => (s.match >= 0 ? n + 1 : n), 0);
}

/** One or more blank lines: a paragraph break. */
const PARAGRAPH_BREAK = /\n[^\S\n]*\n\s*/;

/**
 * Groups segments into paragraphs for display. Any run of blank lines becomes one paragraph
 * gap and single newlines stay as line breaks inside a paragraph. This only shapes what is
 * shown: the raw text is left untouched for copying.
 */
export function toParagraphs(segments: TextSegment[]): TextSegment[][] {
  const paragraphs: TextSegment[][] = [[]];
  for (const segment of segments) {
    if (segment.match >= 0) {
      paragraphs[paragraphs.length - 1].push(segment);
      continue;
    }
    segment.text.split(PARAGRAPH_BREAK).forEach((part, i) => {
      if (i > 0) paragraphs.push([]);
      if (part) paragraphs[paragraphs.length - 1].push({ text: part, match: -1 });
    });
  }
  return paragraphs.map(trimParagraph).filter((p) => p.length > 0);
}

function trimParagraph(paragraph: TextSegment[]): TextSegment[] {
  const out = [...paragraph];
  if (out.length && out[0].match < 0) out[0] = { ...out[0], text: out[0].text.replace(/^\s+/, '') };
  const end = out.length - 1;
  if (end >= 0 && out[end].match < 0) out[end] = { ...out[end], text: out[end].text.replace(/\s+$/, '') };
  return out.filter((s) => s.text !== '');
}
