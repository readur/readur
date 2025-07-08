-- Fix the get_ocr_queue_stats function to ensure it matches the expected structure
-- This migration ensures the function correctly gets completed_today from documents table
-- and handles the case where migration 20250620100019 may have failed silently

CREATE OR REPLACE FUNCTION get_ocr_queue_stats()
RETURNS TABLE (
    pending_count BIGINT,
    processing_count BIGINT,
    failed_count BIGINT,
    completed_today BIGINT,
    avg_wait_time_minutes DOUBLE PRECISION,
    oldest_pending_minutes DOUBLE PRECISION
) AS $$
BEGIN
    RETURN QUERY
    WITH queue_stats AS (
        SELECT 
            COUNT(*) FILTER (WHERE status = 'pending') as pending_count,
            COUNT(*) FILTER (WHERE status = 'processing') as processing_count,
            COUNT(*) FILTER (WHERE status = 'failed' AND attempts >= max_attempts) as failed_count,
            CAST(AVG(EXTRACT(EPOCH FROM (COALESCE(started_at, NOW()) - created_at))/60) FILTER (WHERE status IN ('processing', 'completed')) AS DOUBLE PRECISION) as avg_wait_time_minutes,
            CAST(MAX(EXTRACT(EPOCH FROM (NOW() - created_at))/60) FILTER (WHERE status = 'pending') AS DOUBLE PRECISION) as oldest_pending_minutes
        FROM ocr_queue
    ),
    document_stats AS (
        -- Count documents that completed OCR today (looking at documents table where actual completion is tracked)
        SELECT COUNT(*) as completed_today
        FROM documents
        WHERE ocr_status = 'completed'
        AND updated_at >= CURRENT_DATE
        AND updated_at < CURRENT_DATE + INTERVAL '1 day'
    )
    SELECT 
        queue_stats.pending_count,
        queue_stats.processing_count,
        queue_stats.failed_count,
        document_stats.completed_today,
        queue_stats.avg_wait_time_minutes,
        queue_stats.oldest_pending_minutes
    FROM queue_stats, document_stats;
END;
$$ LANGUAGE plpgsql;