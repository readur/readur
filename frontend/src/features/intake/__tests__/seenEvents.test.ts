import { beforeEach, describe, expect, it } from 'vitest';
import { acknowledgeAll, isShownLit, LIT_SHOWN_CAP } from '../../board/litStore';
import { DOCUMENT_EVENTS_KEY, flagNewFailures } from '../shared/seenEvents';

const T = Date.parse('2026-09-30T12:00:00Z');
const iso = (minutesAgo: number) => new Date(T - minutesAgo * 60_000).toISOString();
const page = (from: number) =>
  Array.from({ length: LIT_SHOWN_CAP }, (_, i) => ({ id: `f${from + i}`, eventKey: `f${from + i}`, at: iso(from + i) }));

beforeEach(() => {
  window.localStorage.clear();
  acknowledgeAll();
});

describe('flagNewFailures', () => {
  it('flags each event once and ranks it by when it happened', () => {
    // A newest-first list, viewed page 1 then page 2.
    expect(flagNewFailures(DOCUMENT_EVENTS_KEY, 'attention', page(0))).toBe(LIT_SHOWN_CAP);
    expect(flagNewFailures(DOCUMENT_EVENTS_KEY, 'attention', page(LIT_SHOWN_CAP))).toBe(LIT_SHOWN_CAP);
    expect(flagNewFailures(DOCUMENT_EVENTS_KEY, 'attention', page(0))).toBe(0);
    expect(isShownLit('attention', 'f0')).toBe(true);
    expect(isShownLit('attention', `f${LIT_SHOWN_CAP - 1}`)).toBe(true);
    expect(isShownLit('attention', `f${LIT_SHOWN_CAP}`)).toBe(false);
  });

  it('falls back to marking time when the event time is missing or unreadable', () => {
    flagNewFailures(DOCUMENT_EVENTS_KEY, 'attention', [
      { id: 'x', eventKey: 'x', at: 'not a date' },
      { id: 'y', eventKey: 'y', at: null },
    ]);
    expect(isShownLit('attention', 'x')).toBe(true);
    expect(isShownLit('attention', 'y')).toBe(true);
  });
});
