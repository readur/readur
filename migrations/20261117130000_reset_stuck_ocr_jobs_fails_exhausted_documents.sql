-- A stale OCR job with no attempts left is moved to 'failed', but
-- reset_stuck_ocr_jobs (20261117120000) only reset the documents of jobs it
-- put back in the queue: an exhausted job's document stayed
-- ocr_status = 'processing' forever, with no job left to finish it.
--
-- Such a document now fails the way an exhausted attempt fails elsewhere
-- (OcrQueueService::record_failed_attempt): ocr_status = 'failed' with an
-- ocr_failure_reason and ocr_error, plus a failed_documents record so it
-- shows up among the user's failed documents. Its worker stopped responding
-- for the whole stale window, so the reason is 'ocr_timeout'.
--
-- The Rust-side stale recovery (OcrQueueService::recover_stale_items) calls
-- this function, so the two cannot drift apart again.
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
        WHERE ocr_status = 'processing'
          AND id IN (SELECT document_id FROM reset WHERE status = 'failed')
        RETURNING id, user_id, filename, original_filename, file_path, file_size,
                  file_hash, mime_type, ocr_error
    ), failure_records AS (
        INSERT INTO failed_documents (
            user_id, filename, original_filename, file_path, file_size, file_hash,
            mime_type, failure_reason, failure_stage, ingestion_source,
            error_message, retry_count
        )
        SELECT d.user_id, d.filename, d.original_filename, d.file_path, d.file_size,
               d.file_hash, d.mime_type, 'ocr_timeout', 'ocr', 'ocr_queue',
               d.ocr_error, r.attempts
        FROM failed_documents_marked d
        JOIN reset r ON r.document_id = d.id
        RETURNING id
    )
    SELECT COUNT(*) INTO reset_count FROM reset;

    RAISE NOTICE 'Reset % stuck OCR jobs', reset_count;
    RETURN reset_count;
END;
$$ LANGUAGE plpgsql;
