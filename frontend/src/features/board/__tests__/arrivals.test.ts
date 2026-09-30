import { describe, expect, it, vi } from 'vitest';

const m = vi.hoisted(() => ({ api: { get: vi.fn() } }));
vi.mock('../../../services/api', () => ({ default: m.api, api: m.api, documentService: {} }));

import { fetchArrivals, isQuiet, laneHealth, laneHref, median, weekTotal, type SourceArrivals } from '../arrivals';
import { documentLane, sourceHue, tintKind } from '../sourceTint';
import { lane } from './homeTestUtils';

const NOW = Date.now();
const hoursAgo = (h: number) => new Date(NOW - h * 3600_000).toISOString();
const asLane = (l: ReturnType<typeof lane>) => l as unknown as SourceArrivals;

describe('median', () => {
  it('handles empty, odd and even lists', () => {
    expect(median([])).toBe(0);
    expect(median([3, 1, 2])).toBe(2);
    expect(median([4, 1, 0, 2])).toBe(1.5);
  });
});

describe('isQuiet', () => {
  const busy = [1, 2, 1, 3, 1, 2, 1, 1, 2, 1, 1, 2, 1];

  it('is quiet when a usually busy lane got nothing today and nothing for over a day', () => {
    expect(isQuiet(asLane(lane('s', [...busy, 0], { last_arrival_at: hoursAgo(30) })), NOW)).toBe(true);
    expect(isQuiet(asLane(lane('s', [...busy, 0], { last_arrival_at: null })), NOW)).toBe(true);
  });

  it('is not quiet when any of the three conditions fails', () => {
    expect(isQuiet(asLane(lane('s', [...busy, 2], { last_arrival_at: hoursAgo(30) })), NOW)).toBe(false);
    expect(isQuiet(asLane(lane('s', [...busy, 0], { last_arrival_at: hoursAgo(20) })), NOW)).toBe(false);
    const sparse = [0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0];
    expect(isQuiet(asLane(lane('s', [...sparse, 0], { last_arrival_at: hoursAgo(90) })), NOW)).toBe(false);
  });

  it('never flags a disabled lane', () => {
    expect(isQuiet(asLane(lane('s', [...busy, 0], { enabled: false, last_arrival_at: hoursAgo(30) })), NOW)).toBe(false);
  });
});

describe('weekTotal', () => {
  it('sums the last seven days of every lane', () => {
    expect(weekTotal([asLane(lane('a', [9, 1, 1, 1, 1, 1, 1, 1])), asLane(lane('b', [2]))])).toBe(9);
  });
});

describe('laneHealth', () => {
  it('orders off, error, quiet, syncing and healthy', () => {
    expect(laneHealth(asLane(lane('s', [1], { enabled: false, status: 'error' })))).toBe('off');
    expect(laneHealth(asLane(lane('s', [1], { status: 'error' })))).toBe('error');
    expect(laneHealth(asLane(lane('s', [1, 1, 1, 0], { status: 'syncing', last_arrival_at: hoursAgo(48) })), NOW)).toBe('quiet');
    expect(laneHealth(asLane(lane('s', [1], { status: 'syncing' })))).toBe('syncing');
    expect(laneHealth(asLane(lane('upload', [1])))).toBe('healthy');
  });
});

describe('laneHref', () => {
  it('leads each lane to its Intake section', () => {
    expect(laneHref({ kind: 'upload', source_id: null })).toBe('/intake?section=upload');
    expect(laneHref({ kind: 'watch', source_id: null })).toBe('/intake?section=watch');
    expect(laneHref({ kind: 'webdav', source_id: 'a b' })).toBe('/intake?section=connections&source=a%20b');
    expect(laneHref({ kind: 's3', source_id: null })).toBe('/intake?section=connections&source=');
  });
});

describe('fetchArrivals', () => {
  it('asks for the given number of days and returns the lanes', async () => {
    m.api.get.mockResolvedValueOnce({ data: [{ key: 'upload' }] });
    await expect(fetchArrivals(7)).resolves.toEqual([{ key: 'upload' }]);
    expect(m.api.get).toHaveBeenCalledWith('/sources/arrivals', { params: { days: 7 } });
  });

  it('returns no lanes for an unexpected body', async () => {
    m.api.get.mockResolvedValueOnce({ data: { nope: true } });
    await expect(fetchArrivals()).resolves.toEqual([]);
  });
});

describe('source colours', () => {
  it('gives uploads and the watch folder fixed slots, and sources a stable slot from 3 to 8', () => {
    expect(sourceHue(null, 'upload')).toEqual({ varName: '--src-1', softVarName: '--src-1-soft', index: 1 });
    expect(sourceHue(null, 'watch').index).toBe(2);
    expect(sourceHue(null).index).toBe(1);
    const a = sourceHue('0b5c-source', 'webdav').index;
    expect(a).toBeGreaterThanOrEqual(3);
    expect(a).toBeLessThanOrEqual(8);
    expect(sourceHue('0b5c-source', 's3').index).toBe(a);
  });

  it('maps server kinds', () => {
    expect(tintKind('local_folder')).toBe('local');
    expect(tintKind('webdav')).toBe('webdav');
    expect(tintKind('nope')).toBeUndefined();
    expect(tintKind(null)).toBeUndefined();
  });

  it('finds the lane a document came from', () => {
    expect(documentLane({ source_id: 's1', source_type: 'webdav' })).toEqual({ key: 's1', kind: 'webdav' });
    expect(documentLane({ source_id: 's1', source_type: 'filesystem' })).toEqual({ key: 's1', kind: 'local' });
    expect(documentLane({ source_type: 'watch_folder' })).toEqual({ key: 'watch', kind: 'watch' });
    expect(documentLane({ source_type: 'web_upload' })).toEqual({ key: 'upload', kind: 'upload' });
    expect(documentLane({})).toEqual({ key: 'upload', kind: 'upload' });
  });
});
