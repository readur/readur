-- Fix type mismatch in get_ocr_queue_stats function
-- The AVG() and MAX() functions return NUMERIC but we need DOUBLE PRECISION

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
    SELECT 
        COUNT(*) FILTER (WHERE status = 'pending') as pending_count,
        COUNT(*) FILTER (WHERE status = 'processing') as processing_count,
        COUNT(*) FILTER (WHERE status = 'failed' AND attempts >= max_attempts) as failed_count,
        COUNT(*) FILTER (WHERE status = 'completed' AND completed_at >= CURRENT_DATE) as completed_today,
        CAST(AVG(EXTRACT(EPOCH FROM (COALESCE(started_at, NOW()) - created_at))/60) FILTER (WHERE status IN ('processing', 'completed')) AS DOUBLE PRECISION) as avg_wait_time_minutes,
        CAST(MAX(EXTRACT(EPOCH FROM (NOW() - created_at))/60) FILTER (WHERE status = 'pending') AS DOUBLE PRECISION) as oldest_pending_minutes
    FROM ocr_queue;
END;
$$ LANGUAGE plpgsql;