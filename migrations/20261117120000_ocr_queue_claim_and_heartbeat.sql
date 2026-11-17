-- OCR job ownership and liveness.
--
-- claim_id: a fresh token written by every dequeue. A worker writes a job's
-- outcome (result, failed attempt, progress, completion) only while the row
-- still carries its claim_id and is 'processing'; a job that was cancelled,
-- or reset as stale and claimed again by another worker, no longer matches,
-- so the original worker writes nothing.
--
-- heartbeat_at: bumped when the worker starts processing a job and
-- periodically while it runs. Stale-job recovery keys on it rather than on
-- started_at (set at dequeue, before any wait for a worker slot), so a slow
-- but live OCR run is never reset under its worker; a job whose worker died
-- stops heartbeating and is still recovered.
ALTER TABLE ocr_queue ADD COLUMN IF NOT EXISTS claim_id UUID;
ALTER TABLE ocr_queue ADD COLUMN IF NOT EXISTS heartbeat_at TIMESTAMPTZ;

-- Jobs already running when this applies were claimed by a worker without a
-- claim token; their last sign of life is their start.
UPDATE ocr_queue SET heartbeat_at = started_at
WHERE status = 'processing' AND heartbeat_at IS NULL;

-- The guardrail helpers judged staleness by started_at too.
CREATE OR REPLACE FUNCTION find_stuck_ocr_jobs(stuck_threshold_minutes INTEGER DEFAULT 30)
RETURNS TABLE (
    document_id UUID,
    filename TEXT,
    worker_id TEXT,
    started_at TIMESTAMPTZ,
    minutes_stuck INTEGER
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        d.id,
        d.filename,
        q.worker_id,
        q.started_at,
        EXTRACT(EPOCH FROM (NOW() - COALESCE(q.heartbeat_at, q.started_at)))::INTEGER / 60 AS minutes_stuck
    FROM documents d
    JOIN ocr_queue q ON d.id = q.document_id
    WHERE d.ocr_status = 'processing'
      AND q.status = 'processing'
      AND COALESCE(q.heartbeat_at, q.started_at) < NOW() - make_interval(mins => stuck_threshold_minutes)
    ORDER BY COALESCE(q.heartbeat_at, q.started_at) ASC;
END;
$$ LANGUAGE plpgsql;

-- Resets only jobs whose worker stopped heartbeating, and only the documents
-- of jobs it put back in the queue. (It used to reset any 'processing'
-- document not updated for a while, live worker or not.)
CREATE OR REPLACE FUNCTION reset_stuck_ocr_jobs(stuck_threshold_minutes INTEGER DEFAULT 30)
RETURNS INTEGER AS $$
DECLARE
    reset_count INTEGER;
BEGIN
    WITH reset AS (
        UPDATE ocr_queue
        SET status = CASE WHEN attempts >= max_attempts THEN 'failed' ELSE 'pending' END,
            worker_id = NULL,
            started_at = NULL,
            claim_id = NULL,
            heartbeat_at = NULL,
            error_message = 'Reset due to timeout'
        WHERE status = 'processing'
          AND COALESCE(heartbeat_at, started_at) < NOW() - make_interval(mins => stuck_threshold_minutes)
        RETURNING document_id, status
    ), requeued_documents AS (
        UPDATE documents
        SET ocr_status = 'pending', updated_at = NOW()
        WHERE ocr_status = 'processing'
          AND id IN (SELECT document_id FROM reset WHERE status = 'pending')
        RETURNING id
    )
    SELECT COUNT(*) INTO reset_count FROM reset;

    RAISE NOTICE 'Reset % stuck OCR jobs', reset_count;
    RETURN reset_count;
END;
$$ LANGUAGE plpgsql;
