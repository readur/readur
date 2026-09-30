import { describe, test, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { HighlightedText, SnippetLine, bestSnippet, byteRangesToUtf16, matchRanges } from '../Highlight';
import { shortType } from '../../../lib/fileType';
import { NO_MIME_MATCH, groupOf, mimeTypesFor } from '../mime';
import { DEFAULT_SIZE, parseQuery, toParams, type LibraryQuery } from '../urlState';
import { formatBytes, formatRelative } from '../format';

const parse = (qs: string) => parseQuery(new URLSearchParams(qs));

describe('URL state', () => {
  test('defaults: newest first, 50 per page, page 1', () => {
    const q = parse('');
    expect(q).toMatchObject({ q: '', sort: 'created_at', order: 'desc', sortExplicit: false, page: 1, size: DEFAULT_SIZE });
  });

  test('reads every parameter', () => {
    const q = parse('q=tax&sort=file_size&order=asc&type=pdf,image&labels=a,b&status=failed&source=s1,s2&from=2026-01-01&to=2026-02-01&page=3&size=100&mode=fuzzy');
    expect(q).toEqual({
      q: 'tax',
      sort: 'file_size',
      order: 'asc',
      sortExplicit: true,
      relevance: false,
      types: ['pdf', 'image'],
      labels: ['a', 'b'],
      status: 'failed',
      sources: ['s1', 's2'],
      from: '2026-01-01',
      to: '2026-02-01',
      page: 3,
      size: 100,
      mode: 'fuzzy',
    });
  });

  test('sort=relevance asks for best matches first and survives the round trip', () => {
    const q = parse('q=knee&sort=relevance');
    expect(q).toMatchObject({ relevance: true, sortExplicit: false, sort: 'created_at' });
    expect(toParams(q).toString()).toBe('q=knee&sort=relevance');
  });

  test('label= (the collection link) is read as a collection filter', () => {
    expect(parse('label=a').labels).toEqual(['a']);
    expect(parse('label=a&labels=a,b').labels).toEqual(['a', 'b']);
    expect(toParams(parse('label=a')).toString()).toBe('labels=a');
  });

  test('drops values it does not know', () => {
    const q = parse('sort=x&order=y&type=pdf,zip&status=weird&from=yesterday&page=-2&size=7&mode=regex');
    expect(q).toMatchObject({ sort: 'created_at', order: 'desc', sortExplicit: false, types: ['pdf'], status: null, from: null, page: 1, size: 50, mode: null });
  });

  test('round-trips through the URL, leaving defaults out', () => {
    const qs = 'q=tax&sort=filename&order=asc&type=office&labels=a&status=pending&source=uploaded&from=2026-01-01&to=2026-01-31&mode=phrase&page=2&size=25';
    expect(toParams(parse(qs)).toString()).toBe(qs);
    expect(toParams(parse('')).toString()).toBe('');
    expect(toParams({ ...parse(''), mode: 'simple' } as LibraryQuery).toString()).toBe('');
  });
});

describe('file types', () => {
  test.each([
    ['application/pdf', 'pdf'],
    ['image/tiff', 'image'],
    ['application/vnd.openxmlformats-officedocument.spreadsheetml.sheet', 'office'],
    ['application/msword', 'office'],
    ['text/csv', 'text'],
    ['application/zip', 'other'],
  ])('%s is in the %s group', (mime, group) => {
    expect(groupOf(mime)).toBe(group);
  });

  test('expands groups using the types present in the library', () => {
    expect(mimeTypesFor(['image'], ['application/pdf', 'image/png', 'image/jpeg'])).toEqual(['image/png', 'image/jpeg']);
    expect(mimeTypesFor(['other'], ['application/pdf', 'application/zip'])).toEqual(['application/zip']);
  });

  test('falls back to common types without facets', () => {
    expect(mimeTypesFor(['pdf'], null)).toEqual(['application/pdf']);
  });

  test('matches nothing when a group has no types', () => {
    expect(mimeTypesFor(['other'], ['application/pdf'])).toEqual([NO_MIME_MATCH]);
    expect(mimeTypesFor([], ['application/pdf'])).toEqual([]);
  });

  test.each([
    ['application/pdf', 'PDF'],
    ['application/vnd.openxmlformats-officedocument.wordprocessingml.document', 'DOCX'],
    ['image/heic', 'HEIC'],
    ['application/x-custom-format', 'FORMAT'],
    [null, '—'],
  ])('short type for %s is %s', (mime, short) => {
    expect(shortType(mime)).toBe(short);
  });
});

describe('highlighting', () => {
  test('wraps the ranges in <mark>', () => {
    const { container } = render(<HighlightedText text="Invoice total due" ranges={[{ start: 0, end: 7 }, { start: 8, end: 13 }]} />);
    expect(Array.from(container.querySelectorAll('mark')).map((m) => m.textContent)).toEqual(['Invoice', 'total']);
    expect(container).toHaveTextContent('Invoice total due');
  });

  test('tidies overlapping and out-of-range ranges', () => {
    const { container } = render(<HighlightedText text="abcdef" ranges={[{ start: 2, end: 4 }, { start: 1, end: 3 }, { start: 5, end: 99 }, { start: 4, end: 4 }]} />);
    expect(Array.from(container.querySelectorAll('mark')).map((m) => m.textContent)).toEqual(['bc', 'd', 'f']);
    expect(container).toHaveTextContent('abcdef');
  });

  test('renders plain text without ranges', () => {
    render(<HighlightedText text="plain" ranges={[]} />);
    expect(screen.getByText('plain')).toBeInTheDocument();
  });

  test('maps backend UTF-8 byte offsets onto the text (German)', () => {
    // "Größe der " is 10 characters but 12 bytes: ö and ß take two bytes each.
    const text = 'Größe der Rechnung';
    expect(byteRangesToUtf16(text, [{ start: 12, end: 20 }])).toEqual([{ start: 10, end: 18 }]);
    const { container } = render(
      <SnippetLine snippet={{ text, start_offset: 0, end_offset: 20, highlight_ranges: [{ start: 12, end: 20 }] }} />,
    );
    expect(container.querySelector('mark')).toHaveTextContent(/^Rechnung$/);
  });

  test('maps byte offsets past emoji and accented letters', () => {
    // 📄 is 4 bytes / 2 UTF-16 units; "é" is 2 bytes / 1 unit.
    const text = '📄 café total';
    const start = new TextEncoder().encode('📄 café ').length;
    const { container } = render(
      <SnippetLine snippet={{ text, start_offset: 0, end_offset: 0, highlight_ranges: [{ start, end: start + 5 }] }} />,
    );
    expect(container.querySelector('mark')).toHaveTextContent(/^total$/);
    expect(byteRangesToUtf16(text, [{ start: 5, end: 10 }])).toEqual([{ start: 3, end: 7 }]);
  });

  test('snaps offsets inside a character outwards and clamps out-of-range ones', () => {
    // Byte 2 is inside "ö" (bytes 1-2): the start snaps back to it, the end forward past it.
    expect(byteRangesToUtf16('Göt', [{ start: 2, end: 2 }])).toEqual([{ start: 1, end: 2 }]);
    expect(byteRangesToUtf16('abc', [{ start: -3, end: 99 }])).toEqual([{ start: 0, end: 3 }]);
    expect(byteRangesToUtf16('ascii only', [{ start: 6, end: 10 }])).toEqual([{ start: 6, end: 10 }]);
  });

  test('picks the snippet with the most matches', () => {
    const a = { text: 'a', start_offset: 0, end_offset: 1, highlight_ranges: [{ start: 0, end: 1 }] };
    const b = { text: 'b', start_offset: 0, end_offset: 1, highlight_ranges: [{ start: 0, end: 1 }, { start: 0, end: 1 }] };
    expect(bestSnippet([a, b])).toBe(b);
    expect(bestSnippet([])).toBeNull();
    expect(bestSnippet(undefined)).toBeNull();
  });

  test('finds query words in text, ignoring case and operators', () => {
    expect(matchRanges('Total due. TOTAL paid.', 'total & !draft')).toEqual([
      { start: 0, end: 5 },
      { start: 11, end: 16 },
    ]);
    expect(matchRanges('anything', '')).toEqual([]);
    expect(matchRanges('and or not', 'and or not')).toEqual([]);
  });
});

describe('formatting', () => {
  test('sizes', () => {
    expect(formatBytes(512, 'en')).toBe('512 B');
    expect(formatBytes(2048, 'en')).toBe('2.0 KB');
    expect(formatBytes(5 * 1024 * 1024, 'en')).toBe('5.0 MB');
    expect(formatBytes(null)).toBe('—');
  });

  test('relative dates', () => {
    const now = Date.parse('2026-09-29T12:00:00Z');
    expect(formatRelative('2026-09-26T12:00:00Z', 'en', now)).toBe('3 days ago');
    expect(formatRelative('2026-09-29T11:59:50Z', 'en', now)).toBe('now');
    expect(formatRelative(null, 'en', now)).toBe('—');
  });
});
