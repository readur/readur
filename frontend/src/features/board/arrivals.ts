/**
 * Arrivals per ingestion lane (every source, the watch folder and uploads), and the rule that
 * flags a lane as quiet.
 */
import { sourceService } from '../../services/api';
import type { SourceArrivals } from '../../types/generated';

export type { DayCount, SourceArrivals } from '../../types/generated';

export const ARRIVAL_DAYS = 14;
const DAY_MS = 24 * 3600 * 1000;

export async function fetchArrivals(days = ARRIVAL_DAYS): Promise<SourceArrivals[]> {
  const res = await sourceService.getArrivals(days);
  return Array.isArray(res.data) ? res.data : [];
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

export type LaneHealth = 'quiet' | 'error' | 'syncing' | 'off' | 'healthy';

export function laneHealth(lane: SourceArrivals, now?: number): LaneHealth {
  if (!lane.enabled) return 'off';
  if (lane.status === 'error') return 'error';
  if (isQuiet(lane, now)) return 'quiet';
  if (lane.status === 'syncing') return 'syncing';
  return 'healthy';
}

/** Where a lane's name leads: the matching Intake section. */
export function laneHref(lane: Pick<SourceArrivals, 'kind' | 'source_id'>): string {
  if (lane.kind === 'upload') return '/intake?section=upload';
  if (lane.kind === 'watch') return '/intake?section=watch';
  return `/intake?section=connections&source=${encodeURIComponent(lane.source_id ?? '')}`;
}
