import { describe, expect, it } from 'vitest';
import { ocrRetryOutcome } from '../ocrRetry';

describe('ocrRetryOutcome', () => {
  it('is queued when the server queued a new job', () => {
    expect(
      ocrRetryOutcome({
        success: true,
        queued: true,
        message: 'Document queued for OCR processing',
      }),
    ).toBe('queued');
  });

  it('is alreadyQueued when the request succeeded but a job was already pending', () => {
    expect(
      ocrRetryOutcome({
        success: true,
        queued: false,
        message: 'The document is already queued for OCR',
      }),
    ).toBe('alreadyQueued');
  });

  it('is refused when the server says the request failed', () => {
    expect(ocrRetryOutcome({ success: false, queued: false, message: 'nope' })).toBe('refused');
    expect(ocrRetryOutcome({ success: false })).toBe('refused');
  });

  it('treats a body without queued as queued', () => {
    expect(ocrRetryOutcome({})).toBe('queued');
    expect(ocrRetryOutcome(undefined)).toBe('queued');
    expect(ocrRetryOutcome(null)).toBe('queued');
  });
});
