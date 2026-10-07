import { describe, expect, it, vi } from 'vitest';

const m = vi.hoisted(() => ({ api: { get: vi.fn() }, sourceService: { getArrivals: vi.fn(), list: vi.fn() } }));
vi.mock('../../../services/api', () => ({ default: m.api, api: m.api, documentService: {}, sourceService: m.sourceService }));

import { fetchArrivals, groupLanes, isQuiet, LANE_CAP, laneHealth, laneHref, median, rankLanes, weekTotal, windowTotal, type SourceArrivals } from '../arrivals';
import { documentLane } from '../sourceTint';
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
    expect(laneHealth(asLane(lane('watch', [0])))).toBe('idle');
    // A source that never received anything is idle, unless it reports a problem itself.
    expect(laneHealth(asLane(lane('s', [0], { last_arrival_at: null })))).toBe('idle');
    expect(laneHealth(asLane(lane('s', [0], { last_arrival_at: null, status: 'warning' })))).toBe('warning');
    expect(laneHealth(asLane(lane('s', [0], { last_arrival_at: null, status: 'error' })))).toBe('error');
  });
});

describe('laneHref', () => {
  it('leads uploads to Intake and every other lane to its Sources tab', () => {
    expect(laneHref({ kind: 'upload', source_id: null })).toBe('/intake?section=upload');
    expect(laneHref({ kind: 'watch', source_id: null })).toBe('/sources?section=watch');
    expect(laneHref({ kind: 'webdav', source_id: 'a b' })).toBe('/sources?section=connections&source=a%20b');
    expect(laneHref({ kind: 's3', source_id: null })).toBe('/sources?section=connections&source=');
  });
});

describe('fetchArrivals', () => {
  it('asks for the given number of days and returns the lanes', async () => {
    m.sourceService.getArrivals.mockResolvedValueOnce({ data: [{ key: 'upload' }] });
    m.sourceService.list.mockResolvedValueOnce({ data: {} });
    await expect(fetchArrivals(7)).resolves.toEqual([{ key: 'upload' }]);
    expect(m.sourceService.getArrivals).toHaveBeenCalledWith(7);
  });

  it('folds each source\'s last health check into its status', async () => {
    m.sourceService.getArrivals.mockResolvedValueOnce({
      data: [
        lane('w', [1]),
        lane('c', [1]),
        lane('busy', [1], { status: 'syncing' }),
        lane('ok', [1]),
        lane('upload', [1]),
      ],
    });
    m.sourceService.list.mockResolvedValueOnce({
      data: [
        { id: 'w', validation_status: 'warning' },
        { id: 'c', validation_status: 'critical' },
        { id: 'busy', validation_status: 'warning' },
        { id: 'ok', validation_status: 'healthy' },
      ],
    });
    const out = await fetchArrivals();
    expect(out.map((l) => l.status)).toEqual(['warning', 'error', 'syncing', 'idle', null]);
  });

  it('keeps the lanes when the sources list fails', async () => {
    m.sourceService.getArrivals.mockResolvedValueOnce({ data: [lane('w', [1])] });
    m.sourceService.list.mockRejectedValueOnce(new Error('no'));
    await expect(fetchArrivals()).resolves.toHaveLength(1);
  });

  it('returns no lanes for an unexpected body', async () => {
    m.sourceService.getArrivals.mockResolvedValueOnce({ data: { nope: true } });
    await expect(fetchArrivals()).resolves.toEqual([]);
  });
});

describe('documentLane', () => {
  it('finds the lane a document came from', () => {
    expect(documentLane({ source_id: 's1', source_type: 'webdav' })).toEqual({ key: 's1', kind: 'source' });
    expect(documentLane({ source_type: 'watch_folder' })).toEqual({ key: 'watch', kind: 'watch' });
    expect(documentLane({ source_type: 'watch' })).toEqual({ key: 'watch', kind: 'watch' });
    expect(documentLane({ source_type: 'web_upload' })).toEqual({ key: 'upload', kind: 'upload' });
    expect(documentLane({})).toEqual({ key: 'upload', kind: 'upload' });
  });
});

describe('ranking and the cap', () => {
  const busy = [1, 2, 1, 3, 1, 2, 1, 1, 2, 1, 1, 2, 1];
  const keys = (ls: SourceArrivals[]) => ls.map((l) => l.key);

  it('orders by arrivals in the window, then by the latest arrival', () => {
    const lanes = [
      lane('small', [1, 0], { last_arrival_at: hoursAgo(10) }),
      lane('upload', [5, 5]),
      lane('older', [0, 1], { last_arrival_at: hoursAgo(5) }),
      lane('newer', [0, 1], { last_arrival_at: hoursAgo(1) }),
      lane('none', [0, 0], { last_arrival_at: null }),
    ].map(asLane);
    expect(keys(rankLanes(lanes))).toEqual(['upload', 'newer', 'older', 'small', 'none']);
    expect(windowTotal(lanes[1])).toBe(10);
  });

  it('guarantees problem lanes a slot, replacing the least active, but keeps activity order', () => {
    const lanes = [
      ...Array.from({ length: 6 }, (_, i) => lane(`a${i}`, [0, i + 1])),
      lane('broken', [0, 0], { status: 'error', last_arrival_at: null }),
      lane('quiet', [...busy, 0], { last_arrival_at: hoursAgo(40) }),
    ].map(asLane);
    // quiet has 18 arrivals in the window: it is the most active anyway.
    expect(keys(groupLanes(lanes, NOW).shown)).toEqual(['quiet', 'a5', 'a4', 'a3', 'broken']);
  });

  it('fills every slot with problems when there are more problems than slots', () => {
    const lanes = [
      lane('busy', [9, 9]),
      ...Array.from({ length: 6 }, (_, i) => lane(`e${i}`, [0, 0], { status: 'error', last_arrival_at: hoursAgo(i + 1) })),
    ].map(asLane);
    expect(keys(groupLanes(lanes, NOW).shown)).toEqual(['e0', 'e1', 'e2', 'e3', 'e4']);
  });

  it(`shows every lane when there are ${LANE_CAP} or fewer`, () => {
    const lanes = [lane('upload', [0], { last_arrival_at: null }), lane('watch', [0], { last_arrival_at: null })].map(asLane);
    expect(groupLanes(lanes, NOW)).toEqual({ shown: lanes, hidden: 0 });
  });

  it(`caps at ${LANE_CAP}, keeps problem lanes, and never gives a slot to a silent lane`, () => {
    const silent = Array.from({ length: 200 }, (_, i) => asLane(lane(`s${i}`, [0, 0], { last_arrival_at: null })));
    const active = Array.from({ length: 6 }, (_, i) => asLane(lane(`a${i}`, [0, i + 1])));
    const broken = asLane(lane('broken', [0, 0], { status: 'error', last_arrival_at: null }));
    const { shown, hidden } = groupLanes([...silent, ...active, broken], NOW);
    expect(keys(shown)).toEqual(['a5', 'a4', 'a3', 'a2', 'broken']);
    expect(hidden).toBe(202);
  });

  it('shows no lane when none of many has arrivals or a problem', () => {
    const silent = Array.from({ length: 8 }, (_, i) => asLane(lane(`s${i}`, [0], { last_arrival_at: null })));
    expect(groupLanes(silent, NOW)).toEqual({ shown: [], hidden: 8 });
  });
});
