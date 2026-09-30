import { describe, expect, it } from 'vitest';
import { humanizeConnectionFailure } from '../connections/connectionFailure';
import { kindOfSourceType } from '../shared/sourceTypes';

describe('humanizeConnectionFailure', () => {
  it.each([
    ['WebDAV error: 401 Unauthorized', /refused the sign-in/],
    ['Access denied', /refused the sign-in/],
    ['NoSuchBucket: the bucket does not exist', /wasn't found/],
    ['error sending request: Connection refused (os error 111)', /Can't reach the server/],
    ['invalid peer certificate: UnknownIssuer', /certificate/],
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
