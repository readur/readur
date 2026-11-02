import i18n from 'i18next';
import { afterEach, describe, expect, it } from 'vitest';
import { formatAbsoluteDate, formatRelativeTime } from '../relativeTime';

const NOW = Date.parse('2026-09-30T12:00:00Z');
const ago = (s: number) => NOW - s * 1000;
const en = { now: NOW, locale: 'en' };

describe('formatRelativeTime', () => {
  const original = i18n.language;
  afterEach(() => {
    void i18n.changeLanguage(original);
  });

  it('says "now" under a minute, past or future', () => {
    expect(formatRelativeTime(ago(20), en)).toBe('now');
    expect(formatRelativeTime(NOW + 20_000, en)).toBe('now');
  });

  it('uses short units in the past', () => {
    expect(formatRelativeTime(ago(21 * 60), en)).toBe('21 min. ago');
    expect(formatRelativeTime(ago(3 * 3600 + 50 * 60), en)).toBe('3 hr. ago');
    expect(formatRelativeTime(ago(2 * 86400), en)).toBe('2 days ago');
  });

  it('handles the future', () => {
    expect(formatRelativeTime(NOW + 42 * 60_000, en)).toBe('in 42 min.');
  });

  it('switches to the date beyond a week, unless told not to', () => {
    expect(formatRelativeTime(ago(8 * 86400), en)).toBe('Sep 22, 2026');
    expect(formatRelativeTime(ago(8 * 86400), { ...en, absoluteAfterDays: null })).toBe('1 wk. ago');
    expect(formatRelativeTime(ago(3 * 86400), { ...en, absoluteAfterDays: 2 })).toBe('Sep 27, 2026');
  });

  it('accepts ISO strings, numbers and Dates', () => {
    const iso = new Date(ago(300)).toISOString();
    expect(formatRelativeTime(iso, en)).toBe('5 min. ago');
    expect(formatRelativeTime(new Date(ago(300)), en)).toBe('5 min. ago');
  });

  it('returns the fallback for missing or bad input', () => {
    expect(formatRelativeTime(null, en)).toBe('—');
    expect(formatRelativeTime(undefined, en)).toBe('—');
    expect(formatRelativeTime('not a date', en)).toBe('—');
    expect(formatRelativeTime('', { ...en, fallback: '' })).toBe('');
  });

  it.each([
    ['de', 'vor 21 Min.'],
    ['es', 'hace 21 min'],
    ['fr', 'il y a 21 min'],
  ])('localizes to %s', (locale, text) => {
    expect(formatRelativeTime(ago(21 * 60), { now: NOW, locale }).replace(/\s/g, ' ')).toBe(text);
  });

  it('follows the current i18n language by default', async () => {
    await i18n.changeLanguage('de');
    expect(formatRelativeTime(ago(21 * 60), { now: NOW })).toBe('vor 21 Min.');
  });
});

describe('formatAbsoluteDate', () => {
  it('formats a medium date or the fallback', () => {
    expect(formatAbsoluteDate(NOW, 'en')).toBe('Sep 30, 2026');
    expect(formatAbsoluteDate(null, 'en')).toBe('—');
  });
});
