/**
 * Arrivals per ingestion lane (every source, the watch folder and uploads), and the rule that
 * flags a lane as quiet.
 */
import { sourceService } from '../../services/api';
import type { SourceArrivals } from '../../types/generated';

export type { DayCount, SourceArrivals } from '../../types/generated';

export const ARRIVAL_DAYS = 14;
const DAY_MS = 24 * 3600 * 1000;

/** Source id → its last health check (`warning`, `critical`…), or an empty map if unavailable. */
async function validations(): Promise<Map<string, string>> {
  try {
    const res = await sourceService.list();
    const list = Array.isArray(res.data) ? res.data : [];
    return new Map(list.filter((s) => s.validation_status).map((s) => [s.id, s.validation_status as string]));
  } catch {
    return new Map();
  }
}

/**
 * The lanes, with each source's last health check folded into its status (the same rule as the
 * sidebar): a critical check reads as an error, a warning as a warning, unless the source is
 * already syncing or in error.
 */
export async function fetchArrivals(days = ARRIVAL_DAYS): Promise<SourceArrivals[]> {
  const [res, checks] = await Promise.all([sourceService.getArrivals(days), validations()]);
  const lanes = Array.isArray(res.data) ? res.data : [];
  return lanes.map((lane) => {
    const check = lane.source_id ? checks.get(lane.source_id) : undefined;
    if (!check || lane.status === 'error' || lane.status === 'syncing') return lane;
    if (check === 'critical') return { ...lane, status: 'error' };
    if (check === 'warning') return { ...lane, status: 'warning' };
    return lane;
  });
}

export function median(values: readonly number[]): number {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
}

/**
 * A lane is quiet when it usually receives documents (median of the days before today is at
 * least one), nothing arrived today, and the last arrival is more than a day old.
 */
export function isQuiet(lane: SourceArrivals, now: number = Date.now()): boolean {
  if (!lane.enabled) return false;
  const prior = lane.days.slice(0, -1).map((d) => d.count);
  if (median(prior) < 1 || lane.today > 0) return false;
  const last = lane.last_arrival_at ? Date.parse(lane.last_arrival_at) : NaN;
  return Number.isNaN(last) || now - last > DAY_MS;
}

/** Documents that arrived over the last seven days (today included), across all lanes. */
export function weekTotal(lanes: readonly SourceArrivals[]): number {
  return lanes.reduce((sum, lane) => sum + lane.days.slice(-7).reduce((s, d) => s + d.count, 0), 0);
}

export type LaneHealth = 'quiet' | 'error' | 'warning' | 'syncing' | 'off' | 'idle' | 'healthy';

export function laneHealth(lane: SourceArrivals, now?: number): LaneHealth {
  if (!lane.enabled) return 'off';
  if (lane.status === 'error') return 'error';
  if (lane.status === 'warning') return 'warning';
  if (isQuiet(lane, now)) return 'quiet';
  if (lane.status === 'syncing') return 'syncing';
  // Nothing has ever arrived: there is nothing to call healthy yet.
  if (!lane.last_arrival_at) return 'idle';
  return 'healthy';
}

const PROBLEMS: readonly LaneHealth[] = ['error', 'warning', 'quiet'];

/** A lane that needs a look: its source reports an error or warning, or it went quiet. */
export function isProblem(lane: SourceArrivals, now?: number): boolean {
  return PROBLEMS.includes(laneHealth(lane, now));
}

/** Documents that arrived in the lane's whole window. */
export function windowTotal(lane: SourceArrivals): number {
  return lane.days.reduce((sum, d) => sum + d.count, 0);
}

const lastAt = (lane: SourceArrivals) => (lane.last_arrival_at ? Date.parse(lane.last_arrival_at) || 0 : 0);

/**
 * Problems first, then the busiest lanes in the window, then the most recent arrival. Uploads
 * and the watch folder compete in the same order; ties keep the server's order.
 */
export function rankLanes(lanes: readonly SourceArrivals[], now?: number): SourceArrivals[] {
  return lanes
    .map((lane, index) => ({ lane, index, problem: isProblem(lane, now), total: windowTotal(lane), last: lastAt(lane) }))
    .sort(
      (a, b) =>
        Number(b.problem) - Number(a.problem) || b.total - a.total || b.last - a.last || a.index - b.index,
    )
    .map((r) => r.lane);
}

/** How many lanes Home shows. */
export const LANE_CAP = 5;

export interface LaneGroups {
  /** The most active lanes, shown on Home. */
  shown: SourceArrivals[];
  /** How many other lanes there are (listed in Intake, not on Home). */
  hidden: number;
}

/**
 * The lanes Home shows: the {@link LANE_CAP} most active, in {@link rankLanes} order, so a lane
 * with a problem always makes the cut ahead of a merely busy one. With more lanes than the cap,
 * a lane with no arrivals in the window and no problem never takes a slot, so hundreds of unused
 * sources never crowd out the ones that are working.
 */
export function groupLanes(lanes: readonly SourceArrivals[], now?: number, cap: number = LANE_CAP): LaneGroups {
  const ranked = rankLanes(lanes, now);
  const pool = ranked.length <= cap ? ranked : ranked.filter((l) => windowTotal(l) > 0 || isProblem(l, now));
  const shown = pool.slice(0, cap);
  return { shown, hidden: ranked.length - shown.length };
}

/** Where a lane's name leads: the matching Intake section. */
export function laneHref(lane: Pick<SourceArrivals, 'kind' | 'source_id'>): string {
  if (lane.kind === 'upload') return '/intake?section=upload';
  if (lane.kind === 'watch') return '/intake?section=watch';
  return `/intake?section=connections&source=${encodeURIComponent(lane.source_id ?? '')}`;
}
