/**
 * How a counted server action went: every item done, some done, or none. The server reports
 * counts (deleted_count, queued_count) and a 200 with a zero count is still a failure.
 */
export type Outcome = 'all' | 'some' | 'none';

export function outcomeOf(done: number | null | undefined, requested: number): Outcome {
  const n = typeof done === 'number' && Number.isFinite(done) ? done : 0;
  if (n <= 0) return 'none';
  return n >= requested ? 'all' : 'some';
}

export const toneOf = (o: Outcome): 'success' | 'danger' => (o === 'all' ? 'success' : 'danger');

/** What a bulk delete reported: how it went, how many were deleted, and which ids are gone. */
export interface BulkDeleteResult {
  outcome: Outcome;
  deleted: number;
  /** Ids the server confirms as deleted; the rest still exist. */
  gone: Set<string>;
}

/**
 * Reads a `/documents/bulk/delete` response. Only ids the server lists as deleted count as gone;
 * without a list, all of them are gone only when the count covers every requested id.
 */
export function bulkDeleteResult(body: unknown, requested: readonly string[]): BulkDeleteResult {
  const b = (body ?? {}) as { deleted_count?: number; deleted_documents?: string[] };
  const outcome = outcomeOf(b.deleted_count, requested.length);
  const deleted = typeof b.deleted_count === 'number' && Number.isFinite(b.deleted_count) ? b.deleted_count : 0;
  const listed = Array.isArray(b.deleted_documents) ? b.deleted_documents : outcome === 'all' ? requested : [];
  const asked = new Set(requested);
  return { outcome, deleted, gone: new Set(listed.filter((id) => asked.has(id))) };
}
