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
