import { describe, test, expect, vi } from 'vitest';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { I18nextProvider } from 'react-i18next';
import i18n from 'i18next';
import '../../../test/test-utils';
import { Timeline } from '../search/Timeline';
import { fillMonths, groupByMonth, monthKey, monthLabel, monthRange, rangeMonths, MAX_MONTHS } from '../months';
import { hueKind, sourceStyle } from '../SourceBadge';
import { LABELS_CHANGED_EVENT, notifyLabelsChanged } from '../../labels';

const BARS = fillMonths([
  { month: '2024-03', count: 5 },
  { month: '2024-01', count: 2 },
  { month: '2024-04', count: 1 },
]);

function renderTimeline(props: Partial<Parameters<typeof Timeline>[0]> = {}) {
  const onPick = vi.fn();
  render(
    <I18nextProvider i18n={i18n}>
      <Timeline bars={BARS} from={null} to={null} onPick={onPick} {...props} />
    </I18nextProvider>,
  );
  return onPick;
}

const bar = (name: string) => screen.getByRole('button', { name });

describe('Timeline', () => {
  test('one button per month with matches; empty months are gaps, not buttons', () => {
    renderTimeline();
    const chart = screen.getByRole('group', { name: 'Matches by month' });
    expect(within(chart).getAllByRole('button')).toHaveLength(3);
    expect(bar('January 2024: 2 matches')).toBeInTheDocument();
    expect(bar('April 2024: 1 match')).toBeInTheDocument();
  });

  test('draws nothing without data', () => {
    renderTimeline({ bars: [] });
    expect(screen.queryByRole('group', { name: 'Matches by month' })).not.toBeInTheDocument();
  });

  test('pressing a month picks it; pressing the lone pick again clears it', async () => {
    const user = userEvent.setup();
    const onPick = renderTimeline();
    await user.click(bar('March 2024: 5 matches'));
    expect(onPick).toHaveBeenLastCalledWith({ start: '2024-03', end: '2024-03' });
  });

  test('a picked month is pressed and clears on a second press', async () => {
    const user = userEvent.setup();
    const onPick = renderTimeline({ from: '2024-03-01', to: '2024-03-31' });
    expect(bar('March 2024: 5 matches')).toHaveAttribute('aria-pressed', 'true');
    expect(bar('January 2024: 2 matches')).toHaveAttribute('aria-pressed', 'false');
    await user.click(bar('March 2024: 5 matches'));
    expect(onPick).toHaveBeenLastCalledWith(null);
  });

  test('Shift+press stretches the pick into a span', async () => {
    const user = userEvent.setup();
    const onPick = renderTimeline();
    await user.click(bar('April 2024: 1 match'));
    await user.keyboard('{Shift>}');
    await user.click(bar('January 2024: 2 matches'));
    await user.keyboard('{/Shift}');
    expect(onPick).toHaveBeenLastCalledWith({ start: '2024-01', end: '2024-04' });
  });

  test('is one tab stop; arrow keys, Home and End move between months', async () => {
    const user = userEvent.setup();
    const onPick = renderTimeline();
    await user.tab();
    expect(bar('April 2024: 1 match')).toHaveFocus();
    await user.keyboard('{ArrowLeft}');
    expect(bar('March 2024: 5 matches')).toHaveFocus();
    await user.keyboard('{Home}');
    expect(bar('January 2024: 2 matches')).toHaveFocus();
    await user.keyboard('{End}');
    expect(bar('April 2024: 1 match')).toHaveFocus();
    await user.keyboard('{ArrowRight}');
    expect(bar('April 2024: 1 match')).toHaveFocus();
    await user.keyboard('{Enter}');
    expect(onPick).toHaveBeenLastCalledWith({ start: '2024-04', end: '2024-04' });
  });

  test('hovering a bar shows its month and count', async () => {
    const user = userEvent.setup();
    renderTimeline();
    expect(screen.getByText('Jan 2024 – Apr 2024')).toBeInTheDocument();
    await user.hover(bar('March 2024: 5 matches'));
    expect(screen.getByText('March 2024: 5 matches', { selector: 'p' })).toBeInTheDocument();
  });

  test('long spans label only the years', () => {
    const long = fillMonths([
      { month: '2020-05', count: 1 },
      { month: '2023-02', count: 1 },
    ]);
    renderTimeline({ bars: long });
    expect(screen.getByText('2021')).toBeInTheDocument();
    expect(screen.queryByText('Jun')).not.toBeInTheDocument();
  });
});

