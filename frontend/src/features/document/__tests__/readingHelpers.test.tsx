import type { TFunction } from 'i18next';
import { describe, expect, it } from 'vitest';
import { formatKeyName } from '../details/MetadataDisplay';
import { queryTerms, splitMatches, toParagraphs } from '../reading/highlight';
import { formatAdded, metaParts, pageCount } from '../meta';
import { makeDocument, makeOcr } from './testUtils';

const t = ((key: string, opts?: string | { defaultValue?: string; [k: string]: unknown }) => {
  const options = typeof opts === 'string' ? { defaultValue: opts } : (opts ?? {});
  return String(options.defaultValue ?? key).replace(/\{\{(\w+)\}\}/g, (_, name) => String(options[name] ?? ''));
}) as unknown as TFunction;

describe('queryTerms', () => {
  it('keeps quoted phrases whole, drops repeats and puts the longest first', () => {
    expect(queryTerms('in "left shoulder" injury IN')).toEqual(['left shoulder', 'injury', 'in']);
  });

  it('is empty for a blank query', () => {
    expect(queryTerms('   ')).toEqual([]);
  });
});

describe('splitMatches', () => {
  it('prefers the whole phrase when it occurs', () => {
    const segments = splitMatches('the shoulder injury and the injury', 'shoulder injury');
    expect(segments.filter((s) => s.match >= 0).map((s) => s.text)).toEqual(['shoulder injury']);
  });

  it('falls back to each word when the phrase does not occur', () => {
    const segments = splitMatches('injury to the shoulder', 'shoulder injury');
    expect(segments.filter((s) => s.match >= 0).map((s) => [s.text, s.match])).toEqual([
      ['injury', 0],
      ['shoulder', 1],
    ]);
  });

  it('finds a quoted phrase literally', () => {
    const segments = splitMatches('a (net) b', '"(net)"');
    expect(segments.filter((s) => s.match >= 0).map((s) => s.text)).toEqual(['(net)']);
  });

  it('finds nothing for a single missing word', () => {
    expect(splitMatches('abc', 'zzz')).toEqual([{ text: 'abc', match: -1 }]);
  });
});

describe('toParagraphs', () => {
  it('splits on any run of blank lines, trims each paragraph and drops empty ones', () => {
    const paragraphs = toParagraphs(splitMatches('\n\nOne\ntwo\n\n\n\n  \nThree  \n\n', ''));
    expect(paragraphs.map((p) => p.map((s) => s.text).join(''))).toEqual(['One\ntwo', 'Three']);
  });

  it('keeps matches, with their numbers, inside the right paragraph', () => {
    const paragraphs = toParagraphs(splitMatches('tax here\n\n\nmore tax', 'tax'));
    expect(paragraphs).toEqual([
      [
        { text: 'tax', match: 0 },
        { text: ' here', match: -1 },
      ],
      [
        { text: 'more ', match: -1 },
        { text: 'tax', match: 1 },
      ],
    ]);
  });
});

describe('meta', () => {
  it('formats the added date or gives null', () => {
    expect(formatAdded('2026-09-30T08:04:00Z', 'en-GB')).toMatch(/30 Sept? 2026/);
    expect(formatAdded(null)).toBeNull();
    expect(formatAdded('not a date')).toBeNull();
  });

  it('reads the page count from OCR, then OCR progress, never from the file metadata', () => {
    expect(pageCount(makeDocument({ source_metadata: { page_count: 4 } }), makeOcr())).toBeNull();
    expect(pageCount(makeDocument(), { ...makeOcr(), pages_processed: 3 } as ReturnType<typeof makeOcr>)).toBe(3);
    expect(pageCount(makeDocument({ ocr_progress_total: 7 }), null)).toBe(7);
    expect(pageCount(makeDocument(), null)).toBeNull();
  });

  it('only shows OCR confidence once OCR has finished', () => {
    const parts = metaParts({ document: makeDocument({ ocr_status: 'processing' }), ocr: null, t });
    expect(parts.some((p) => p.startsWith('OCR '))).toBe(false);
  });

  it('prefers the connection name over the source type', () => {
    const parts = metaParts({ document: makeDocument({ source_type: 'webdav' }), ocr: null, sourceName: 'NAS', t });
    expect(parts).toContain('NAS');
    expect(parts).not.toContain('WebDAV');
  });
});

describe('formatKeyName', () => {
  it('writes keys in sentence case and keeps known acronyms upper case', () => {
    expect(formatKeyName('pdf_creation_date')).toBe('PDF creation date');
    expect(formatKeyName('cameraModel')).toBe('Camera model');
    expect(formatKeyName('source-id')).toBe('Source ID');
  });
});
