import { describe, expect, it } from 'vitest';
import { humanizeAdvice, humanizeConnectionFailure } from '../connections/connectionFailure';
import { kindOfSourceType } from '../shared/sourceTypes';

describe('humanizeConnectionFailure', () => {
  it.each([
    ['WebDAV error: 401 Unauthorized', /refused the sign-in/],
    ['Access denied', /refused the sign-in/],
    ['NoSuchBucket: the bucket does not exist', /wasn't found/],
    ['error sending request: Connection refused (os error 111)', /Can't reach the server/],
    ['invalid peer certificate: UnknownIssuer', /certificate/],
    ['High empty sync ratio: 100.0% of recent syncs found no files', /found no new files/],
    ['HTTP 503 Service Unavailable', /server reported an error/],
  ])('reads %s', (raw, summary) => {
    const human = humanizeConnectionFailure(raw);
    expect(human.summary).toMatch(summary);
    expect(human.detail).toBe(raw);
  });

  it('falls back to the document patterns, then the first line', () => {
    expect(humanizeConnectionFailure("OCR failed for '/app/x.pdf'").summary).toBe('OCR failed');
    expect(humanizeConnectionFailure('Something odd\nsecond line').summary).toBe('Something odd');
  });
});

describe('kindOfSourceType', () => {
  it('maps connection types to the colour kinds', () => {
    expect(kindOfSourceType('webdav')).toBe('webdav');
    expect(kindOfSourceType('s3')).toBe('s3');
    expect(kindOfSourceType('local_folder')).toBe('local');
    expect(kindOfSourceType('other')).toBeUndefined();
    expect(kindOfSourceType(null)).toBeUndefined();
  });
});

describe('humanizeAdvice', () => {
  it('rewrites known advice and keeps unknown advice as written', () => {
    expect(humanizeAdvice('Check server URL, credentials, and network connectivity')).toMatch(/server address/);
    expect(humanizeAdvice('This may indicate connectivity issues or that the source has no new content')).toMatch(/folder path/);
    expect(humanizeAdvice('Try turning it off')).toBe('Try turning it off');
  });
});