describe('months', () => {
  test('monthKey reads local months and rejects junk', () => {
    expect(monthKey('2024-03-15T12:00:00')).toBe('2024-03');
    expect(monthKey('nope')).toBe('');
    expect(monthKey(null)).toBe('');
  });

  test('groupByMonth keeps runs in order', () => {
    const rows = [{ d: '2024-03-02T12:00:00' }, { d: '2024-03-01T12:00:00' }, { d: '2024-01-09T12:00:00' }];
    expect(groupByMonth(rows, (r) => r.d).map((g) => [g.key, g.items.length])).toEqual([
      ['2024-03', 2],
      ['2024-01', 1],
    ]);
  });

  test('monthRange gives the first and last day, leap years included', () => {
    expect(monthRange('2024-02')).toEqual({ from: '2024-02-01', to: '2024-02-29' });
    expect(monthRange('2023-12')).toEqual({ from: '2023-12-01', to: '2023-12-31' });
    expect(monthRange('2023-13')).toBeNull();
  });

  test('monthLabel names the month in the language asked for', () => {
    expect(monthLabel('2024-03', 'en')).toBe('March 2024');
    expect(monthLabel('2024-03', 'en', 'short')).toBe('Mar 2024');
    expect(monthLabel('junk', 'en')).toBe('junk');
  });

  test('fillMonths sorts, sums duplicates, fills gaps across years and drops bad rows', () => {
    const bars = fillMonths([
      { month: '2024-01', count: 1 },
      { month: '2023-11', count: 2 },
      { month: '2023-11', count: 1 },
      { month: 'bad', count: 4 },
      { month: '2023-10', count: 0 },
    ]);
    expect(bars).toEqual([
      { month: '2023-11', count: 3 },
      { month: '2023-12', count: 0 },
      { month: '2024-01', count: 1 },
    ]);
    expect(fillMonths([])).toEqual([]);
  });

  test('fillMonths keeps the newest months of a very long span', () => {
    const bars = fillMonths([
      { month: '1990-01', count: 1 },
      { month: '2026-01', count: 1 },
    ]);
    expect(bars).toHaveLength(MAX_MONTHS);
    expect(bars.at(-1)).toEqual({ month: '2026-01', count: 1 });
  });

  test('rangeMonths covers open-ended ranges', () => {
    expect(rangeMonths(null, null)).toBeNull();
    expect(rangeMonths('2024-03-01', null)).toEqual({ start: '2024-03', end: '9999-99' });
    expect(rangeMonths(null, '2024-03-31')).toEqual({ start: '0000-00', end: '2024-03' });
  });
});

describe('source colours', () => {
  test('uploads, the watch folder and connections get their slot family', () => {
    expect(hueKind({ source_id: null, source_type: 'direct_upload' })).toBe('upload');
    expect(hueKind({ source_id: null, source_type: null })).toBe('upload');
    expect(hueKind({ source_id: null, source_type: 'watch_folder' })).toBe('watch');
    expect(hueKind({ source_id: 's1', source_type: 'webdav' })).toBe('webdav');
  });

  test('styles point at the source hue tokens', () => {
    const style = sourceStyle({ source_id: null, source_type: 'watch_folder' }) as Record<string, string>;
    expect(style['--src']).toBe('var(--src-2)');
    expect(style['--src-soft']).toBe('var(--src-2-soft)');
  });
});

describe('label change event', () => {
  test('notifyLabelsChanged dispatches readur:labels-changed on window', () => {
    const listener = vi.fn();
    window.addEventListener(LABELS_CHANGED_EVENT, listener);
    notifyLabelsChanged();
    window.removeEventListener(LABELS_CHANGED_EVENT, listener);
    expect(LABELS_CHANGED_EVENT).toBe('readur:labels-changed');
    expect(listener).toHaveBeenCalledTimes(1);
  });
});
