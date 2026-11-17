-- reset_stuck_ocr_jobs (20261117130000) failed the document of a stale job
-- with no attempts left only while that document was still 'processing'.
-- Dequeuing only moves a 'pending' document to 'processing', so a document
-- already 'failed' by an earlier attempt kept its old reason and got no
-- failed_documents record -- unlike OcrQueueService::record_failed_attempt,
-- which records every exhausted attempt. The function also raised a NOTICE
-- on every call, and OcrQueueService::recover_stale_items now calls it every
-- few minutes; the Rust caller logs when it recovers anything instead.
--
-- Now every job the function moves to 'failed' gets a failed_documents record
-- ('ocr_timeout'), and its document is marked failed with that reason --
-- unless the document is 'completed', whose text is kept, as cancelling does.
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
        RETURNING document_id, status, attempts
    ), requeued_documents AS (
        UPDATE documents
        SET ocr_status = 'pending', updated_at = NOW()
        WHERE ocr_status = 'processing'
          AND id IN (SELECT document_id FROM reset WHERE status = 'pending')
        RETURNING id
    ), failed_documents_marked AS (
        UPDATE documents
        SET ocr_status = 'failed',
            ocr_failure_reason = 'ocr_timeout',
            ocr_error = 'OCR worker stopped responding and the job has no attempts left',
            updated_at = NOW()
        WHERE ocr_status IS DISTINCT FROM 'completed'
          AND id IN (SELECT document_id FROM reset WHERE status = 'failed')
        RETURNING id
    ), failure_records AS (
        INSERT INTO failed_documents (
            user_id, filename, original_filename, file_path, file_size, file_hash,
            mime_type, failure_reason, failure_stage, ingestion_source,
            error_message, retry_count
        )
        SELECT d.user_id, d.filename, d.original_filename, d.file_path, d.file_size,
               d.file_hash, d.mime_type, 'ocr_timeout', 'ocr', 'ocr_queue',
               'OCR worker stopped responding and the job has no attempts left', r.attempts
        FROM reset r
        JOIN documents d ON d.id = r.document_id
        WHERE r.status = 'failed'
        RETURNING id
    )
    SELECT COUNT(*) INTO reset_count FROM reset;

    RETURN reset_count;
END;
$$ LANGUAGE plpgsql;

-- Documents the 20261117120000 function stranded: their only job failed, yet
-- they are still 'processing', with nothing left to finish them.
-- (Tested in tests/new_migration_data_tests.rs.)
DO $repair$
BEGIN
    WITH stranded AS (
        UPDATE documents d
        SET ocr_status = 'failed',
            ocr_failure_reason = 'ocr_timeout',
            ocr_error = 'OCR worker stopped responding and the job has no attempts left',
            updated_at = NOW()
        WHERE d.ocr_status = 'processing'
          AND EXISTS (SELECT 1 FROM ocr_queue q WHERE q.document_id = d.id AND q.status = 'failed')
          AND NOT EXISTS (SELECT 1 FROM ocr_queue q WHERE q.document_id = d.id AND q.status <> 'failed')
        RETURNING d.id, d.user_id, d.filename, d.original_filename, d.file_path, d.file_size,
                  d.file_hash, d.mime_type
    )
    INSERT INTO failed_documents (
        user_id, filename, original_filename, file_path, file_size, file_hash,
        mime_type, failure_reason, failure_stage, ingestion_source,
        error_message, retry_count
    )
    SELECT s.user_id, s.filename, s.original_filename, s.file_path, s.file_size,
           s.file_hash, s.mime_type, 'ocr_timeout', 'ocr', 'ocr_queue',
           'OCR worker stopped responding and the job has no attempts left',
           (SELECT MAX(q.attempts) FROM ocr_queue q WHERE q.document_id = s.id)
    FROM stranded s
    WHERE NOT EXISTS (
        SELECT 1 FROM failed_documents f
        WHERE f.user_id = s.user_id AND f.file_path = s.file_path
          AND f.failure_reason = 'ocr_timeout'
    );
END;
$repair$;
