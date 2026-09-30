import { describe, expect, it } from 'vitest';
import { typeCodeOf } from '../../../lib/fileType';
import { formatStamp, sourceLabel } from '../format';

const t = (_key: string, fallback: string) => fallback;

describe('typeCodeOf', () => {
  it.each([
    ['application/vnd.openxmlformats-officedocument.wordprocessingml.document', 'a.docx', 'DOCX'],
    ['application/msword', 'a.doc', 'DOC'],
    ['application/pdf', 'scan', 'PDF'],
    ['image/jpeg', 'photo.jpg', 'JPEG'],
    ['application/pdf; charset=binary', 'x', 'PDF'],
    ['application/octet-stream', 'notes.odt', 'ODT'],
    ['', 'IMG_01.PNG', 'PNG'],
    [null, null, '—'],
    ['application/octet-stream', 'no-extension', '—'],
  ])('%s + %s -> %s', (mime, name, code) => {
    expect(typeCodeOf(mime, name)).toBe(code);
  });
});

describe('sourceLabel', () => {
  it.each([
    [null, 'Upload'],
    ['web_upload', 'Upload'],
    ['direct_upload', 'Upload'],
    ['webdav', 'WebDAV'],
    ['web_dav', 'WebDAV'],
    ['s3', 'S3'],
    ['local_folder', 'Local folder'],
    ['batch_ingest', 'Batch import'],
    ['some_new_kind', 'Some new kind'],
  ])('%s -> %s', (type, label) => {
    expect(sourceLabel(type, t)).toBe(label);
  });
});

describe('formatStamp', () => {
  it('formats a local date and time as YYYY-MM-DD HH:mm', () => {
    const d = new Date(2026, 8, 29, 11, 4, 59);
    expect(formatStamp(d.toISOString())).toBe('2026-09-29 11:04');
  });

  it('shows a dash for missing or invalid values', () => {
    expect(formatStamp(null)).toBe('—');
    expect(formatStamp('not a date')).toBe('—');
  });
});
