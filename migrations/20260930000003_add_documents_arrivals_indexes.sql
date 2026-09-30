-- Indexes for GET /api/sources/arrivals, so neither of its queries reads more
-- than the window's rows plus one index entry per lane.

-- Per-day counts for one user's documents inside the window
-- (user_id = $1 AND created_at >= $2).
CREATE INDEX IF NOT EXISTS idx_documents_user_created_at
    ON documents (user_id, created_at DESC);

-- Newest arrival per lane. These are keyed on the expression
-- `created_at AT TIME ZONE 'UTC'` rather than on created_at on purpose: the
-- last-arrival lookups ORDER BY that expression, which idx_documents_created_at_id
-- cannot provide. Otherwise the planner may walk that index newest-first,
-- filtering for the lane, which reads the whole table for a lane that has been
-- quiet for a long time. The expressions and WHERE clauses must stay identical
-- to the queries in src/db/arrivals.rs.

-- Per source.
CREATE INDEX IF NOT EXISTS idx_documents_source_last_arrival
    ON documents (source_id, (created_at AT TIME ZONE 'UTC') DESC)
    WHERE source_id IS NOT NULL;

-- Per user, watch folder lane.
CREATE INDEX IF NOT EXISTS idx_documents_watch_lane_last_arrival
    ON documents (user_id, (created_at AT TIME ZONE 'UTC') DESC)
    WHERE source_id IS NULL AND source_type = 'watch_folder';

-- Per user, uploads lane (every other document without a source).
CREATE INDEX IF NOT EXISTS idx_documents_upload_lane_last_arrival
    ON documents (user_id, (created_at AT TIME ZONE 'UTC') DESC)
    WHERE source_id IS NULL AND source_type IS DISTINCT FROM 'watch_folder';
